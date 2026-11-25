//! Overlap independent native production with the Scoop pipeline until packaging.

use std::path::PathBuf;
use std::thread::JoinHandle;

use scoop_process::{WorkerSlot, WorkerSlots};
use scoop_protocol::CompilerInputV1;
use scoop_toolchain::{PreparedNativeObjects, ResolvedTargetProfile, ToolchainError};

use super::{LoadedCurrentConeInput, LoadedSingleConeBuildRequest, error};

pub(in crate::request::preflight) struct Work {
    workers: WorkerSlots,
    frontend: Option<WorkerSlot>,
    native: Option<NativeWork>,
    inputs: Vec<CompilerInputV1>,
}

enum NativeWork {
    Deferred(Box<Job>),
    Running(JoinHandle<Result<Product, ToolchainError>>),
}

struct Job {
    manifest: Box<scoop_manifest::LoadedConeManifest>,
    target: ResolvedTargetProfile,
    optimization: scoop_lir::OptimizationMode,
    purpose: scoop_protocol::CompilationPurpose,
    sysroot: Option<PathBuf>,
    cache: Option<PathBuf>,
    workers: WorkerSlots,
}

pub(super) struct Product {
    pub objects: PreparedNativeObjects,
    // Standalone scoopc keeps its temporary cache alive through object consumption.
    _temporary: Option<tempfile::TempDir>,
}

impl Work {
    pub(in crate::request::preflight) fn new(
        request: &LoadedSingleConeBuildRequest,
        produce: bool,
    ) -> Result<Self, ToolchainError> {
        let workers = WorkerSlots::new(request.jobs);
        let frontend = Some(workers.acquire());
        let mut work = Self {
            workers,
            frontend,
            native: None,
            inputs: Vec::new(),
        };
        if !produce || request.output.artifact().is_none() {
            return Ok(work);
        }
        let LoadedCurrentConeInput::Manifest { manifest } = &request.current else {
            return Ok(work);
        };
        let target = request.target.production();
        if scoop_toolchain::selected_native_object_sources(
            manifest,
            target.lir_target(),
            request.purpose,
        )
        .is_empty()
        {
            return Ok(work);
        }
        let job = Job {
            manifest: manifest.clone(),
            target: target.clone(),
            optimization: request.optimization,
            purpose: request.purpose,
            sysroot: request.native_sysroot.clone(),
            cache: request.native_cache.clone(),
            workers: work.workers.clone(),
        };
        work.native = Some(if work.workers.limit().get() == 1 {
            NativeWork::Deferred(Box::new(job))
        } else {
            NativeWork::Running(
                std::thread::Builder::new()
                    .name("scoop-native".into())
                    .spawn(move || job.run())
                    .map_err(|cause| error(scoop_source::DiagnosticCode::E2201, cause))?,
            )
        });
        Ok(work)
    }

    pub(in crate::request::preflight) fn workers(&self) -> &WorkerSlots {
        &self.workers
    }

    pub(in crate::request::preflight) fn release_frontend(&mut self) {
        self.frontend.take();
    }

    pub(super) fn finish(&mut self) -> Result<Option<Product>, ToolchainError> {
        let product = match self.native.take() {
            Some(NativeWork::Deferred(job)) => job.run()?,
            Some(NativeWork::Running(handle)) => handle.join().map_err(|_| {
                error(
                    scoop_source::DiagnosticCode::ICE2201,
                    "native worker panicked",
                )
            })??,
            None => return Ok(None),
        };
        self.inputs = product
            .objects
            .input_metadata()
            .iter()
            .map(|input| input.observation())
            .collect();
        self.inputs.sort_unstable();
        self.inputs.dedup();
        Ok(Some(product))
    }

    pub(in crate::request::preflight) fn take_observations(&mut self) -> Vec<CompilerInputV1> {
        std::mem::take(&mut self.inputs)
    }
}

impl Drop for Work {
    fn drop(&mut self) {
        // A failed frontend must release its slot before waiting for native workers.
        self.frontend.take();
        if let Some(NativeWork::Running(handle)) = self.native.take() {
            let _ = handle.join();
        }
    }
}

impl Job {
    fn run(self) -> Result<Product, ToolchainError> {
        let sysroot = scoop_toolchain::configured_sysroot_root(self.sysroot)?;
        let temporary = self
            .cache
            .is_none()
            .then(|| {
                tempfile::Builder::new()
                    .prefix("scoop-native-")
                    .tempdir()
                    .map_err(|cause| error(scoop_source::DiagnosticCode::E2201, cause))
            })
            .transpose()?;
        let cache = self.cache.as_deref().unwrap_or_else(|| {
            temporary
                .as_ref()
                .expect("standalone native production has a cache")
                .path()
        });
        let objects = scoop_toolchain::prepare_native_objects(
            &self.manifest,
            self.target.lir_target(),
            || Ok(&self.target),
            self.optimization,
            &sysroot,
            cache,
            self.purpose,
            &self.workers,
        )?;
        Ok(Product {
            objects,
            _temporary: temporary,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::num::NonZeroUsize;

    #[test]
    fn frontend_failure_releases_its_slot_before_joining_native_work() {
        let workers = WorkerSlots::new(NonZeroUsize::new(2).unwrap());
        let mut work = Work {
            frontend: Some(workers.acquire()),
            workers: workers.clone(),
            native: None,
            inputs: Vec::new(),
        };
        let other_worker = workers.acquire();
        let (finished, completion) = std::sync::mpsc::channel();
        work.native = Some(NativeWork::Running(std::thread::spawn(move || {
            let _slot = workers.acquire();
            finished.send(()).unwrap();
            Err(error(
                scoop_source::DiagnosticCode::E2203,
                "failed native unit",
            ))
        })));
        drop(work);
        completion.try_recv().expect("native worker was joined");
        drop(other_worker);
    }
}
