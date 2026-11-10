use super::*;
use crate::callable_atom_boundaries::{BoundaryDefinitionV1, MachOLayout};
use object::{Object, ObjectSection};

pub(super) fn materialize(
    mut bytes: Vec<u8>,
    target: LirTargetProfile,
    atoms: &[&DefinitionSymbolPlanV1],
) -> Result<Vec<u8>, CodegenError> {
    let layout = MachOLayout::parse(&bytes, target)?;
    let file =
        object::File::parse(bytes.as_slice()).map_err(|error| CodegenError(error.to_string()))?;
    let eh = file.section_by_name("__eh_frame");
    let eh_ranges = eh
        .as_ref()
        .map(|section| super::unwind::eh_frame(&file, section))
        .transpose()?;
    let compact = file.section_by_name("__compact_unwind");
    let compact_ranges = compact
        .as_ref()
        .map(|section| super::unwind::compact(&file, section))
        .transpose()?;
    let primary_ends = super::unwind::primary_ends(&file)?;
    let normalization = target.contract().native_symbol_normalization();
    let mut additions = Vec::with_capacity(atoms.len() * 2);
    for definition in atoms {
        let primary_name = normalization
            .compiler_generated_object_symbol(definition.primary_symbol().symbol().as_str())
            .into_bytes();
        let primary = layout.require_external_definition(&primary_name)?;
        let primary_section = file
            .section_by_index(object::SectionIndex(primary.section_ordinal() as usize))
            .map_err(|error| CodegenError(error.to_string()))?;
        let primary_end = primary_ends[&(primary_section.index().0, primary.value())];
        for boundary in definition.atom_boundaries() {
            let (section, start, end) = match boundary.atom_role() {
                scoop_lir::DefinitionAtomRole::Primary => (
                    layout.require_section_ordinal(primary.section_ordinal())?,
                    primary.value(),
                    primary_end,
                ),
                scoop_lir::DefinitionAtomRole::CompactUnwind => {
                    let section = layout.require_section(b"__LD", b"__compact_unwind")?;
                    {
                        let ranges = compact_ranges
                            .as_ref()
                            .ok_or_else(|| CodegenError("missing compact unwind records".into()))?;
                        let (start, end) = super::unwind::require(
                            ranges,
                            primary_section.index(),
                            primary.value(),
                        )?;
                        (section, section.address() + start, section.address() + end)
                    }
                }
                scoop_lir::DefinitionAtomRole::EhFrame => {
                    let section = layout.require_section(b"__TEXT", b"__eh_frame")?;
                    {
                        let ranges = eh_ranges
                            .as_ref()
                            .ok_or_else(|| CodegenError("missing EH records".into()))?;
                        let (start, end) = super::unwind::require(
                            ranges,
                            primary_section.index(),
                            primary.value(),
                        )?;
                        (section, section.address() + start, section.address() + end)
                    }
                }
                role => {
                    return Err(CodegenError(format!(
                        "unexpected generated-C Mach-O atom {role:?}"
                    )));
                }
            };
            for (symbol, value) in [(boundary.start(), start), (boundary.end(), end)] {
                additions.push(BoundaryDefinitionV1::new(
                    normalization
                        .compiler_generated_object_symbol(symbol.symbol().as_str())
                        .into_bytes(),
                    section.ordinal(),
                    value,
                    scoop_lir::LinkageClass::ConeStrong,
                ));
            }
        }
    }
    drop(file);
    layout.add_external_definitions(&mut bytes, additions)?;
    Ok(bytes)
}
