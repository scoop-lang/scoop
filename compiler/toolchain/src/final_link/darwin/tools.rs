use std::ffi::OsStr;

use super::*;

pub(in crate::final_link) fn candidates(
    explicit: Option<&Path>,
) -> Result<Vec<PathBuf>, ToolchainError> {
    let mut paths = Vec::new();
    let name = match explicit {
        Some(path) if path.components().count() > 1 => {
            return Ok(vec![std::path::absolute(path).map_err(error)?]);
        }
        Some(path) => path.as_os_str(),
        None => {
            let executable = crate::installed_executable()?;
            let root = executable
                .parent()
                .and_then(Path::parent)
                .ok_or_else(|| error("cannot locate the installed LLVM tools"))?;
            paths.push(root.join("libexec/llvm/bin/lld"));
            if let Some(prefix) = std::env::var_os("LLVM_SYS_221_PREFIX") {
                paths.push(PathBuf::from(prefix).join("bin/lld"));
            }
            OsStr::new("lld")
        }
    };
    if let Some(search) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&search).map(|root| root.join(name)));
    }
    paths
        .into_iter()
        .map(|path| std::path::absolute(path).map_err(error))
        .collect()
}

pub(super) fn resolve(candidates: Vec<PathBuf>) -> Result<PathBuf, ToolchainError> {
    let Some(index) = candidates.iter().position(|path| path.is_file()) else {
        return Err(error(format!(
            "Mach-O LLD not found; provide --linker, install the bundled LLVM tools, or set LLVM_SYS_221_PREFIX/PATH (searched: {})",
            candidates
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )));
    };
    Ok(candidates[index].clone())
}
