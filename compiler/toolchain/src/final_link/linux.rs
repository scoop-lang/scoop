use super::elf::{GNU_ELF_LINKER_SYMBOLS, GnuElfLinkPlan};
use super::*;
use crate::linux_target::{LinuxLibc, LinuxTarget};
use scoop_process::CommandExt;
use scoop_wire::sha256;

mod probe;
mod runtime;
use runtime::RuntimeLibraries;
mod wire;

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
    runtime: RuntimeLibraries,
    #[serde(with = "crate::build_record::values::paths")]
    inputs: Vec<PathBuf>,
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
                scoop_source::DiagnosticCode::E2301,
                "Linux shared library output requires dynamic system libraries",
            ));
        }
        if selected.libc == LinuxLibc::Gnu && mode == LinkMode::Static {
            return Err(error(
                scoop_source::DiagnosticCode::E2301,
                "M28 supports glibc dynamic PIE; glibc static linking is outside its target matrix",
            ));
        }
        let runtime = RuntimeLibraries::resolve(startup.target(), options, cxx)?;
        let linker = probe::linker(&startup, &runtime)?;
        let mut profile = Self {
            startup,
            target: selected,
            data: Discovery {
                output,
                native_roots,
                mode,
                linker,
                runtime,
                inputs: Vec::new(),
            },
        };
        profile.data.inputs = probe::system_inputs(&profile)?;
        Ok(profile)
    }

    pub fn mode(&self) -> LinkMode {
        self.data.mode
    }
    pub fn startup_toolchain(&self) -> &ValidatedCBridgeToolchainInvocation {
        &self.startup
    }
    pub fn linker_driver(&self) -> &Path {
        &self.data.linker
    }
    pub fn elf_inputs(&self) -> ElfLinkInputs<'_> {
        ElfLinkInputs {
            mode: self.mode(),
            system_paths: &self.data.inputs,
            compatibility_providers: match self.target.libc {
                LinuxLibc::Gnu => &[],
                LinuxLibc::Musl => &["libc.a", "libc.so", "libc.so.6"],
            },
            conflicting_unwind_prefixes: &["libgcc_s.so"],
            linker_defined_symbols: GNU_ELF_LINKER_SYMBOLS,
        }
    }
    fn uses_paired_lld(&self) -> bool {
        self.startup.target().architecture() == scoop_lir::TargetArchitecture::Aarch64
    }

    fn driver_options(&self) -> &'static [&'static str] {
        match (self.uses_paired_lld(), self.mode()) {
            (false, _) => &["-pthread"],
            (true, LinkMode::Dynamic) => &["-pthread", "-fuse-ld=lld"],
            // musl-gcc supplies an interpreter even when invoking the static linker.
            (true, LinkMode::Static) => &["-pthread", "-fuse-ld=lld", "-Wl,--no-dynamic-linker"],
        }
    }

    fn driver_command(&self) -> Command {
        let mut command = self.data.runtime.command(&self.startup);
        if self.uses_paired_lld() {
            // Both GCC and Clang search this directory first for ld.lld.
            command
                .arg("-B")
                .arg(self.data.linker.parent().expect("absolute linker path"));
        }
        command
    }

    pub fn linker_args(&self) -> Vec<&'static str> {
        self.driver_options()
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

    pub fn command(&self, scratch: &Path, output: &Path) -> Result<Command, ToolchainError> {
        let mut command = self.driver_command();
        if !self.cxx() {
            command.arg("-nodefaultlibs");
        }
        command.env("TMPDIR", scratch).args(self.driver_options());
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
                .args(["-lc", "-lm", "-lpthread"]);
            if self.target.libc == LinuxLibc::Musl
                && self.startup.target().architecture() == scoop_lir::TargetArchitecture::Aarch64
            {
                command.arg(prefix.join("lib/libclang_rt.builtins-aarch64.a"));
            } else {
                command.arg("-lgcc");
            }
            command.arg("-Wl,--end-group");
        }
    }

    fn elf_plan(&self) -> GnuElfLinkPlan<'_> {
        let dynamic_script = if self.uses_paired_lld() {
            include_str!("../elf/lld-dynamic.ld")
        } else {
            include_str!("../elf/dynamic.ld")
        };
        GnuElfLinkPlan::new(&self.data.output, self.data.mode, dynamic_script)
    }
}

#[cfg(all(test, target_os = "linux", target_arch = "x86_64"))]
mod tests;
