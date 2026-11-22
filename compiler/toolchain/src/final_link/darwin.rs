//! Explicit Darwin final-link toolchain; independent of the Scoop backend.
use super::{FinalLinkOptions, FinalOutputKind, OutputConfiguration, output::NATIVE_APP_ENTRY};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::ValidatedCBridgeToolchainInvocation;
use scoop_lir::{LirTargetProfile, TargetProfileId};
use scoop_wire::{Digest256, Encoder, WireEncode, domain_separated_cbor_hash};

use crate::{SystemProvider, ToolchainError};

#[cfg(all(test, target_os = "macos"))]
mod tests;
pub(super) mod tools;

const OPTIONS: &[&str] = &[
    "-no_deduplicate",
    "-no_fixup_chains",
    "-no_implicit_dylibs",
    "-adhoc_codesign",
];

const PRODUCTION_SYMBOLS: &str = "_scoop$1$bs$*\n";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DarwinFinalLinkProfile {
    startup: ValidatedCBridgeToolchainInvocation,
    swift_toolchain: Option<PathBuf>,
    pub(super) data: Discovery,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub(super) struct Discovery {
    pub(super) output: OutputConfiguration,
    #[serde(with = "crate::build_record::values::paths")]
    pub(super) native_roots: Vec<PathBuf>,
    #[serde(with = "crate::build_record::values::path")]
    linker: PathBuf,
    system: SystemProvider,
    cxx: Option<crate::ValidatedCxxToolchain>,
}

impl DarwinFinalLinkProfile {
    pub(crate) fn from_startup(
        startup: ValidatedCBridgeToolchainInvocation,
        options: &FinalLinkOptions,
        cxx: Option<crate::ValidatedCxxToolchain>,
        native_roots: Vec<PathBuf>,
    ) -> Result<Self, ToolchainError> {
        let output = OutputConfiguration::resolve(startup.target(), options)?;
        let linker = tools::resolve(tools::candidates(options.linker.as_deref())?)?;
        let system = SystemProvider::read(
            startup.target().architecture(),
            startup.apple_platform().ok_or_else(|| {
                error(
                    scoop_source::DiagnosticCode::ICE2201,
                    "Mach-O final link requires an Apple deployment platform",
                )
            })?,
            startup
                .sdk_root()
                .map_err(|cause| error(scoop_source::DiagnosticCode::ICE2201, cause))?,
            startup
                .profile()
                .contract()
                .deployment()
                .map_err(|cause| error(scoop_source::DiagnosticCode::ICE2201, cause))?
                .minimum_os(),
        )?;
        Ok(Self {
            startup,
            swift_toolchain: options.swift_toolchain.clone(),
            data: Discovery {
                output,
                native_roots,
                linker,
                system,
                cxx,
            },
        })
    }

    pub const fn id(&self) -> TargetProfileId {
        self.startup.target().id()
    }
    pub const fn target(&self) -> LirTargetProfile {
        self.startup.target()
    }
    pub const fn canonical_triple(&self) -> &'static str {
        self.startup.target().contract().canonical_triple()
    }
    pub fn linker_driver(&self) -> &Path {
        &self.data.linker
    }
    pub fn linker_args(&self) -> Vec<&'static str> {
        // LLD 22.1 can move the destination write of AdrpLdrGot ahead of an
        // intervening use of the old register value.
        let architecture = self.target().architecture();
        let hint_option = (architecture == scoop_lir::TargetArchitecture::Aarch64)
            .then_some("-ignore_optimization_hints");
        let output_options: &[&str] = match self.data.output.kind() {
            FinalOutputKind::Executable => &[
                "-execute",
                "-pie",
                "-e",
                "_main",
                "-unexported_symbol",
                "_scoop$1$*",
            ],
            FinalOutputKind::SharedLibrary => &["-dylib"],
        };
        [
            "-flavor",
            "darwin",
            "-arch",
            scoop_target_data::apple_architecture(architecture),
        ]
        .into_iter()
        .chain(hint_option)
        .chain(output_options.iter().copied())
        .chain(OPTIONS.iter().copied())
        .collect()
    }
    pub fn startup_toolchain(&self) -> &ValidatedCBridgeToolchainInvocation {
        &self.startup
    }
    pub fn platform(&self) -> scoop_target_data::ApplePlatform {
        self.startup
            .apple_platform()
            .expect("Apple startup toolchain")
    }
    pub fn system_provider(&self) -> &SystemProvider {
        &self.data.system
    }

    pub fn cxx(&self) -> bool {
        self.data.cxx.is_some()
    }

    /// Only queried after an actually required Swift library was absent in the SDK.
    pub fn swift_library_roots(&self) -> Result<Vec<PathBuf>, ToolchainError> {
        let root = crate::swift::toolchain_root(self.swift_toolchain.as_deref())?;
        let platform = self.platform().sdk();
        Ok(vec![
            root.join("usr/lib/swift").join(platform),
            root.join("usr/lib/swift_static").join(platform),
        ])
    }

    pub fn linker_argument(&self, command: &mut Command, argument: impl AsRef<std::ffi::OsStr>) {
        command.arg(argument);
    }

    pub fn linker_system_requirements(&self) -> &'static [&'static str] {
        &["dyld_stub_binder"]
    }

    pub fn fingerprint(&self) -> Result<Digest256, ToolchainError> {
        domain_separated_cbor_hash("scoop-final-link-profile-v3", self)
            .map_err(|cause| error(scoop_source::DiagnosticCode::ICE2201, cause))
    }

    /// Callers append the ordered objects, aliases and selected system stubs.
    pub fn command(
        &self,
        sdk_root: &Path,
        scratch: &Path,
        output: &Path,
    ) -> Result<Command, ToolchainError> {
        let deployment = self
            .startup
            .profile()
            .contract()
            .deployment()
            .map_err(|cause| error(scoop_source::DiagnosticCode::ICE2201, cause))?;
        // The link plan supplies the selected SDK and C++ stubs explicitly.
        let mut command = Command::new(&self.data.linker);
        command
            .env_clear()
            .env("LC_ALL", "C")
            .env("LANG", "C")
            .env("TZ", "UTC");
        for argument in self.linker_args() {
            self.linker_argument(&mut command, argument);
        }
        // Unwind boundary aliases refer to input records consumed by LLD. Resolve
        // their relocations, but do not emit these production-only aliases into
        // the final symbol table. Real callable and data symbols remain available.
        std::fs::create_dir_all(scratch)
            .map_err(|cause| error(scoop_source::DiagnosticCode::E2201, cause))?;
        let production_symbols = scratch.join("production-symbols.txt");
        std::fs::write(&production_symbols, PRODUCTION_SYMBOLS)
            .map_err(|cause| error(scoop_source::DiagnosticCode::E2201, cause))?;
        command
            .arg("-non_global_symbols_strip_list")
            .arg(production_symbols);
        if self.data.output.kind() == FinalOutputKind::SharedLibrary {
            let exports = scratch.join("native-app.exports");
            std::fs::write(&exports, format!("_{NATIVE_APP_ENTRY}\n"))
                .map_err(|cause| error(scoop_source::DiagnosticCode::E2201, cause))?;
            self.linker_argument(&mut command, "-exported_symbols_list");
            self.linker_argument(&mut command, exports);
            self.linker_argument(&mut command, "-install_name");
            self.linker_argument(
                &mut command,
                self.data.output.loader_name(output, "@rpath/")?,
            );
        }
        for argument in [
            "-platform_version".into(),
            self.platform().linker_name().into(),
            deployment.minimum_os().to_string().into(),
            deployment.sdk().to_string().into(),
            "-syslibroot".into(),
            sdk_root.as_os_str().to_owned(),
            "-o".into(),
            output.as_os_str().to_owned(),
        ] {
            self.linker_argument(&mut command, argument);
        }
        Ok(command)
    }
}

impl WireEncode for DarwinFinalLinkProfile {
    fn encode(&self, e: &mut Encoder) -> Result<(), scoop_wire::cbor::EncodeError> {
        e.map(9)?;
        e.field(1)?;
        self.target().wire_id().encode(e)?;
        e.field(2)?;
        self.startup.profile().contract().encode(e)?;
        e.field(3)?;
        let arguments = self.linker_args();
        e.array(arguments.len() as u64)?;
        for option in arguments {
            e.text(option)?;
        }
        e.field(4)?;
        e.array(self.data.system.files().len() as u64)?;
        for file in self.data.system.files() {
            e.map(2)?;
            e.field(1)?;
            e.text(&file.relative_path().to_string_lossy())?;
            e.field(2)?;
            file.digest().encode(e)?;
        }
        e.field(5)?;
        e.array(self.linker_system_requirements().len() as u64)?;
        for symbol in self.linker_system_requirements() {
            e.text(symbol)?;
        }
        e.field(6)?;
        e.unsigned(1)?;
        e.field(7)?;
        e.array(u64::from(self.data.cxx.is_some()))?;
        if let Some(cxx) = &self.data.cxx {
            cxx.fingerprint().encode(e)?;
        }
        e.field(8)?;
        self.data.output.encode(e)?;
        e.field(9)?;
        e.text(PRODUCTION_SYMBOLS)
    }
}

fn error(code: scoop_source::DiagnosticCode, value: impl std::fmt::Display) -> ToolchainError {
    ToolchainError::new(code, format!("final-link toolchain: {value}"))
}
