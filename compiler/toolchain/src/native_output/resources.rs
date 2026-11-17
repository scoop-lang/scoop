//! Deploy SwiftPM resource bundles independently of the linked executable.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::NativeLibraryOutput;
use super::files::{self, available, error};
use crate::ToolchainError;
use crate::build_record::{self, Record, values};

#[derive(Serialize, Deserialize)]
struct Deployment {
    #[serde(with = "values::digest")]
    signature: scoop_wire::Digest256,
    #[serde(with = "values::paths")]
    required: Vec<PathBuf>,
}

pub fn deploy_native_resources(
    outputs: &[NativeLibraryOutput],
    executable: &Path,
    cache: &Path,
) -> Result<Vec<NativeLibraryOutput>, ToolchainError> {
    let executable = std::path::absolute(executable).map_err(error)?;
    let parent = executable
        .parent()
        .ok_or_else(|| error("output has no directory"))?;
    let path = |path: &Path| scoop_protocol::HostPathCarrier::from_path(path).map_err(error);
    let key = build_record::fingerprint(&path(&executable)?)?;
    let directory = cache.join("native-resources-v1");
    let record_path = directory.join(format!("{key}.json"));
    let mut outputs = outputs.to_vec();
    let mut copies = BTreeMap::new();
    let mut names = BTreeMap::new();
    for resource in outputs.iter_mut().flat_map(|output| &mut output.resources) {
        let basename = resource
            .file_name()
            .ok_or_else(|| error("resource has no basename"))?;
        let destination = parent.join(basename);
        if destination == executable {
            return Err(error(format!(
                "native resource conflicts with executable {}",
                executable.display()
            )));
        }
        let name = basename.to_string_lossy().to_lowercase();
        if let Some(previous) = names.insert(name, resource.clone())
            && previous != *resource
        {
            return Err(error(format!(
                "conflicting native resource {}: {} and {}",
                destination.display(),
                previous.display(),
                resource.display()
            )));
        }
        if *resource != destination {
            copies.insert(destination.clone(), resource.clone());
        }
        *resource = destination;
    }
    if copies.is_empty() && !record_path.try_exists().map_err(error)? {
        return Ok(outputs);
    }
    let signature = build_record::fingerprint(
        &copies
            .iter()
            .map(|(destination, source)| Ok((path(destination)?, path(source)?)))
            .collect::<Result<Vec<_>, ToolchainError>>()?,
    )?;
    build_record::cached(
        &directory,
        key,
        |result: &Deployment| Ok(result.signature == signature && available(&result.required)?),
        || {
            let mut inputs = Vec::new();
            let mut required = Vec::new();
            for (destination, source) in &copies {
                let (files, observed) = files::publish(source, destination, cache)?;
                required.extend(files);
                inputs.extend(observed);
            }
            match std::fs::File::open(&record_path) {
                Ok(file) => {
                    let previous: Record<Deployment> =
                        serde_json::from_reader(file).map_err(error)?;
                    let obsolete = previous
                        .result
                        .required
                        .into_iter()
                        .filter(|old| {
                            !outputs
                                .iter()
                                .flat_map(|output| &output.resources)
                                .any(|current| old.starts_with(current))
                        })
                        .collect();
                    files::retire(obsolete, &required)?;
                }
                Err(cause) if cause.kind() == std::io::ErrorKind::NotFound => {}
                Err(cause) => return Err(error(cause)),
            }
            Ok(Record {
                inputs,
                result: Deployment {
                    signature,
                    required,
                },
            })
        },
    )?;
    Ok(outputs)
}
