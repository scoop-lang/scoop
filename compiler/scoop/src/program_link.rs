//! Read the built artifacts when final linking needs their objects.
use scoop_linker::LinkError;
use scoop_toolchain::ResolvedTargetProfile;

use crate::ValidatedArtifactClosure;

pub fn read_built_program(
    artifacts: &ValidatedArtifactClosure,
    target: &ResolvedTargetProfile,
) -> Result<scoop_slib::ProgramLinkClosure, LinkError> {
    let root = artifacts.artifact(artifacts.root()).ok_or_else(|| {
        LinkError::new(
            scoop_source::DiagnosticCode::ICE2201,
            format!("build closure is missing root {}", artifacts.root()),
        )
    })?;
    let dependencies: Vec<_> = artifacts
        .artifacts()
        .filter(|(identity, _)| *identity != artifacts.root())
        .map(|(_, artifact)| artifact.file())
        .collect::<Result<_, _>>()
        .map_err(|error| LinkError::new(scoop_source::DiagnosticCode::E2201, error))?;
    let closure = scoop_slib::read_program_link_closure(
        root.file()
            .map_err(|error| LinkError::new(scoop_source::DiagnosticCode::E2201, error))?,
        &dependencies,
        target.lir_target_selection(),
    )
    .map_err(|error| LinkError::new(error.code(), format!("built program Link input: {error}")))?;
    Ok(closure)
}
