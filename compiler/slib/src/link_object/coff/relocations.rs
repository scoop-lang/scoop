use super::*;

pub(super) fn read<'data, H: CoffHeader>(
    file: &CoffFile<'data, &'data [u8], H>,
    symbols: &[ObservedObjectSymbolV1],
    machine: ObjectMachineV1,
) -> Result<Vec<ObservedObjectRelocationV1>, CoffObjectError> {
    let mut relocations = Vec::new();
    for section in file.sections() {
        let data = section.data().map_err(error)?;
        for raw in section.coff_relocations().map_err(error)? {
            let kind = raw.typ.get(object::LittleEndian);
            let decoded = CoffRelocationKind::decode(machine, kind).ok_or_else(|| {
                error(format!(
                    "unsupported {machine:?} relocation kind 0x{kind:04x}"
                ))
            })?;
            let Some(width) = decoded.storage_width() else {
                continue;
            };
            let target_symbol = raw.symbol_table_index.get(object::LittleEndian);
            symbols::require_symbol(symbols, target_symbol)?;
            let offset = raw.virtual_address.get(object::LittleEndian) as usize;
            let end = offset
                .checked_add(usize::from(width))
                .ok_or_else(|| error("COFF relocation extent overflow"))?;
            let field = data
                .get(offset..end)
                .ok_or_else(|| error("COFF relocation field outside section"))?;
            let mut encoded = [0; 8];
            encoded[..usize::from(width)].copy_from_slice(field);
            relocations.push(ObservedObjectRelocationV1 {
                containing_section_ordinal: ordinal(section.index())?,
                offset: offset as u64,
                encoded_value: u64::from_le_bytes(encoded),
                shape: ObjectRelocationShapeV1::Coff {
                    machine,
                    kind,
                    target_symbol,
                    storage_width: width,
                },
            });
        }
    }
    Ok(relocations)
}
