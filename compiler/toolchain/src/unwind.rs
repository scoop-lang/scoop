use std::path::{Path, PathBuf};

use scoop_lir::LirTargetProfile;

use crate::ToolchainError;

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
    let backend = scoop_target_data::definition(target.id())
        .ok_or_else(|| {
            ToolchainError::new(
                scoop_source::DiagnosticCode::E2307,
                "unknown runtime target".into(),
            )
        })?
        .selection
        .backend()
        .contract();
    if backend.unwind_provider == scoop_lir::UnwindProviderV1::DarwinLibSystem {
        if prefix.is_some() {
            return Err(ToolchainError::new(
                scoop_source::DiagnosticCode::E2307,
                "Darwin runtime uses the SDK unwind headers".into(),
            ));
        }
        return Ok(None);
    }
    let prefix = selected_unwind_prefix(target, None, prefix)?;
    let include = prefix.join("include");
    if !include.join("unwind.h").is_file() {
        return Err(ToolchainError::new(
            scoop_source::DiagnosticCode::E2302,
            format!("missing LLVM unwind headers at {}", include.display()),
        )
        .with_help(
            scoop_source::DiagnosticCode::H2301,
            format!(
                "provide --unwind-prefix or run scripts/build_llvm_unwind.py for {}",
                target.contract().canonical_triple()
            ),
        ));
    }
    // Keep this locator's symlinks: the dependency cache must observe changes
    // to the selected header alias, not only to its previous real path.
    std::path::absolute(include).map(Some).map_err(|error| {
        ToolchainError::new(scoop_source::DiagnosticCode::E2201, error.to_string())
    })
}
