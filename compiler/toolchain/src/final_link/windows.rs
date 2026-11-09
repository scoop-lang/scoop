//! PE linking through the selected SDK/CRT's compiler driver.
use super::*;
use crate::ValidatedCxxToolchain;
use crate::build_record::Input;
use scoop_process::CommandExt;
use std::collections::BTreeSet;

mod command;
mod probe;
mod trace;
mod wire;

#[cfg(all(test, windows, target_arch = "x86_64"))]
mod shared_tests;
#[cfg(all(test, windows, target_arch = "x86_64"))]
mod tests;

const MSVC_OPTIONS: &[&str] = &[
    "/dynamicbase",
    "/highentropyva",
    "/nxcompat",
    "/stack:8388608,4096",
    "/opt:ref,noicf",
    "/incremental:no",
    "/Brepro",
    "/ignore:4217",
];
const MINGW_OPTIONS: &[&str] = &[
    "--gc-sections",
    "--icf=none",
    "--dynamicbase",
    "--high-entropy-va",
    "--nxcompat",
    "--stack=8388608,4096",
    "--no-insert-timestamp",
    "--Xlink=/ignore:4217",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
enum LinkerKind {
    LldCoff,
    Microsoft,
    LldMinGw,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowsFinalLinkProfile {
    startup: ValidatedCBridgeToolchainInvocation,
    pub(super) data: Discovery,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub(super) struct Discovery {
    pub(super) output: OutputConfiguration,
    #[serde(with = "crate::build_record::values::paths")]
    pub(super) native_roots: Vec<PathBuf>,
    #[serde(with = "crate::build_record::values::path")]
    driver: PathBuf,
    #[serde(with = "crate::build_record::values::path")]
    linker: PathBuf,
    linker_kind: LinkerKind,
    linker_version: String,
    #[serde(with = "crate::build_record::values::digest")]
    compiler_digest: Digest256,
    #[serde(with = "crate::build_record::values::digest")]
    linker_digest: Digest256,
    #[serde(with = "crate::build_record::values::path")]
    unwind_prefix: PathBuf,
    #[serde(with = "crate::build_record::values::path")]
    unwind_include: PathBuf,
    cxx: Option<ValidatedCxxToolchain>,
    #[serde(with = "crate::build_record::values::paths")]
    observed_paths: Vec<PathBuf>,
    inputs: Vec<SystemInput>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
struct SystemInput {
    #[serde(with = "crate::build_record::values::path")]
    path: PathBuf,
    #[serde(with = "crate::build_record::values::digest")]
    digest: Digest256,
}

impl WindowsFinalLinkProfile {
    pub(super) fn into_discovery(self) -> Discovery {
        self.data
    }

    pub(super) fn from_discovery(
        startup: ValidatedCBridgeToolchainInvocation,
        data: Discovery,
    ) -> Self {
        Self { startup, data }
    }

    pub fn resolve(
        startup: ValidatedCBridgeToolchainInvocation,
        options: &FinalLinkOptions,
        cxx: Option<ValidatedCxxToolchain>,
    ) -> Result<Self, ToolchainError> {
        let output = OutputConfiguration::resolve(startup.target(), options)?;
        let environment = startup.windows_environment().map_err(error)?;
        if options.mode == Some(LinkMode::Static) {
            return Err(error(
                "Windows target resources use their selected dynamic CRT; --link-mode static is not supported",
            ));
        }
        let microsoft =
            startup.native_environment() == scoop_target_data::NativeEnvironment::MicrosoftUcrt;
        let mut observed = BTreeSet::new();
        let driver = if microsoft || environment.bundled_mingw.is_some() {
            startup
                .compiler_driver()
                .with_file_name(crate::host_executable_name(if microsoft {
                    "clang-cl"
                } else if cxx.is_some() {
                    "clang++"
                } else {
                    "clang"
                }))
        } else {
            let prefix = environment.native_sysroot.as_ref().expect("MinGW prefix");
            prefix
                .join("bin")
                .join(crate::host_executable_name(if cxx.is_some() {
                    "clang++"
                } else {
                    "clang"
                }))
        };
        let driver = crate::paths::find_program(driver.as_os_str(), &environment.search_path)?;
        let default_linker = if microsoft {
            startup
                .compiler_driver()
                .with_file_name(crate::host_executable_name("lld-link"))
        } else {
            driver.with_file_name(crate::host_executable_name("ld.lld"))
        };
        let selected = options.linker.as_deref().unwrap_or(&default_linker);
        crate::native_record::program_candidates(
            selected.as_os_str(),
            &environment.search_path,
            &mut observed,
        );
        let linker = crate::paths::find_program(selected.as_os_str(), &environment.search_path)?;
        let linker_kind = if !microsoft {
            LinkerKind::LldMinGw
        } else if linker
            .file_stem()
            .is_some_and(|name| name.eq_ignore_ascii_case("link"))
        {
            LinkerKind::Microsoft
        } else {
            LinkerKind::LldCoff
        };
        let mut version_command = environment.bare_command(&linker);
        version_command
            .env("VSLANG", "1033")
            .arg(if linker_kind == LinkerKind::Microsoft {
                "/?"
            } else {
                "--version"
            });
        // LINK returns 1100 for its normal help output. The banner selects the
        // command dialect; the real link below verifies executable capability.
        let version = version_command.scoop_output().map_err(error)?;
        let linker_version = String::from_utf8_lossy(&version.stdout)
            .lines()
            .next()
            .unwrap_or_default()
            .to_owned();
        if !match linker_kind {
            LinkerKind::Microsoft => {
                linker_version.contains("Microsoft") && linker_version.contains("Linker")
            }
            _ => linker_version.contains("LLD"),
        } {
            return Err(error(format!(
                "incompatible Windows linker: {linker_version}"
            )));
        }
        let unwind_prefix = std::path::absolute(crate::selected_unwind_prefix(
            startup.target(),
            options.sysroot.as_deref(),
            options.unwind_prefix.as_deref(),
        )?)
        .map_err(error)?;
        let unwind_include =
            crate::unwind::link_unwind_include(&unwind_prefix, options.unwind_prefix.is_some())?;
        let archive = unwind_prefix.join("lib/libunwind.a");
        if !archive.is_file() {
            return Err(error(format!(
                "missing Windows LLVM unwind resource {}",
                archive.display()
            )));
        }
        observed.extend(environment.libraries.iter().cloned());
        observed.extend([driver.clone(), linker.clone()]);
        if let Some(cxx) = &cxx {
            observed.extend(cxx.input_paths()?);
        }
        let compiler_digest = crate::build_record::fingerprint(&Input::read(&driver)?)?;
        let linker_digest = crate::build_record::fingerprint(&Input::read(&linker)?)?;
        let mut profile = Self {
            data: Discovery {
                output,
                native_roots: options.native_roots(startup.target())?,
                driver,
                linker,
                linker_kind,
                linker_version,
                compiler_digest,
                linker_digest,
                unwind_prefix,
                unwind_include,
                cxx,
                observed_paths: Vec::new(),
                inputs: Vec::new(),
            },
            startup,
        };
        profile.data.inputs = probe::check(&profile, &mut observed)?;
        profile.data.observed_paths = observed.into_iter().collect();
        Ok(profile)
    }

    pub fn startup_toolchain(&self) -> &ValidatedCBridgeToolchainInvocation {
        &self.startup
    }
    pub fn compiler_digest(&self) -> Digest256 {
        self.data.compiler_digest
    }
    pub fn linker_driver(&self) -> &Path {
        &self.data.linker
    }
    pub fn cxx(&self) -> bool {
        self.data.cxx.is_some()
    }
    pub fn microsoft(&self) -> bool {
        self.data.linker_kind != LinkerKind::LldMinGw
    }
    pub fn unwind_prefix(&self) -> &Path {
        &self.data.unwind_prefix
    }
    pub fn library_search_paths(&self) -> &[PathBuf] {
        &self
            .startup
            .windows_environment()
            .expect("Windows environment")
            .libraries
    }
    pub fn linker_args(&self) -> Vec<&'static str> {
        if self.microsoft() {
            let environment = self
                .startup
                .windows_environment()
                .expect("Windows environment");
            let mut arguments = vec![environment.machine.msvc_linker_machine_arg()];
            arguments.extend(MSVC_OPTIONS);
            arguments
        } else {
            MINGW_OPTIONS.to_vec()
        }
    }
    pub fn input_paths(&self) -> Vec<PathBuf> {
        self.data
            .observed_paths
            .iter()
            .cloned()
            .chain(self.data.inputs.iter().map(|input| input.path.clone()))
            .collect()
    }
    pub fn system_input_paths(&self) -> impl Iterator<Item = &Path> {
        self.data.inputs.iter().map(|input| input.path.as_path())
    }
    pub fn fingerprint(&self) -> Result<Digest256, ToolchainError> {
        domain_separated_cbor_hash("scoop-windows-final-link-profile-v1", self).map_err(error)
    }
}
