use super::*;
use scoop_lir::TargetArchitecture;
use scoop_target_data::{ApplePlatform, apple_architecture};
use serde::Deserialize;
mod directives;
mod interface;
pub use directives::PreviousExport;
pub(super) use directives::StubDirectives;
pub use interface::{TextStubInterface, read_text_stubs, write_link_stub};

fn target_name(architecture: TargetArchitecture, platform: ApplePlatform) -> String {
    format!(
        "{}-{}",
        apple_architecture(architecture),
        platform.linker_name()
    )
}

#[derive(Clone, Copy)]
pub(super) enum StubSource {
    Explicit,
    Sdk,
    SdkReexport,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub(super) struct Record {
    tbd_version: u32,
    targets: Vec<String>,
    #[serde(default)]
    flags: Vec<String>,
    pub install_name: String,
    #[serde(default = "default_version", deserialize_with = "version")]
    pub current_version: u32,
    #[serde(default = "default_version", deserialize_with = "version")]
    pub compatibility_version: u32,
    #[serde(default)]
    exports: Vec<Exports>,
    #[serde(default)]
    reexports: Vec<Exports>,
    #[serde(default)]
    reexported_libraries: Vec<Libraries>,
}

fn default_version() -> u32 {
    1 << 16
}

fn version<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<u32, D::Error> {
    use serde::de::Error;
    let value = serde_yaml_ng::Value::deserialize(deserializer)?;
    let text = match value {
        serde_yaml_ng::Value::String(value) => value,
        serde_yaml_ng::Value::Number(value) => value.to_string(),
        _ => return Err(D::Error::custom("SDK dylib version is not a number/string")),
    };
    if text == "0" {
        return Ok(0);
    }
    let text = if text.contains('.') {
        text
    } else {
        format!("{text}.0")
    };
    crate::c_bridge::parse_darwin_version(&text, "SDK dylib version")
        .map(|version| version.packed())
        .map_err(D::Error::custom)
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
struct Exports {
    targets: Vec<String>,
    #[serde(default)]
    symbols: Vec<String>,
    #[serde(default)]
    weak_symbols: Vec<String>,
    #[serde(default)]
    thread_local_symbols: Vec<String>,
    #[serde(default)]
    objc_classes: Vec<String>,
    #[serde(default)]
    objc_eh_types: Vec<String>,
    #[serde(default)]
    objc_ivars: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
struct Libraries {
    targets: Vec<String>,
    libraries: Vec<String>,
}

pub(super) fn parse(bytes: &[u8]) -> Result<Vec<Record>, ToolchainError> {
    serde_yaml_ng::Deserializer::from_slice(bytes)
        .map(|document| {
            let record = Record::deserialize(document).map_err(|error| {
                ToolchainError::new(
                    scoop_source::DiagnosticCode::E2303,
                    format!("invalid SDK text stub: {error}"),
                )
            })?;
            if record.tbd_version != 4 {
                return Err(ToolchainError::new(
                    scoop_source::DiagnosticCode::E2303,
                    format!("SDK text stub version {} is not v4", record.tbd_version),
                ));
            }
            Ok(record)
        })
        .collect()
}

impl Record {
    fn target(
        &self,
        architecture: TargetArchitecture,
        platform: ApplePlatform,
        source: StubSource,
    ) -> Option<&str> {
        let ordinary = target_name(architecture, platform);
        let arm64e = (!matches!(source, StubSource::Explicit)
            && architecture == TargetArchitecture::Aarch64)
            .then(|| format!("arm64e-{}", platform.linker_name()));
        for target in [Some(ordinary.as_str()), arm64e.as_deref()]
            .into_iter()
            .flatten()
        {
            if let Some(found) = self.targets.iter().find(|candidate| *candidate == target) {
                return Some(found);
            }
        }
        None
    }

    pub(super) fn collect(
        &self,
        architecture: TargetArchitecture,
        platform: ApplePlatform,
        deployment: DarwinPackedVersionV1,
        source: StubSource,
        exports: &mut BTreeMap<String, NativeExport>,
        pending: &mut Vec<String>,
    ) -> Result<StubDirectives, ToolchainError> {
        let selected = self.target(architecture, platform, source);
        let target = selected
            .or_else(|| {
                if matches!(source, StubSource::SdkReexport)
                    && platform == ApplePlatform::IosSimulator
                {
                    self.target(architecture, ApplePlatform::MacOs, source)
                } else {
                    None
                }
            })
            .ok_or_else(|| {
                ToolchainError::new(
                    scoop_source::DiagnosticCode::E2303,
                    format!(
                        "SDK {} has no compatible {} target",
                        self.install_name,
                        target_name(architecture, platform),
                    ),
                )
            })?;
        for library in &self.reexported_libraries {
            if library.targets.iter().any(|value| value == target) {
                pending.extend(library.libraries.iter().cloned());
            }
        }
        let mut directives = Vec::new();
        for section in self
            .exports
            .iter()
            .chain(&self.reexports)
            .filter(|section| section.targets.iter().any(|value| value == target))
        {
            for (symbols, weak) in [(&section.symbols, false), (&section.weak_symbols, true)] {
                for symbol in symbols {
                    if symbol.starts_with("$ld$") {
                        if selected.is_some() {
                            directives.push(symbol);
                        }
                    } else {
                        insert(
                            exports,
                            symbol.clone(),
                            NativeExport {
                                kind: SystemExportKind::Symbol,
                                weak,
                            },
                        )?;
                    }
                }
            }
            for symbol in &section.thread_local_symbols {
                insert(
                    exports,
                    symbol.clone(),
                    NativeExport {
                        kind: SystemExportKind::ThreadLocal,
                        weak: false,
                    },
                )?;
            }
            for (names, prefixes) in [
                (
                    &section.objc_classes,
                    &["_OBJC_CLASS_$_", "_OBJC_METACLASS_$_"][..],
                ),
                (&section.objc_eh_types, &["_OBJC_EHTYPE_$_"][..]),
                (&section.objc_ivars, &["_OBJC_IVAR_$_"][..]),
            ] {
                for name in names {
                    for prefix in prefixes {
                        insert(
                            exports,
                            format!("{prefix}{name}"),
                            NativeExport {
                                kind: SystemExportKind::Symbol,
                                weak: false,
                            },
                        )?;
                    }
                }
            }
        }
        let mut result = StubDirectives::default();
        for directive in directives {
            result.apply(exports, directive, platform, deployment, self)?;
        }
        Ok(result)
    }
}

pub(super) fn insert(
    exports: &mut BTreeMap<String, NativeExport>,
    symbol: String,
    kind: NativeExport,
) -> Result<(), ToolchainError> {
    if let Some(previous) = exports.insert(symbol.clone(), kind)
        && previous.kind != kind.kind
    {
        return Err(ToolchainError::new(
            scoop_source::DiagnosticCode::E2303,
            format!("SDK export {symbol} has conflicting TLS storage"),
        ));
    }
    Ok(())
}
