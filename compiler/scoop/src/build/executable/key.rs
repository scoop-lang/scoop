use super::*;
use scoop_protocol::HostPathCarrier;
use scoop_wire::{Digest256, encode, sha256};

pub(super) fn of(request: &Request<'_>) -> BuildResult<Digest256> {
    let artifacts = request
        .graph
        .artifacts()
        .iter()
        .map(|node| {
            let summary = node.artifact().summary();
            Ok((
                encode(&summary.cone().identity()).map_err(failure)?,
                encode(&summary.code_fingerprint_availability()).map_err(failure)?,
            ))
        })
        .collect::<BuildResult<Vec<_>>>()?;
    let path = |path: &Path| HostPathCarrier::from_path(path).map_err(failure);
    let options = request.options;
    let mode = match options.mode {
        None => "default",
        Some(scoop_toolchain::LinkMode::Static) => "static",
        Some(scoop_toolchain::LinkMode::Dynamic) => "dynamic",
    };
    let runtime = match request.runtime {
        RuntimeInput::ObjectIndex(index) => ("index", path(index)?),
        RuntimeInput::SourceRoot(root) => ("source", path(root)?),
    };
    let configuration = (
        artifacts,
        encode(request.graph.root().artifact().summary().cone()).map_err(failure)?,
        (
            runtime,
            request.enable_gc_stress,
            request.enable_startup_stats,
        ),
        encode(&request.target.lir_target().wire_id()).map_err(failure)?,
        request.profile.as_str(),
        request.keep_symbols(),
        request.enable_stack_trace(),
        (
            request.emit_debug_file(),
            (request.emit_debug_file()
                || options.output_kind == scoop_toolchain::FinalOutputKind::SharedLibrary)
                .then(|| {
                    request
                        .output
                        .file_name()
                        .map(std::ffi::OsStr::as_encoded_bytes)
                }),
        ),
        (
            mode,
            options.output_kind == scoop_toolchain::FinalOutputKind::SharedLibrary,
            &options.install_name,
            &options.soname,
        ),
        options.sysroot.as_deref().map(path).transpose()?,
        options.unwind_prefix.as_deref().map(path).transpose()?,
        request
            .libraries
            .iter()
            .map(|p| path(p))
            .collect::<BuildResult<Vec<_>>>()?,
    );
    let bytes = serde_json::to_vec(&configuration).map_err(failure)?;
    Ok(sha256(&bytes))
}
