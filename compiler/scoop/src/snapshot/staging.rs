use std::fmt;
use std::path::{Path, PathBuf};

use crate::ArtifactCacheRoot;
use crate::cache::COMPILE_CACHE_NAMESPACE;

#[derive(Debug)]
pub(super) struct PreparedStaging {
    parent: PathBuf,
    root: std::sync::OnceLock<tempfile::TempDir>,
}

impl PreparedStaging {
    pub(super) fn new(cache_root: &ArtifactCacheRoot) -> Self {
        Self {
            parent: cache_root
                .as_path()
                .join(COMPILE_CACHE_NAMESPACE)
                .join(".staging"),
            root: std::sync::OnceLock::new(),
        }
    }

    pub(super) fn root(&self) -> Result<&Path, StagingError> {
        if self.root.get().is_none() {
            create_directory(&self.parent)?;
            let root = tempfile::Builder::new()
                .prefix("build-")
                .tempdir_in(&self.parent)
                .map_err(|source| StagingError {
                    operation: StagingIoOperation::CreateDirectory,
                    path: self.parent.clone(),
                    source,
                })?;
            // If another caller initialized first, dropping this unused directory
            // leaves the shared staging lifetime with the existing owner.
            let _ = self.root.set(root);
        }
        Ok(self.root.get().expect("staging was initialized").path())
    }

    pub(super) fn plan_output(&self, cone: &str) -> Result<PathBuf, StagingError> {
        Ok(self.root()?.join(format!("{cone}.slib")))
    }
}

fn create_directory(path: &Path) -> Result<(), StagingError> {
    std::fs::create_dir_all(path).map_err(|source| StagingError {
        operation: StagingIoOperation::CreateDirectory,
        path: path.to_path_buf(),
        source,
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingIoOperation {
    CreateDirectory,
    Write,
}

impl fmt::Display for StagingIoOperation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::CreateDirectory => "create directory",
            Self::Write => "write",
        })
    }
}

#[derive(Debug)]
pub struct StagingError {
    operation: StagingIoOperation,
    path: PathBuf,
    source: std::io::Error,
}

impl fmt::Display for StagingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "cannot {} {}: {}",
            self.operation,
            self.path.display(),
            self.source
        )
    }
}

impl std::error::Error for StagingError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}
