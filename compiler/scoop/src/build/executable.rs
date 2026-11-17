//! Reuse a completed executable before reading Link artifacts or preparing tools.
use super::*;
use fs4::FileExt;
use scoop_linker::ResolvedLinkPlanFingerprint;
use scoop_toolchain::NativeInputMetadata;
use std::collections::BTreeSet;

mod key;
mod record;

pub(super) struct Request<'a> {
    pub graph: &'a crate::BuildGraphOutcome,
    pub runtime: &'a RuntimeInput,
    pub target: &'a GraphTarget,
    pub cache: &'a Path,
    pub profile: crate::BuildProfile,
    pub options: &'a scoop_toolchain::FinalLinkOptions,
    pub libraries: &'a [PathBuf],
    pub vendor_root: &'a Path,
    pub enable_gc_stress: bool,
    pub enable_startup_stats: bool,
    pub output: &'a Path,
    pub native_outputs: &'a [scoop_toolchain::NativeLibraryOutput],
}

impl Request<'_> {
    fn keep_symbols(&self) -> bool {
        self.options.keep_symbols || self.profile == crate::BuildProfile::Debug
    }

    fn emit_debug_file(&self) -> bool {
        self.options.emit_debug_file
    }

    fn enable_stack_trace(&self) -> bool {
        self.options
            .stack_trace_enabled(self.profile.optimization())
    }
}

pub(super) struct Executable {
    pub path: PathBuf,
    pub runtime_index: PathBuf,
    pub debug_file: Option<scoop_linker::DebugFile>,
    pub import_library: Option<PathBuf>,
    pub fingerprint: ResolvedLinkPlanFingerprint,
    pub result_key: [u8; 32],
    pub native_outputs: Vec<scoop_toolchain::NativeLibraryOutput>,
}

pub(super) fn get(request: Request<'_>) -> BuildResult<Executable> {
    let key = key::of(&request)?;
    let namespace = request.cache.join("final-products-v5");
    let entry = namespace.join(format!("{key}.json"));
    if let Some(mut executable) = record::load(&entry)? {
        refresh_outputs(&mut executable, request.native_outputs);
        return Ok(executable);
    }
    std::fs::create_dir_all(&namespace)
        .map_err(|error| failure(scoop_source::DiagnosticCode::E2201, error))?;
    let lock = crate::cache::open_build_lock(&namespace.join(format!("{key}.lock")))
        .map_err(|error| failure(scoop_source::DiagnosticCode::E2201, error))?;
    FileExt::lock(&lock).map_err(|error| failure(scoop_source::DiagnosticCode::E2201, error))?;
    if let Some(mut executable) = record::load(&entry)? {
        refresh_outputs(&mut executable, request.native_outputs);
        return Ok(executable);
    }
    let target = request.target.production().map_err(|error| {
        BuildFailure::tool(
            scoop_source::DiagnosticCode::E2303,
            BuildFailurePhase::FinalLink,
            error,
        )
    })?;
    let (runtime, runtime_index, runtime_paths) = runtime_input(
        request.runtime,
        target,
        request.cache,
        request.options,
        request.vendor_root,
        request.enable_gc_stress,
        request.enable_startup_stats,
    )?;
    let directory = tempfile::Builder::new()
        .prefix("program-")
        .tempdir_in(&namespace)
        .map_err(|error| failure(scoop_source::DiagnosticCode::E2201, error))?;
    let closure =
        read_built_program(request.graph.closure(), target).map_err(BuildFailure::link)?;
    let profile = scoop_linker::resolve_program_link_profile(&closure, target, request.options)
        .map_err(BuildFailure::link)?;
    let linked = scoop_linker::link_program(
        &closure,
        &runtime,
        &profile,
        scoop_linker::ProgramLinkOptions {
            native_outputs: request.native_outputs,
            keep_symbols: request.keep_symbols(),
            emit_debug_file: request.emit_debug_file(),
            enable_stack_trace: request.enable_stack_trace(),
        },
        request.libraries,
        &directory.path().join(
            if request.emit_debug_file()
                || request.options.output_kind == scoop_toolchain::FinalOutputKind::SharedLibrary
            {
                request.output.file_name().ok_or_else(|| {
                    failure(
                        scoop_source::DiagnosticCode::E2020,
                        "executable has no basename",
                    )
                })?
            } else if scoop_target_data::definition(request.target.id())
                .expect("resolved build target")
                .os
                == scoop_target_data::TargetOs::Windows
            {
                std::ffi::OsStr::new("program.exe")
            } else {
                std::ffi::OsStr::new("program")
            },
        ),
        request.cache,
    )
    .map_err(BuildFailure::link)?;
    let mut paths: BTreeSet<_> = linked.input_paths.into_iter().collect();
    paths.extend(runtime_paths);
    let inputs = paths
        .iter()
        .map(|path| NativeInputMetadata::read(path).map_err(|error| failure(error.code(), error)))
        .collect::<BuildResult<Vec<_>>>()?;
    let result_key = *scoop_wire::sha256(
        &serde_json::to_vec(&(key.as_array(), &inputs))
            .map_err(|error| failure(scoop_source::DiagnosticCode::ICE2201, error))?,
    )
    .as_array();
    let executable = Executable {
        native_outputs: linked.native_outputs,
        path: linked.path,
        runtime_index,
        debug_file: linked.debug_file,
        import_library: linked.import_library,
        fingerprint: linked.fingerprint,
        result_key,
    };
    let _ = directory.keep();
    record::save(&entry, &executable, inputs)?;
    Ok(executable)
}

fn failure(code: scoop_source::DiagnosticCode, error: impl std::fmt::Display) -> Box<BuildFailure> {
    BuildFailure::tool(code, BuildFailurePhase::FinalLink, error)
}

fn refresh_outputs(executable: &mut Executable, current: &[scoop_toolchain::NativeLibraryOutput]) {
    for output in &mut executable.native_outputs {
        if let Some(updated) = current
            .iter()
            .find(|candidate| candidate.name == output.name)
        {
            *output = updated.clone();
        }
    }
}
