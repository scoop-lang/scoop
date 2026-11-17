//! Publish a portable companion directory without reading outputs on a hit.
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::files::{self, available, error};
use super::{NativeLibraryOutput, NativeOutputsDescriptor};
use crate::ToolchainError;
use crate::build_record::{self, Input, Record, values};
use crate::swift::SwiftProduct;

mod settings;

#[derive(Serialize, Deserialize)]
struct Publication {
    #[serde(with = "values::digest")]
    signature: scoop_wire::Digest256,
    outputs: Vec<NativeLibraryOutput>,
    #[serde(with = "values::paths")]
    required: Vec<PathBuf>,
}

pub fn publish_native_outputs(
    products: &[SwiftProduct],
    destination: &Path,
    cache: &Path,
) -> Result<Vec<NativeLibraryOutput>, ToolchainError> {
    let outputs = SwiftProduct::outputs(products)?;
    let destination = std::path::absolute(destination).map_err(error)?;
    let key = build_record::fingerprint(
        &scoop_protocol::HostPathCarrier::from_path(&destination).map_err(error)?,
    )?;
    let directory = cache.join("native-publications-v1");
    let record = directory.join(format!("{key}.json"));
    if outputs.is_empty() && !record.try_exists().map_err(error)? {
        return Ok(outputs);
    }
    let signature = build_record::fingerprint(&products)?;
    let result = build_record::cached(
        &directory,
        key,
        |result: &Publication| Ok(result.signature == signature && available(&result.required)?),
        || {
            let mut outputs = outputs;
            let mut observed = BTreeSet::new();
            let mut copies = BTreeMap::new();
            for output in &mut outputs {
                settings::collect(output, &mut observed)?;
                let mut add = |path: &Path, role: &str| -> Result<PathBuf, ToolchainError> {
                    let basename = path
                        .file_name()
                        .ok_or_else(|| error("native output has no basename"))?;
                    let target = destination.join(role).join(basename);
                    if let Some(previous) = copies.insert(target.clone(), path.to_owned())
                        && previous != path
                    {
                        return Err(error(format!(
                            "conflicting native output {}: {} and {}",
                            target.display(),
                            previous.display(),
                            path.display()
                        )));
                    }
                    Ok(target)
                };
                let relative = output
                    .binary
                    .strip_prefix(&output.library)
                    .map(Path::to_owned)
                    .ok();
                output.library = add(&output.library, "libraries")?;
                output.binary = relative
                    .filter(|r| !r.as_os_str().is_empty())
                    .map_or_else(|| output.library.clone(), |r| output.library.join(r));
                for dependency in &mut output.dependencies {
                    let relative = dependency
                        .binary
                        .strip_prefix(&dependency.library)
                        .map(Path::to_owned)
                        .ok();
                    dependency.library = add(&dependency.library, "libraries")?;
                    dependency.binary = relative.filter(|r| !r.as_os_str().is_empty()).map_or_else(
                        || dependency.library.clone(),
                        |r| dependency.library.join(r),
                    );
                }
                for resource in &mut output.resources {
                    *resource = add(resource, "resources")?;
                }
                if !output.settings.library_paths.is_empty() {
                    output.settings.library_paths = vec![destination.join("libraries")];
                }
                if !output.settings.framework_paths.is_empty() {
                    output.settings.framework_paths = vec![destination.join("libraries")];
                }
            }
            let mut required = Vec::new();
            let mut inputs = Vec::new();
            for (target, source) in &copies {
                let (paths, source_inputs) = files::publish(source, target, cache)?;
                required.extend(paths);
                inputs.extend(source_inputs);
            }
            let mut relative = outputs.clone();
            for output in &mut relative {
                output.map_paths(|path| {
                    path.strip_prefix(&destination)
                        .map(Path::to_owned)
                        .map_err(error)
                })?;
            }
            let descriptor = destination.join("native-outputs.json");
            std::fs::create_dir_all(&destination).map_err(error)?;
            let candidate = tempfile::NamedTempFile::new_in(&destination).map_err(error)?;
            serde_json::to_writer_pretty(
                candidate.as_file(),
                &NativeOutputsDescriptor {
                    schema: 1,
                    outputs: relative,
                },
            )
            .map_err(error)?;
            candidate
                .persist(&descriptor)
                .map_err(|cause| error(cause.error))?;
            required.push(descriptor);
            inputs.extend(
                observed
                    .iter()
                    .map(|path| Input::read(path))
                    .collect::<Result<Vec<_>, _>>()?,
            );
            retire(&record, &required)?;
            Ok(Record {
                inputs,
                result: Publication {
                    signature,
                    outputs,
                    required,
                },
            })
        },
    )?;
    Ok(result.result.outputs)
}

fn retire(record: &Path, required: &[PathBuf]) -> Result<(), ToolchainError> {
    let file = match std::fs::File::open(record) {
        Ok(file) => file,
        Err(cause) if cause.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(cause) => return Err(error(cause)),
    };
    let previous: Record<Publication> = serde_json::from_reader(file).map_err(error)?;
    files::retire(previous.result.required, required)
}
