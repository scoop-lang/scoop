//! Directory selection is re-evaluated only when a traversed directory changes.
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{Input, ToolchainError, error, values};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) enum Rule {
    All,
    Manifests,
    Sources {
        #[serde(with = "values::paths")]
        include: Vec<PathBuf>,
        #[serde(with = "values::paths")]
        exclude: Vec<PathBuf>,
    },
}

#[derive(Serialize, Deserialize)]
pub(super) struct Selection {
    #[serde(with = "values::path")]
    root: PathBuf,
    rule: Rule,
    directories: Vec<Input>,
    #[serde(with = "values::paths")]
    files: Vec<PathBuf>,
}

impl Selection {
    pub(super) fn read(root: PathBuf, rule: Rule) -> Result<Self, ToolchainError> {
        let mut result = Self {
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
        if let Rule::Sources { exclude, .. } = &self.rule
            && exclude
                .iter()
                .any(|excluded| relative.starts_with(excluded))
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
                if !matches!(self.rule, Rule::All) && name.to_string_lossy().starts_with('.') {
                    continue;
                }
                let child = relative.join(&name);
                if matches!(self.rule, Rule::Manifests) {
                    let name = name.to_string_lossy();
                    if !relative.as_os_str().is_empty()
                        || !(name == "Package.swift"
                            || name.starts_with("Package@swift-") && name.ends_with(".swift"))
                    {
                        continue;
                    }
                }
                self.walk(&child, ancestors)?;
            }
            ancestors.remove(&canonical);
        } else if self.selected(relative) {
            self.files.push(path);
        }
        Ok(())
    }

    fn selected(&self, relative: &Path) -> bool {
        match &self.rule {
            Rule::All | Rule::Manifests => true,
            Rule::Sources { include, .. } => {
                (include.is_empty() || include.iter().any(|entry| relative.starts_with(entry)))
                    && (relative
                        .file_name()
                        .is_some_and(|name| name == "module.modulemap")
                        || relative
                            .extension()
                            .and_then(|e| e.to_str())
                            .is_some_and(|extension| {
                                matches!(
                                    extension,
                                    "swift"
                                        | "c"
                                        | "m"
                                        | "mm"
                                        | "cc"
                                        | "cpp"
                                        | "cxx"
                                        | "c++"
                                        | "s"
                                        | "S"
                                        | "h"
                                        | "hh"
                                        | "hpp"
                                        | "hxx"
                                        | "modulemap"
                                )
                            }))
            }
        }
    }

    pub(super) fn files(&self) -> &[PathBuf] {
        &self.files
    }

    pub(super) fn is_current(&self) -> Result<bool, ToolchainError> {
        for directory in &self.directories {
            if !directory.is_current()? {
                return Ok(Self::read(self.root.clone(), self.rule.clone())?.files == self.files);
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
