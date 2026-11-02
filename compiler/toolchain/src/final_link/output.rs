use super::*;
use std::ffi::OsString;

pub(super) const NATIVE_APP_ENTRY: &str = "scoop_app_init";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum FinalOutputKind {
    #[default]
    Executable,
    SharedLibrary,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub(super) enum OutputConfiguration {
    Executable,
    SharedLibrary { loader_name: Option<String> },
}

impl OutputConfiguration {
    pub(super) fn resolve(
        target: LirTargetProfile,
        options: &FinalLinkOptions,
    ) -> Result<Self, ToolchainError> {
        let format = target.native_object_format();
        if options.install_name.is_some()
            && (options.output_kind != FinalOutputKind::SharedLibrary
                || format != scoop_lir::NativeObjectFormat::MachO64)
        {
            return Err(error(
                "install name requires a Darwin shared library output",
            ));
        }
        if options.soname.is_some()
            && (options.output_kind != FinalOutputKind::SharedLibrary
                || format != scoop_lir::NativeObjectFormat::Elf64)
        {
            return Err(error("SONAME requires an ELF shared library output"));
        }
        match options.output_kind {
            FinalOutputKind::Executable => Ok(Self::Executable),
            FinalOutputKind::SharedLibrary => {
                let loader_name = options.install_name.as_ref().or(options.soname.as_ref());
                if loader_name.is_some_and(|name| name.is_empty() || name.contains('\0')) {
                    return Err(error(
                        "shared library loader name must be nonempty and contain no NUL",
                    ));
                }
                Ok(Self::SharedLibrary {
                    loader_name: loader_name.cloned(),
                })
            }
        }
    }

    pub(super) fn kind(&self) -> FinalOutputKind {
        match self {
            Self::Executable => FinalOutputKind::Executable,
            Self::SharedLibrary { .. } => FinalOutputKind::SharedLibrary,
        }
    }

    pub(super) fn loader_name(
        &self,
        output: &Path,
        prefix: &str,
    ) -> Result<OsString, ToolchainError> {
        let Self::SharedLibrary { loader_name } = self else {
            return Err(error("executable output has no shared library loader name"));
        };
        if let Some(name) = loader_name {
            return Ok(name.into());
        }
        let mut name = OsString::from(prefix);
        name.push(
            output
                .file_name()
                .ok_or_else(|| error("shared library output has no basename"))?,
        );
        Ok(name)
    }
}

impl WireEncode for OutputConfiguration {
    fn encode(&self, e: &mut Encoder) -> Result<(), scoop_wire::cbor::EncodeError> {
        match self {
            Self::Executable => {
                e.array(1)?;
                e.unsigned(1)
            }
            Self::SharedLibrary { loader_name } => {
                e.array(3)?;
                e.unsigned(2)?;
                e.text(NATIVE_APP_ENTRY)?;
                e.array(u64::from(loader_name.is_some()))?;
                if let Some(name) = loader_name {
                    e.text(name)?;
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loader_names_require_the_matching_shared_library_platform() {
        for target in [
            scoop_target_data::DARWIN_AARCH64,
            scoop_target_data::LINUX_X86_64_GNU,
            scoop_target_data::WINDOWS_X86_64_MSVC_UCRT,
        ] {
            for kind in [FinalOutputKind::Executable, FinalOutputKind::SharedLibrary] {
                for install_name in [false, true] {
                    let mut options = FinalLinkOptions {
                        output_kind: kind,
                        ..Default::default()
                    };
                    if install_name {
                        options.install_name = Some("app".to_owned());
                    } else {
                        options.soname = Some("app".to_owned());
                    }
                    let expected = kind == FinalOutputKind::SharedLibrary
                        && target.native_object_format()
                            == if install_name {
                                scoop_lir::NativeObjectFormat::MachO64
                            } else {
                                scoop_lir::NativeObjectFormat::Elf64
                            };
                    assert_eq!(
                        OutputConfiguration::resolve(target, &options).is_ok(),
                        expected
                    );
                }
            }
        }
        for name in ["", "bad\0name"] {
            let options = FinalLinkOptions {
                output_kind: FinalOutputKind::SharedLibrary,
                soname: Some(name.to_owned()),
                ..Default::default()
            };
            assert!(
                OutputConfiguration::resolve(scoop_target_data::LINUX_X86_64_GNU, &options)
                    .is_err()
            );
        }
    }
}
