use std::path::{Path, PathBuf};

use scoop_lir::LirTargetProfile;

use crate::ToolchainError;

pub(crate) fn installed_unwind_include() -> Result<PathBuf, ToolchainError> {
    crate::installed_executable()?
        .parent()
        .and_then(Path::parent)
        .map(|root| root.join("libexec/scoop/include/unwind"))
        .ok_or_else(|| ToolchainError("cannot locate the installed unwind headers".into()))
}

pub(crate) fn link_unwind_include(
    prefix: &Path,
    explicit: bool,
) -> Result<PathBuf, ToolchainError> {
    let installed = installed_unwind_include()?;
    let include = if !explicit && installed.is_dir() {
        installed
    } else {
        prefix.join("include")
    };
    if !include.join("unwind.h").is_file() {
        return Err(ToolchainError(format!(
            "missing LLVM unwind headers at {}; provide --unwind-prefix or prepare the target unwind resources",
            include.display()
        )));
    }
    std::path::absolute(include).map_err(|error| ToolchainError(error.to_string()))
}

pub fn selected_unwind_prefix(
    target: LirTargetProfile,
    sysroot: Option<&Path>,
    explicit: Option<&Path>,
) -> Result<PathBuf, ToolchainError> {
    if let Some(prefix) = explicit {
        return Ok(prefix.to_owned());
    }
    let sysroot = crate::configured_sysroot_root(sysroot.map(Path::to_owned))?;
    Ok(crate::target_library_root(&sysroot, target).join("unwind"))
}

pub fn runtime_unwind_include(
    target: LirTargetProfile,
    prefix: Option<&Path>,
) -> Result<Option<PathBuf>, ToolchainError> {
    let os = scoop_target_data::definition(target.id())
        .ok_or_else(|| ToolchainError("unknown runtime target".into()))?
        .os;
    match os {
        scoop_target_data::TargetOs::Darwin => {
            if prefix.is_some() {
                return Err(ToolchainError(
                    "Darwin runtime uses the SDK unwind headers".into(),
                ));
            }
            return Ok(None);
        }
        scoop_target_data::TargetOs::Linux | scoop_target_data::TargetOs::Windows => {}
    }
    let prefix = selected_unwind_prefix(target, None, prefix)?;
    let include = prefix.join("include");
    if !include.join("unwind.h").is_file() {
        return Err(ToolchainError(format!(
            "missing LLVM unwind headers at {}; provide --unwind-prefix or run scripts/build_llvm_unwind.py for {}",
            include.display(),
            target.contract().canonical_triple()
        )));
    }
    // Keep this locator's symlinks: the dependency cache must observe changes
    // to the selected header alias, not only to its previous real path.
    std::path::absolute(include)
        .map(Some)
        .map_err(|error| ToolchainError(error.to_string()))
}
