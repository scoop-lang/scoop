use object::{Object, ObjectSection};

use super::*;
use crate::coff_object::{CoffObject, error};

pub(super) fn materialize(
    bytes: &[u8],
    object_target: scoop_lir::ObjectTargetProfileV1,
    atoms: &[&DefinitionSymbolPlanV1],
) -> Result<Vec<u8>, CodegenError> {
    let mut output = CoffObject::read(bytes, object_target)?;
    let primary_ends = super::unwind::primary_ends(&output.file)?;
    let unwind = super::unwind::windows(&output.file)?;
    for definition in atoms {
        let name = definition.primary_symbol().symbol();
        let (section, start) = output.extent(name.as_str())?;
        let end = primary_ends[&(section.0, start)];
        let mut associated = Vec::new();
        for boundary in definition.atom_boundaries() {
            let (section, start, end) = match boundary.atom_role() {
                scoop_lir::DefinitionAtomRole::Primary => (section, start, end),
                scoop_lir::DefinitionAtomRole::WindowsUnwindTable
                | scoop_lir::DefinitionAtomRole::WindowsUnwindInfo => {
                    let records = unwind
                        .get(&section.0)
                        .ok_or_else(|| error("missing bridge unwind records"))?;
                    let index = if boundary.atom_role()
                        == scoop_lir::DefinitionAtomRole::WindowsUnwindTable
                    {
                        records.table
                    } else {
                        records.info
                    };
                    let unwind = output.file.section_by_index(index).map_err(error)?;
                    (index, 0, unwind.size())
                }
                role => return Err(error(format!("unexpected generated-C COFF atom {role:?}"))),
            };
            for (symbol, value) in [(boundary.start(), start), (boundary.end(), end)] {
                output.boundary(symbol.symbol().as_str(), section, value)?;
            }
            associated.push(section);
        }
        if associated.len() > 1 {
            output.associate(section, &associated, definition.primary_symbol().linkage())?;
        }
    }
    output.finish()
}
