use super::*;
use crate::build_record::{Input, Record, cached, fingerprint};
use scoop_protocol::HostPathCarrier;
use serde::{Deserialize, Serialize};

mod build;
mod inputs;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedNativeObjects {
    key: Digest256,
    objects: Vec<PathBuf>,
    inputs: Vec<crate::NativeInputMetadata>,
}

impl PreparedNativeObjects {
    pub fn key(&self) -> Digest256 {
        self.key
    }
    pub fn objects(&self) -> &[PathBuf] {
        &self.objects
    }
    pub fn input_metadata(&self) -> &[crate::NativeInputMetadata] {
        &self.inputs
    }
}

#[derive(Serialize, Deserialize)]
struct Outputs {
    objects: Vec<HostPathCarrier>,
}

/// Check native inputs before invoking any preprocessor or compiler.
#[allow(clippy::too_many_arguments)]
pub fn prepare_native_objects<'a>(
    manifest: &LoadedConeManifest,
    target: scoop_lir::LirTargetProfile,
    resolve: impl Fn() -> Result<&'a crate::ResolvedTargetProfile, ToolchainError>,
    optimization: OptimizationMode,
    sysroot: &Path,
    cache: &Path,
    purpose: scoop_protocol::CompilationPurpose,
    workers: impl Into<scoop_process::WorkerSlots>,
) -> Result<PreparedNativeObjects, ToolchainError> {
    let workers = workers.into();
    let mut prepared = prepare_c_objects(
        manifest,
        target,
        &resolve,
        optimization,
        sysroot,
        cache,
        &workers,
    )?;
    if let Some(resource) = crate::windows_resource::prepare(
        manifest, target, &resolve, purpose, sysroot, cache, &workers,
    )? {
        prepared.key = fingerprint(&(prepared.key.as_array(), resource.key.as_array()))?;
        prepared.objects.push(resource.object);
        prepared.inputs.extend(resource.inputs);
    }
    Ok(prepared)
}

pub fn selected_native_object_sources(
    manifest: &LoadedConeManifest,
    target: scoop_lir::LirTargetProfile,
    purpose: scoop_protocol::CompilationPurpose,
) -> Vec<ConeRelativePath> {
    let mut sources: Vec<_> = manifest
        .parsed()
        .semantic()
        .native()
        .sources()
        .iter()
        .filter(|source| source.predicate().matches(target.id()))
        .map(|source| source.path().clone())
        .collect();
    sources.sort();
    sources.extend(crate::windows_resource::source(manifest, target, purpose).cloned());
    sources
}

fn prepare_c_objects<'a>(
    manifest: &LoadedConeManifest,
    target: scoop_lir::LirTargetProfile,
    resolve: &impl Fn() -> Result<&'a crate::ResolvedTargetProfile, ToolchainError>,
    optimization: OptimizationMode,
    sysroot: &Path,
    cache: &Path,
    workers: &scoop_process::WorkerSlots,
) -> Result<PreparedNativeObjects, ToolchainError> {
    let key = inputs::configuration(manifest, target, optimization, sysroot)?;
    if !manifest
        .parsed()
        .semantic()
        .native()
        .sources()
        .iter()
        .any(|source| source.predicate().matches(target.id()))
    {
        return Ok(PreparedNativeObjects {
            key,
            objects: Vec::new(),
            inputs: Vec::new(),
        });
    }
    let directory = cache.join("native-objects-v3");
    let record = cached(
        &directory,
        key,
        |outputs: &Outputs| {
            for path in &outputs.objects {
                if !path
                    .to_path_buf()
                    .map_err(|cause| error(scoop_source::DiagnosticCode::ICE2201, cause))?
                    .try_exists()
                    .map_err(|cause| error(scoop_source::DiagnosticCode::E2201, cause))?
                {
                    return Ok(false);
                }
            }
            Ok(true)
        },
        || {
            build::build(
                manifest,
                resolve()?,
                optimization,
                sysroot,
                &directory,
                workers,
            )
        },
    )?;
    Ok(PreparedNativeObjects {
        key: fingerprint(&(key.as_array(), &record.inputs))?,
        objects: record
            .result
            .objects
            .into_iter()
            .map(|p| {
                p.to_path_buf()
                    .map_err(|cause| error(scoop_source::DiagnosticCode::ICE2201, cause))
            })
            .collect::<Result<_, _>>()?,
        inputs: record.inputs,
    })
}

/// Configuration participates in the Cone key; observed native files are
/// checked through the completed Cone's compiler input record.
pub fn native_configuration_key(
    manifest: &LoadedConeManifest,
    target: scoop_lir::LirTargetProfile,
    optimization: OptimizationMode,
    sysroot: &Path,
    purpose: scoop_protocol::CompilationPurpose,
) -> Result<Digest256, ToolchainError> {
    let c = inputs::configuration(manifest, target, optimization, sysroot)?;
    let resource = crate::windows_resource::source(manifest, target, purpose);
    let includes = if resource.is_some() {
        manifest
            .parsed()
            .semantic()
            .native()
            .includes(target.id())
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let resource_sysroot = resource
        .map(|_| HostPathCarrier::from_path(sysroot))
        .transpose()
        .map_err(|cause| error(scoop_source::DiagnosticCode::ICE2201, cause))?;
    fingerprint(&(c.as_array(), resource, includes, resource_sysroot))
}
