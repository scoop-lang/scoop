use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::ToolchainError;
use crate::build_record::{self, Input, Record, values};

#[derive(Serialize, Deserialize)]
struct Copied {
    #[serde(with = "values::path")]
    source: PathBuf,
    #[serde(with = "values::paths")]
    required: Vec<PathBuf>,
}

pub(super) fn publish(
    source: &Path,
    target: &Path,
    cache: &Path,
) -> Result<(Vec<PathBuf>, Vec<Input>), ToolchainError> {
    let key = build_record::fingerprint(
        &scoop_protocol::HostPathCarrier::from_path(target).map_err(error)?,
    )?;
    let record = build_record::cached(
        &cache.join("native-copies-v1"),
        key,
        |copy: &Copied| Ok(copy.source == source && available(&copy.required)?),
        || {
            let parent = target
                .parent()
                .ok_or_else(|| error("native output has no directory"))?;
            std::fs::create_dir_all(parent).map_err(error)?;
            let directory = tempfile::Builder::new()
                .prefix(".native-output-")
                .tempdir_in(parent)
                .map_err(error)?;
            let candidate = directory.path().join("candidate");
            let mut inputs = Vec::new();
            let mut paths = Vec::new();
            copy(source, &candidate, &mut inputs, &mut paths)?;
            let required = paths
                .iter()
                .map(|path| {
                    let relative = path
                        .strip_prefix(&candidate)
                        .expect("copied path belongs to candidate");
                    if relative.as_os_str().is_empty() {
                        target.to_owned()
                    } else {
                        target.join(relative)
                    }
                })
                .collect();
            let old = directory.path().join("previous");
            let previous = match std::fs::rename(target, &old) {
                Ok(()) => true,
                Err(cause) if cause.kind() == std::io::ErrorKind::NotFound => false,
                Err(cause) => return Err(error(cause)),
            };
            if let Err(cause) = std::fs::rename(&candidate, target) {
                if previous {
                    std::fs::rename(&old, target).map_err(error)?;
                }
                return Err(error(cause));
            }
            Ok(Record {
                inputs,
                result: Copied {
                    source: source.to_owned(),
                    required,
                },
            })
        },
    )?;
    Ok((record.result.required, record.inputs))
}

fn copy(
    source: &Path,
    target: &Path,
    inputs: &mut Vec<Input>,
    required: &mut Vec<PathBuf>,
) -> Result<(), ToolchainError> {
    inputs.push(Input::read(source)?);
    let metadata = source.symlink_metadata().map_err(error)?;
    if metadata.file_type().is_symlink() {
        #[cfg(unix)]
        std::os::unix::fs::symlink(std::fs::read_link(source).map_err(error)?, target)
            .map_err(error)?;
        #[cfg(not(unix))]
        return Err(error("Swift native publication requires macOS"));
    } else if metadata.is_dir() {
        std::fs::create_dir(target).map_err(error)?;
        for entry in std::fs::read_dir(source).map_err(error)? {
            let entry = entry.map_err(error)?;
            copy(
                &entry.path(),
                &target.join(entry.file_name()),
                inputs,
                required,
            )?;
        }
    } else {
        std::fs::copy(source, target).map_err(error)?;
    }
    required.push(target.to_owned());
    Ok(())
}

pub(super) fn available(paths: &[PathBuf]) -> Result<bool, ToolchainError> {
    for path in paths {
        if !path.try_exists().map_err(error)? {
            return Ok(false);
        }
    }
    Ok(true)
}

pub(super) fn error(cause: impl std::fmt::Display) -> ToolchainError {
    ToolchainError::new(
        scoop_source::DiagnosticCode::E2201,
        format!("native publication: {cause}"),
    )
}

pub(super) fn retire(previous: Vec<PathBuf>, required: &[PathBuf]) -> Result<(), ToolchainError> {
    let mut obsolete: Vec<_> = previous
        .into_iter()
        .filter(|p| !required.contains(p))
        .collect();
    obsolete.sort_by_key(|p| std::cmp::Reverse(p.components().count()));
    for path in obsolete {
        let metadata = match path.symlink_metadata() {
            Ok(metadata) => metadata,
            Err(cause) if cause.kind() == std::io::ErrorKind::NotFound => continue,
            Err(cause) => return Err(error(cause)),
        };
        let result = if metadata.is_dir() {
            std::fs::remove_dir(&path)
        } else {
            std::fs::remove_file(&path)
        };
        match result {
            Ok(()) => {}
            Err(cause)
                if matches!(
                    cause.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::DirectoryNotEmpty
                ) => {}
            Err(cause) => return Err(error(cause)),
        }
    }
    Ok(())
}
