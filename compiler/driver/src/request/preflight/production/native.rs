use scoop_identity::{ConeIdentity, NormalizedNativeSourcePath};
use scoop_toolchain::ToolchainError;

use super::{LoadedCurrentConeInput, LoadedSingleConeBuildRequest};
mod work;
pub(in crate::request::preflight) use work::Work;

pub(super) fn compile(
    request: &LoadedSingleConeBuildRequest,
    cone: ConeIdentity,
    work: &mut Work,
) -> Result<Vec<scoop_slib::UncompressedObjectMember>, ToolchainError> {
    let Some(product) = work.finish()? else {
        return Ok(Vec::new());
    };
    let LoadedCurrentConeInput::Manifest { manifest } = &request.current else {
        unreachable!("only manifest inputs start native production");
    };
    let selected = scoop_toolchain::selected_native_object_sources(
        manifest,
        request.target.production().lir_target(),
        request.purpose,
    );
    let objects = product.objects.objects();
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
