use std::path::Path;

use super::*;

/// An in-memory native object containing the listed logical bridges.
#[derive(Clone, Debug)]
pub struct EmittedGeneratedCBridgeObjectMemberV1 {
    pub units: Vec<scoop_lir::GeneratedBridgeUnitPlanV1>,
    pub bytes: Vec<u8>,
}

/// Complete generated-C production for one LIR product. Temporary source and
/// compiler output files have already been released when this value is returned.
#[derive(Clone, Debug)]
pub struct EmittedGeneratedCBridgeObjectSetV1 {
    pub profile: scoop_lir::CBridgeToolchainProfileV1,
    pub production: scoop_lir::CBridgeProductionSetV1,
    pub members: Vec<EmittedGeneratedCBridgeObjectMemberV1>,
}

impl EmittedGeneratedCBridgeObjectSetV1 {
    pub const fn profile(&self) -> &scoop_lir::CBridgeToolchainProfileV1 {
        &self.profile
    }

    pub const fn production(&self) -> &scoop_lir::CBridgeProductionSetV1 {
        &self.production
    }

    pub fn members(&self) -> &[EmittedGeneratedCBridgeObjectMemberV1] {
        &self.members
    }
}

/// Compile one translation unit for the Cone's bridges and hand the object
/// bytes directly to production. Toolchain execution remains driver-owned.
pub fn emit_c_bridge_object_set(
    input: &scoop_lir::ConeLirOutput,
    temporary_parent: &Path,
    profile: &scoop_lir::CBridgeToolchainProfileV1,
    mut compile: impl FnMut(&Path, &Path) -> Result<(), CodegenError>,
) -> Result<EmittedGeneratedCBridgeObjectSetV1, CodegenError> {
    let sources = render_c_bridge_source_set(input)?;
    let production =
        scoop_lir::CBridgeProductionSetV1::from_generated_bridge_plan(sources.plan(), profile);
    let mut emitted = EmittedGeneratedCBridgeObjectSetV1 {
        profile: profile.clone(),
        production,
        members: Vec::new(),
    };
    if sources.units().is_empty() {
        return Ok(emitted);
    }
    std::fs::create_dir_all(temporary_parent).map_err(|error| {
        CodegenError(format!(
            "cannot create generated-C temporary parent {}: {error}",
            temporary_parent.display()
        ))
    })?;
    let backing = tempfile::Builder::new()
        .prefix("scoop-generated-c-")
        .tempdir_in(temporary_parent)
        .map_err(|error| CodegenError(format!("cannot create generated-C directory: {error}")))?;
    let source_path = backing.path().join("bridges.c");
    let object_path = backing.path().join("bridges.o");
    std::fs::write(&source_path, sources.source())
        .map_err(|error| CodegenError(format!("cannot write generated-C source: {error}")))?;
    compile(&source_path, &object_path)?;
    let bytes = std::fs::read(&object_path)
        .map_err(|error| CodegenError(format!("cannot read generated-C object: {error}")))?;
    let symbols = scoop_lir::ObjectSymbolSurfaceV1::from_foundation(input.foundation())
        .map_err(|error| CodegenError(format!("cannot plan generated-C symbols: {error}")))?;
    let bytes = crate::generated_c_atom_boundaries::materialize_v1(
        bytes,
        input.module().meta.target_profile,
        sources.plan().units(),
        &symbols,
    )?;
    emitted.members.push(EmittedGeneratedCBridgeObjectMemberV1 {
        units: sources.plan().units().to_vec(),
        bytes,
    });
    Ok(emitted)
}
