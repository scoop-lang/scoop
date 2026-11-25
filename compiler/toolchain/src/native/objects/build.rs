//! Preprocess and compile bounded translation units, retaining only objects.

use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;

pub(super) fn build(
    manifest: &LoadedConeManifest,
    target: &crate::ResolvedTargetProfile,
    optimization: OptimizationMode,
    sysroot: &Path,
    directory: &Path,
    workers: &scoop_process::WorkerSlots,
) -> Result<Record<Outputs>, ToolchainError> {
    let preparation_slot = workers.acquire();
    let compiler = NativeToolchain::resolve_target(target, manifest)?;
    let preparation =
        prepare::Preparation::new(manifest, target.id(), &compiler, optimization, sysroot)?
            .expect("native object work has selected translation units");
    let temporary = tempfile::Builder::new()
        .prefix("objects-")
        .tempdir_in(directory)
        .map_err(|cause| error(scoop_source::DiagnosticCode::E2201, cause))?;
    drop(preparation_slot);
    let observed = Mutex::new(BTreeSet::new());
    let compile = |index| {
        let _slot = workers.acquire();
        let scratch = tempfile::Builder::new()
            .prefix(&format!("unit-{index:08}-"))
            .tempdir_in(temporary.path())
            .map_err(|cause| error(scoop_source::DiagnosticCode::E2201, cause))?;
        let (unit, paths) = preparation.unit(index, scratch.path())?;
        let name = format!("{index:08}.o");
        let object = scratch.path().join(&name);
        compile_native_source(
            &unit,
            manifest.parsed().semantic().native(),
            &compiler,
            optimization,
            &object,
        )?;
        drop(unit);
        let published = temporary.path().join(name);
        std::fs::rename(object, &published)
            .map_err(|cause| error(scoop_source::DiagnosticCode::E2201, cause))?;
        observed
            .lock()
            .expect("native input collection is not poisoned")
            .extend(paths);
        Ok(published)
    };
    let units = if workers.limit().get() == 1 || preparation.len() == 1 {
        (0..preparation.len())
            .map(compile)
            .collect::<Result<Vec<_>, ToolchainError>>()?
    } else {
        let next = AtomicUsize::new(0);
        std::thread::scope(|scope| {
            let handles = (0..workers.limit().get().min(preparation.len()))
                .map(|worker| {
                    let compile = &compile;
                    let next = &next;
                    let count = preparation.len();
                    std::thread::Builder::new()
                        .name(format!("scoop-native-unit-{worker}"))
                        .spawn_scoped(scope, move || {
                            let mut results = Vec::new();
                            loop {
                                let index = next.fetch_add(1, Ordering::Relaxed);
                                if index >= count {
                                    return results;
                                }
                                results.push((index, compile(index)));
                            }
                        })
                        .map_err(|cause| error(scoop_source::DiagnosticCode::E2201, cause))
                })
                .collect::<Result<Vec<_>, _>>()?;
            let mut results = Vec::with_capacity(preparation.len());
            for handle in handles {
                results.extend(handle.join().map_err(|_| {
                    error(
                        scoop_source::DiagnosticCode::ICE2201,
                        "native worker panicked",
                    )
                })?);
            }
            results.sort_unstable_by_key(|(index, _)| *index);
            results
                .into_iter()
                .map(|(_, result)| result)
                .collect::<Result<Vec<_>, ToolchainError>>()
        })?
    };
    let mut observed = observed
        .into_inner()
        .expect("native input collection is not poisoned");
    observed.extend(preparation.observed_paths);
    let mut objects = Vec::with_capacity(units.len());
    for object in units {
        objects.push(
            HostPathCarrier::from_path(&object)
                .map_err(|cause| error(scoop_source::DiagnosticCode::ICE2201, cause))?,
        );
    }
    let inputs = observed
        .iter()
        .map(|path| Input::read(path))
        .collect::<Result<_, _>>()?;
    let _ = temporary.keep();
    Ok(Record {
        inputs,
        result: Outputs { objects },
    })
}
