//! Materialize exact generated-C atom boundaries in native objects.

use std::collections::BTreeMap;

use scoop_lir::{
    DefinitionSymbolPlanV1, GeneratedBridgeUnitPlanV1, LirTargetProfile, ObjectSymbolSurfaceV1,
    StrongDefinitionEntityKind,
};

use crate::CodegenError;
mod coff;
mod elf;
mod macho;
mod unwind;

pub(crate) fn materialize_v1(
    bytes: Vec<u8>,
    target: LirTargetProfile,
    units: &[GeneratedBridgeUnitPlanV1],
    symbols: &ObjectSymbolSurfaceV1,
) -> Result<Vec<u8>, CodegenError> {
    let definitions = symbols
        .plans()
        .iter()
        .filter_map(|definition| match definition.owner().kind() {
            StrongDefinitionEntityKind::GeneratedBridgeAtom(atom) => Some((atom, definition)),
            _ => None,
        })
        .collect::<BTreeMap<_, _>>();
    let atoms = units
        .iter()
        .flat_map(|unit| {
            std::iter::once(unit.primary_atom_authority())
                .chain(unit.materialized_associated_atom_authorities())
        })
        .map(|atom| {
            definitions.get(&atom.id()).copied().ok_or_else(|| {
                CodegenError(format!("generated-C atom {} has no symbol plan", atom.id()))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    match target.native_object_format() {
        scoop_lir::NativeObjectFormat::MachO64 => macho::materialize(bytes, target, &atoms),
        scoop_lir::NativeObjectFormat::Elf64 => {
            elf::materialize(&bytes, target.object_target(), &atoms)
        }
        scoop_lir::NativeObjectFormat::Coff64 => {
            coff::materialize(&bytes, target.object_target(), &atoms)
        }
    }
}
