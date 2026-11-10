//! Host tools used only when producing an independent native debug file.

use std::path::{Path, PathBuf};

use crate::ToolchainError;

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum DebugFileKind {
    Dsym,
    Elf,
    Pdb,
}

impl DebugFileKind {
    pub fn output_path(self, executable: &Path) -> PathBuf {
        let mut name = executable.as_os_str().to_os_string();
        name.push(match self {
            Self::Dsym => ".dSYM",
            Self::Elf => ".debug",
            Self::Pdb => ".pdb",
        });
        PathBuf::from(name)
    }
}

pub enum NativeDebugTools {
    Windows,
    Darwin {
        dsymutil: PathBuf,
        strip: PathBuf,
        codesign: PathBuf,
    },
    Elf {
        objcopy: PathBuf,
    },
}

impl NativeDebugTools {
    pub fn resolve(target: scoop_lir::LirTargetProfile) -> Result<Self, ToolchainError> {
        let definition = scoop_target_data::definition(target.id())
            .filter(|definition| definition.selection.target() == target)
            .ok_or_else(|| ToolchainError("a registered debug target is required".into()))?;
        match target.native_object_format() {
            scoop_lir::NativeObjectFormat::MachO64
                if definition.os == scoop_target_data::TargetOs::Darwin =>
            {
                let dsymutil = llvm_tool("dsymutil")?;
                let strip = PathBuf::from("/usr/bin/strip");
                let codesign = PathBuf::from("/usr/bin/codesign");
                Ok(Self::Darwin {
                    dsymutil,
                    strip,
                    codesign,
                })
            }
            scoop_lir::NativeObjectFormat::Elf64 => Ok(Self::Elf {
                objcopy: llvm_tool("llvm-objcopy")?,
            }),
            scoop_lir::NativeObjectFormat::Coff64
                if definition.os == scoop_target_data::TargetOs::Windows =>
            {
                Ok(Self::Windows)
            }
            format => Err(ToolchainError(format!(
                "no native debug tools for {:?}/{format:?}",
                definition.os
            ))),
        }
    }

    pub fn kind(&self) -> DebugFileKind {
        match self {
            Self::Windows => DebugFileKind::Pdb,
            Self::Darwin { .. } => DebugFileKind::Dsym,
            Self::Elf { .. } => DebugFileKind::Elf,
        }
    }
}

fn llvm_tool(name: &str) -> Result<PathBuf, ToolchainError> {
    let mut candidates = Vec::new();
    if let Some(prefix) = std::env::var_os("LLVM_SYS_221_PREFIX") {
        candidates.push(PathBuf::from(prefix).join("bin").join(name));
    }
    candidates.push(crate::installed_executable()?.with_file_name(format!("scoop-{name}")));
    if let Some(path) = std::env::var_os("PATH") {
        candidates.extend(std::env::split_paths(&path).map(|root| root.join(name)));
    }
    for candidate in candidates {
        let candidate = std::path::absolute(candidate)
            .map_err(|error| ToolchainError(format!("debug tool path: {error}")))?;
        let exists = candidate.is_file();
        if exists {
            return Ok(candidate);
        }
    }
    Err(ToolchainError(format!(
        "debug tool {name} not found beside Scoop or on PATH; set LLVM_SYS_221_PREFIX to the LLVM 22.1 installation"
    )))
}
