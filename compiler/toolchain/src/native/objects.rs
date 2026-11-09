use super::*;
use crate::build_record::{Input, Record, cached, fingerprint};
use scoop_protocol::HostPathCarrier;
use serde::{Deserialize, Serialize};

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
pub fn prepare_native_objects<'a>(
    manifest: &LoadedConeManifest,
    target: scoop_lir::LirTargetProfile,
    resolve: impl Fn() -> Result<&'a crate::ResolvedTargetProfile, ToolchainError>,
    optimization: OptimizationMode,
    sysroot: &Path,
    cache: &Path,
    purpose: scoop_protocol::CompilationPurpose,
) -> Result<PreparedNativeObjects, ToolchainError> {
    let mut prepared = prepare_c_objects(manifest, target, &resolve, optimization, sysroot, cache)?;
    if let Some(resource) =
        crate::windows_resource::prepare(manifest, target, &resolve, purpose, sysroot, cache)?
    {
        prepared.key = fingerprint(&(prepared.key.as_array(), resource.key.as_array()))?;
        prepared.objects.push(resource.object);
        prepared.inputs.extend(resource.inputs);
    }
    Ok(prepared)
}

pub fn selected_native_object_sources(
    manifest: &LoadedConeManifest,
    target: &crate::ResolvedTargetProfile,
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
    sources
        .extend(crate::windows_resource::source(manifest, target.lir_target(), purpose).cloned());
    sources
}

fn prepare_c_objects<'a>(
    manifest: &LoadedConeManifest,
    target: scoop_lir::LirTargetProfile,
    resolve: &impl Fn() -> Result<&'a crate::ResolvedTargetProfile, ToolchainError>,
    optimization: OptimizationMode,
    sysroot: &Path,
    cache: &Path,
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
                    .map_err(error)?
                    .try_exists()
                    .map_err(error)?
                {
                    return Ok(false);
                }
            }
            Ok(true)
        },
        || {
            let compiler = NativeToolchain::resolve_target(resolve()?, manifest)?;
            let temporary = tempfile::Builder::new()
                .prefix("objects-")
                .tempdir_in(&directory)
                .map_err(error)?;
            let prepared = compiler.prepare(manifest, optimization, sysroot, temporary.path())?;
            let mut objects = Vec::new();
            for (index, unit) in prepared.units().iter().enumerate() {
                let object = temporary.path().join(format!("{index:08}.o"));
                compile_native_source(
                    unit,
                    manifest.parsed().semantic().native(),
                    &compiler,
                    optimization,
                    &object,
                )?;
                objects.push(HostPathCarrier::from_path(&object).map_err(error)?);
            }
            let inputs = prepared
                .observed_paths
                .iter()
                .map(|p| Input::read(p))
                .collect::<Result<_, _>>()?;
            for name in ["unit.d", "unit.i", "unit.ii"] {
                match std::fs::remove_file(temporary.path().join(name)) {
                    Ok(()) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(err) => return Err(error(err)),
                }
            }
            let _ = temporary.keep();
            Ok(Record {
                inputs,
                result: Outputs { objects },
            })
        },
    )?;
    Ok(PreparedNativeObjects {
        key: fingerprint(&(key.as_array(), &record.inputs))?,
        objects: record
            .result
            .objects
            .into_iter()
            .map(|p| p.to_path_buf().map_err(error))
            .collect::<Result<_, _>>()?,
        inputs: record.inputs,
    })
}
