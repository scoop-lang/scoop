use super::elf::GnuElfLinkPlan;
use super::*;
use crate::linux_target::{LinuxLibc, LinuxTarget};
use scoop_process::CommandExt;
use scoop_wire::sha256;

mod probe;
mod runtime;
use runtime::RuntimeLibraries;
mod wire;

const LINUX_DRIVER_OPTIONS: &[&str] = &["-pthread"];

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
struct SystemInput {
    #[serde(with = "crate::build_record::values::path")]
    path: PathBuf,
    #[serde(with = "crate::build_record::values::digest")]
    digest: Digest256,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LinuxFinalLinkProfile {
    startup: ValidatedCBridgeToolchainInvocation,
    target: LinuxTarget,
    pub(super) data: Discovery,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub(super) struct Discovery {
    pub(super) output: OutputConfiguration,
    #[serde(with = "crate::build_record::values::paths")]
    pub(super) native_roots: Vec<PathBuf>,
    mode: LinkMode,
    #[serde(with = "crate::build_record::values::path")]
    linker: PathBuf,
    linker_version: String,
    #[serde(with = "crate::build_record::values::digest")]
    compiler_digest: Digest256,
    runtime: RuntimeLibraries,
    inputs: Vec<SystemInput>,
}

impl LinuxFinalLinkProfile {
    pub(super) fn resolve(
        startup: ValidatedCBridgeToolchainInvocation,
        options: &FinalLinkOptions,
        cxx: Option<crate::ValidatedCxxToolchain>,
        native_roots: Vec<PathBuf>,
    ) -> Result<Self, ToolchainError> {
        let target = startup.profile().contract().target().id();
        let output = OutputConfiguration::resolve(startup.target(), options)?;
        let selected = LinuxTarget::select(target)?;
        let mode = options
            .mode
            .unwrap_or(match (output.kind(), selected.libc) {
                (FinalOutputKind::SharedLibrary, _) => LinkMode::Dynamic,
                (_, LinuxLibc::Musl) => LinkMode::Static,
                (_, LinuxLibc::Gnu) => LinkMode::Dynamic,
            });
        if output.kind() == FinalOutputKind::SharedLibrary && mode == LinkMode::Static {
            return Err(error(
                "Linux shared library output requires dynamic system libraries",
            ));
        }
        if selected.libc == LinuxLibc::Gnu && mode == LinkMode::Static {
            return Err(error(
                "M28 supports glibc dynamic PIE; glibc static linking is outside its target matrix",
            ));
        }
        let runtime = RuntimeLibraries::resolve(startup.target(), options, cxx)?;
        let linker = probe::driver_program(runtime.command(&startup), "ld")?;
        let version = probe::run(Command::new(&linker).arg("--version"))?;
        let linker_version = String::from_utf8(version.stdout).map_err(error)?;
        if !linker_version.starts_with("GNU ld ") {
            return Err(error("this Linux driver profile requires GNU ld"));
        }
        let compiler_digest = crate::build_record::fingerprint(&crate::build_record::Input::read(
            runtime.driver(&startup),
        )?)?;
        let mut profile = Self {
            startup,
            target: selected,
            data: Discovery {
                output,
                native_roots,
                mode,
                linker,
                linker_version,
                compiler_digest,
                runtime,
                inputs: Vec::new(),
            },
        };
        profile.data.inputs = probe::check(&profile)?;
        Ok(profile)
    }

    pub fn mode(&self) -> LinkMode {
        self.data.mode
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
    pub fn linker_args(&self) -> Vec<&'static str> {
        LINUX_DRIVER_OPTIONS
            .iter()
            .copied()
            .chain(self.elf_plan().linker_args())
            .collect()
    }
    pub fn unwind_prefix(&self) -> Option<&Path> {
        self.data.runtime.unwind_prefix()
    }
    pub fn cxx(&self) -> bool {
        self.data.runtime.cxx()
    }
    pub fn input_paths(&self) -> impl Iterator<Item = &Path> {
        self.data.inputs.iter().map(|input| input.path.as_path())
    }

    pub fn command(&self, scratch: &Path, output: &Path) -> Result<Command, ToolchainError> {
        let mut command = self.data.runtime.command(&self.startup);
        if !self.cxx() {
            command.arg("-nodefaultlibs");
        }
        command.env("TMPDIR", scratch).args(LINUX_DRIVER_OPTIONS);
        self.elf_plan()
            .configure_command(&mut command, scratch, output)?;
        Ok(command)
    }

    /// Append after the ordinary objects, so archive extraction sees their uses.
    pub fn append_system_libraries(&self, command: &mut Command) {
        if let Some(prefix) = self.unwind_prefix() {
            command
                .arg("-Wl,--start-group")
                .arg(prefix.join("lib/libunwind.a"))
                .args(["-lc", "-lm", "-lpthread", "-lgcc", "-Wl,--end-group"]);
        }
    }

    fn elf_plan(&self) -> GnuElfLinkPlan<'_> {
        GnuElfLinkPlan::new(&self.data.output, self.data.mode)
    }
}

#[cfg(all(test, target_os = "linux", target_arch = "x86_64"))]
mod tests;
