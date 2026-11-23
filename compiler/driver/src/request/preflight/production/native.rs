use std::path::Path;

use scoop_identity::{ConeIdentity, NormalizedNativeSourcePath};
use scoop_toolchain::ToolchainError;

use super::{LoadedCurrentConeInput, LoadedSingleConeBuildRequest};
use crate::request::NativeInputOrigin;

pub(super) fn compile(
    request: &LoadedSingleConeBuildRequest,
    cone: ConeIdentity,
    temporary_parent: &Path,
) -> Result<Vec<scoop_slib::UncompressedObjectMember>, ToolchainError> {
    let LoadedCurrentConeInput::Manifest { manifest } = &request.current else {
        if let NativeInputOrigin::Objects(inputs) = &request.native_inputs
            && !inputs.is_empty()
        {
            return Err(error(
                scoop_source::DiagnosticCode::E2001,
                "single-file input cannot have native translation units",
            ));
        }
        return Ok(Vec::new());
    };
    let target = request.target.production();
    let selected =
        scoop_toolchain::selected_native_object_sources(manifest, target, request.purpose);
    if selected.is_empty() && matches!(&request.native_inputs, NativeInputOrigin::Source { .. }) {
        return Ok(Vec::new());
    }
    let directory;
    let prepared;
    let objects = match &request.native_inputs {
        NativeInputOrigin::Objects(objects) => objects.as_slice(),
        NativeInputOrigin::Source { sysroot } => {
            let sysroot = scoop_toolchain::configured_sysroot_root(sysroot.clone())?;
            directory = tempfile::Builder::new()
                .prefix("native-")
                .tempdir_in(temporary_parent)
                .map_err(|cause| error(scoop_source::DiagnosticCode::E2201, cause))?;
            prepared = scoop_toolchain::prepare_native_objects(
                manifest,
                target.lir_target(),
                || Ok(target),
                request.optimization,
                &sysroot,
                directory.path(),
                request.purpose,
            )?;
            prepared.objects()
        }
    };
    if selected.len() != objects.len() {
        return Err(error(
            scoop_source::DiagnosticCode::ICE2201,
            format!(
                "native object count {} does not match {} selected sources",
                objects.len(),
                selected.len()
            ),
        ));
    }
    selected
        .iter()
        .zip(objects)
        .map(|(source, object)| {
            let source = NormalizedNativeSourcePath::new(source.as_str())
                .map_err(|cause| error(scoop_source::DiagnosticCode::ICE2401, cause))?;
            scoop_slib::native_link_object_member(
                cone,
                &source,
                std::fs::read(object)
                    .map_err(|cause| error(scoop_source::DiagnosticCode::E2201, cause))?,
            )
            .map_err(|cause| error(scoop_source::DiagnosticCode::ICE2302, cause))
        })
        .collect()
}

fn error(code: scoop_source::DiagnosticCode, error: impl std::fmt::Display) -> ToolchainError {
    ToolchainError::new(code, format!("native compilation: {error}"))
}

pub(super) fn libraries(
    request: &LoadedSingleConeBuildRequest,
) -> Result<Vec<scoop_lir::CanonicalNativeLibraryRequirementV1>, ToolchainError> {
    match &request.current {
        LoadedCurrentConeInput::Manifest { manifest } => manifest
            .parsed()
            .semantic()
            .native()
            .library_requirements(request.target.production().id())
            .map_err(|cause| error(scoop_source::DiagnosticCode::E2017, cause)),
        LoadedCurrentConeInput::SingleFile { .. } => Ok(Vec::new()),
    }
}

pub(super) fn cxx(request: &LoadedSingleConeBuildRequest) -> bool {
    matches!(&request.current, LoadedCurrentConeInput::Manifest { manifest } if manifest.parsed().semantic().native().cxx(request.target.production().id()))
}
