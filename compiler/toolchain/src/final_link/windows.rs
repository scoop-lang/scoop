//! PE linking through the selected SDK/CRT's compiler driver.
use super::*;
use crate::ValidatedCxxToolchain;

mod command;
#[cfg(all(test, windows, target_arch = "x86_64"))]
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
    #[serde(with = "crate::build_record::values::path")]
    unwind_prefix: PathBuf,
    cxx: Option<ValidatedCxxToolchain>,
}

impl WindowsFinalLinkProfile {
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
        let unwind_prefix = std::path::absolute(crate::selected_unwind_prefix(
            startup.target(),
            options.sysroot.as_deref(),
            options.unwind_prefix.as_deref(),
        )?)
        .map_err(error)?;
        let archive = unwind_prefix.join("lib/libunwind.a");
        if !archive.is_file() {
            return Err(error(format!(
                "missing Windows LLVM unwind resource {}",
                archive.display()
            )));
        }
        Ok(Self {
            data: Discovery {
                output,
                native_roots: options.native_roots(startup.target())?,
                driver,
                linker,
                linker_kind,
                unwind_prefix,
                cxx,
            },
            startup,
        })
    }

    pub fn startup_toolchain(&self) -> &ValidatedCBridgeToolchainInvocation {
        &self.startup
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
    pub fn fingerprint(&self) -> Result<Digest256, ToolchainError> {
        domain_separated_cbor_hash("scoop-windows-final-link-profile-v2", self).map_err(error)
    }
}
