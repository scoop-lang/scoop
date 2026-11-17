//! Directory selection is re-evaluated only when a traversed directory changes.
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{Input, ToolchainError, error, values};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) enum Rule {
    All,
    Package {
        #[serde(with = "values::path")]
        cache: PathBuf,
    },
}

#[derive(Serialize, Deserialize)]
pub(super) struct Selection {
    #[serde(with = "values::path")]
    root: PathBuf,
    present: bool,
    rule: Rule,
    directories: Vec<Input>,
    #[serde(with = "values::paths")]
    files: Vec<PathBuf>,
}

impl Selection {
    pub(super) fn read(root: PathBuf, rule: Rule) -> Result<Self, ToolchainError> {
        let mut result = Self {
            present: root.try_exists().map_err(error)?,
            root,
            rule,
            directories: Vec::new(),
            files: Vec::new(),
        };
        result.walk(Path::new(""), &mut BTreeSet::new())?;
        result.files.sort();
        result.files.dedup();
        Ok(result)
    }

    fn walk(
        &mut self,
        relative: &Path,
        ancestors: &mut BTreeSet<PathBuf>,
    ) -> Result<(), ToolchainError> {
        let path = if relative.as_os_str().is_empty() {
            self.root.clone()
        } else {
            self.root.join(relative)
        };
        if let Rule::Package { cache } = &self.rule
            && (path.starts_with(cache)
                || relative
                    .components()
                    .any(|c| c.as_os_str() == ".git" || c.as_os_str() == ".build"))
        {
            return Ok(());
        }
        let metadata = match path.metadata() {
            Ok(value) => value,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                self.directories.push(Input::read(&path)?);
                return Ok(());
            }
            Err(e) => return Err(error(format!("{}: {e}", path.display()))),
        };
        if metadata.is_dir() {
            // Prevent recursive directory links; paths otherwise have normal I/O semantics.
            let canonical = path.canonicalize().map_err(error)?;
            if !ancestors.insert(canonical.clone()) {
                return Err(error(format!(
                    "recursive input directory {}",
                    path.display()
                )));
            }
            self.directories.push(Input::read(&path)?);
            let mut entries = std::fs::read_dir(&path)
                .map_err(error)?
                .map(|entry| entry.map(|entry| entry.file_name()))
                .collect::<Result<Vec<_>, _>>()
                .map_err(error)?;
            entries.sort();
            for name in entries {
                let child = relative.join(&name);
                self.walk(&child, ancestors)?;
            }
            ancestors.remove(&canonical);
        } else {
            self.files.push(path);
        }
        Ok(())
    }

    pub(super) fn files(&self) -> &[PathBuf] {
        &self.files
    }

    pub(super) fn is_current(&self) -> Result<bool, ToolchainError> {
        for directory in &self.directories {
            if !directory.is_current()? {
                let current = Self::read(self.root.clone(), self.rule.clone())?;
                return Ok(current.present == self.present && current.files == self.files);
            }
        }
        Ok(true)
    }
}

pub(super) fn add_selection(
    root: PathBuf,
    rule: Rule,
    files: &mut BTreeSet<PathBuf>,
    selections: &mut Vec<Selection>,
) -> Result<(), ToolchainError> {
    let selection = Selection::read(root, rule)?;
    files.extend(selection.files.iter().cloned());
    selections.push(selection);
    Ok(())
}
