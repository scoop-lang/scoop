//! Deterministic Mach-O symbol-table rebuild for callable atom boundaries.

use std::collections::BTreeMap;

use object::macho;

use super::{BoundaryDefinitionV1, MachOLayout, NLIST_64_SIZE, malformed, read_u32, write_u32};
use crate::CodegenError;

impl MachOLayout {
    pub(crate) fn materialize_odr_definition(
        &self,
        bytes: &mut [u8],
        name: &[u8],
    ) -> Result<(), CodegenError> {
        let symbol = self.require_external_definition(name)?;
        let index = self
            .symbols
            .iter()
            .position(|candidate| candidate.name == name)
            .ok_or_else(malformed)?;
        let offset = usize::try_from(self.symtab.symbol_offset)
            .ok()
            .and_then(|start| {
                index
                    .checked_mul(NLIST_64_SIZE)
                    .and_then(|relative| start.checked_add(relative))
            })
            .and_then(|start| start.checked_add(6))
            .ok_or_else(malformed)?;
        let flags = symbol.description | macho::N_WEAK_DEF;
        bytes
            .get_mut(offset..offset.checked_add(2).ok_or_else(malformed)?)
            .ok_or_else(malformed)?
            .copy_from_slice(&flags.to_le_bytes());
        Ok(())
    }

    pub(crate) fn add_external_definitions(
        &self,
        bytes: &mut Vec<u8>,
        mut additions: Vec<BoundaryDefinitionV1>,
    ) -> Result<(), CodegenError> {
        validate_additions(self, &mut additions)?;
        let symbol_start = self.symtab.symbol_offset as usize;
        let old_symbol_end = symbol_start + self.symtab.symbol_count as usize * NLIST_64_SIZE;
        let old_symbols = bytes
            .get(symbol_start..old_symbol_end)
            .ok_or_else(malformed)?
            .to_vec();
        let mut strings = bytes
            .get(self.symtab.string_offset as usize..)
            .ok_or_else(malformed)?
            .to_vec();
        let encoded = encode_additions(&additions, &mut strings)?;
        let undefined_start = self.dysymtab.undefined_index as usize;
        let additions_by_name = additions
            .iter()
            .enumerate()
            .map(|(index, addition)| (addition.name.as_slice(), index))
            .collect::<BTreeMap<_, _>>();
        let mut remap = (0..self.symtab.symbol_count).collect::<Vec<_>>();
        let mut remaining = Vec::new();
        for (index, symbol) in self.symbols.iter().enumerate().skip(undefined_start) {
            let target = if let Some(addition) = additions_by_name.get(symbol.name.as_slice()) {
                undefined_start + addition
            } else {
                let target = undefined_start + additions.len() + remaining.len();
                remaining.push(index);
                target
            };
            remap[index] = u32::try_from(target).map_err(|_| malformed())?;
        }
        self.remap_relocation_indices(bytes, &remap)?;
        let new_external_count = self
            .dysymtab
            .external_definition_count
            .checked_add(u32::try_from(additions.len()).map_err(|_| malformed())?)
            .ok_or_else(malformed)?;
        let new_undefined_index = self
            .dysymtab
            .local_count
            .checked_add(new_external_count)
            .ok_or_else(malformed)?;
        let new_undefined_count = u32::try_from(remaining.len()).map_err(|_| malformed())?;
        let new_symbol_count = new_undefined_index
            .checked_add(new_undefined_count)
            .ok_or_else(malformed)?;
        bytes.truncate(symbol_start);
        bytes.extend_from_slice(&old_symbols[..undefined_start * NLIST_64_SIZE]);
        bytes.extend_from_slice(&encoded);
        for index in remaining {
            bytes.extend_from_slice(
                &old_symbols[index * NLIST_64_SIZE..(index + 1) * NLIST_64_SIZE],
            );
        }
        let new_string_offset = u32::try_from(bytes.len())
            .map_err(|_| CodegenError("callable object exceeds Mach-O u32 offsets".to_string()))?;
        bytes.extend_from_slice(&strings);
        let new_string_size = u32::try_from(strings.len())
            .map_err(|_| CodegenError("callable string table exceeds u32".to_string()))?;

        write_u32(bytes, self.symtab.command_offset + 12, new_symbol_count)?;
        write_u32(bytes, self.symtab.command_offset + 16, new_string_offset)?;
        write_u32(bytes, self.symtab.command_offset + 20, new_string_size)?;
        write_u32(bytes, self.dysymtab.command_offset + 20, new_external_count)?;
        write_u32(
            bytes,
            self.dysymtab.command_offset + 24,
            new_undefined_index,
        )?;
        write_u32(
            bytes,
            self.dysymtab.command_offset + 28,
            new_undefined_count,
        )?;
        Ok(())
    }

    fn remap_relocation_indices(
        &self,
        bytes: &mut [u8],
        remap: &[u32],
    ) -> Result<(), CodegenError> {
        for section in &self.sections {
            for index in 0..section.relocation_count {
                let offset = usize::try_from(section.relocation_offset)
                    .ok()
                    .and_then(|start| {
                        usize::try_from(index)
                            .ok()
                            .and_then(|index| index.checked_mul(8))
                            .and_then(|relative| start.checked_add(relative))
                    })
                    .ok_or_else(malformed)?;
                let address_word = read_u32(bytes, offset)?;
                if address_word & macho::R_SCATTERED != 0 {
                    return Err(CodegenError(
                        "callable object uses a scattered relocation".to_string(),
                    ));
                }
                let info = read_u32(bytes, offset + 4)?;
                if (info >> 27) & 1 == 0 {
                    continue;
                }
                let target = info & 0x00ff_ffff;
                if target >= self.symtab.symbol_count {
                    return Err(CodegenError(
                        "callable relocation symbol index is out of bounds".to_string(),
                    ));
                }
                let shifted = remap[target as usize];
                if shifted > 0x00ff_ffff {
                    return Err(CodegenError(
                        "callable relocation symbol index overflows".into(),
                    ));
                }
                write_u32(bytes, offset + 4, (info & 0xff00_0000) | shifted)?;
            }
        }
        Ok(())
    }
}

fn validate_additions(
    layout: &MachOLayout,
    additions: &mut [BoundaryDefinitionV1],
) -> Result<(), CodegenError> {
    if additions.is_empty() {
        return Err(CodegenError(
            "callable object has no backend boundary symbols to materialize".to_string(),
        ));
    }
    additions.sort_unstable_by(|left, right| left.name.cmp(&right.name));
    if additions
        .windows(2)
        .any(|pair| pair[0].name == pair[1].name)
    {
        return Err(CodegenError(
            "callable boundary plan repeats a Mach-O symbol".to_string(),
        ));
    }
    for addition in additions.iter() {
        for (index, symbol) in layout
            .symbols
            .iter()
            .enumerate()
            .filter(|(_, symbol)| symbol.name == addition.name)
        {
            if index < layout.dysymtab.undefined_index as usize
                || symbol.symbol_type & (macho::N_STAB | macho::N_TYPE | macho::N_EXT)
                    != (macho::N_UNDF | macho::N_EXT)
                || symbol.section_ordinal != 0
                || symbol.value != 0
            {
                return Err(CodegenError(format!(
                    "callable boundary symbol `{}` is already defined",
                    String::from_utf8_lossy(&addition.name)
                )));
            }
        }
    }
    if additions.iter().any(|addition| {
        addition.name.is_empty()
            || addition.name.contains(&0)
            || addition.section_ordinal == 0
            || usize::from(addition.section_ordinal) > layout.sections.len()
    }) {
        return Err(CodegenError(
            "callable boundary plan contains an invalid Mach-O definition".to_string(),
        ));
    }
    Ok(())
}

fn encode_additions(
    additions: &[BoundaryDefinitionV1],
    strings: &mut Vec<u8>,
) -> Result<Vec<u8>, CodegenError> {
    let capacity = additions
        .len()
        .checked_mul(NLIST_64_SIZE)
        .ok_or_else(|| CodegenError("callable symbol table size overflows".to_string()))?;
    let mut encoded = Vec::with_capacity(capacity);
    for addition in additions {
        let string_index = u32::try_from(strings.len())
            .map_err(|_| CodegenError("callable boundary string table exceeds u32".to_string()))?;
        strings.extend_from_slice(&addition.name);
        strings.push(0);
        encoded.extend_from_slice(&string_index.to_le_bytes());
        encoded.push(macho::N_SECT | macho::N_EXT);
        encoded.push(addition.section_ordinal);
        let flags = if addition.linkage == scoop_lir::LinkageClass::OdrWeak {
            macho::N_WEAK_DEF
        } else {
            0
        };
        encoded.extend_from_slice(&flags.to_le_bytes());
        encoded.extend_from_slice(&addition.value.to_le_bytes());
    }
    Ok(encoded)
}
