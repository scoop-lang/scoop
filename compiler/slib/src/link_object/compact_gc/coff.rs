use object::{Object, ObjectSection, pe};

use super::*;

pub(super) fn replace(input: &[u8], map: &[u8]) -> Result<Vec<u8>, CompactGcMapError> {
    let big = matches!(
        object::FileKind::parse(input).map_err(error)?,
        object::FileKind::CoffBig
    );
    let (header_size, symbol_size, table_field, count) = if big {
        (56, 20, 48, u32_at(input, 44)? as usize)
    } else {
        (20, 18, 8, u16_at(input, 2)? as usize)
    };
    let file = object::File::parse(input).map_err(error)?;
    let gc = file
        .section_by_name(".llvm_stackmaps")
        .ok_or_else(|| error("missing temporary LLVM stackmap section"))?;
    let gc = gc.index().0 - 1;
    let mut output = span(input, 0, (header_size + count * 40) as u64)?.to_vec();
    let gc_header = header_size + gc * 40;
    let old_size = u32_at(input, gc_header + 16)? as u64;
    let table = u32_at(input, table_field)? as usize;
    let symbol_count = u32_at(input, table_field + 4)? as usize;
    let mut symbols = span(input, table as u64, (symbol_count * symbol_size) as u64)?.to_vec();
    let string_start = table + symbol_count * symbol_size;
    let mut strings = span(
        input,
        string_start as u64,
        u32_at(input, string_start)? as u64,
    )?
    .to_vec();
    let name_offset = u32::try_from(strings.len()).map_err(error)?;
    strings.extend_from_slice(b".rdata$sgc\0");
    let name = format!("/{name_offset}");
    if name.len() > 8 {
        return Err(error("COFF GC section name offset exceeds its header"));
    }
    output[gc_header..gc_header + 8].fill(0);
    output[gc_header..gc_header + name.len()].copy_from_slice(name.as_bytes());
    for index in 0..count {
        let header = header_size + index * 40;
        let mut flags = u32_at(input, header + 36)?;
        let size = u32_at(input, header + 16)?;
        if flags & pe::IMAGE_SCN_CNT_UNINITIALIZED_DATA == 0 && size != 0 {
            let data = if index == gc {
                map
            } else {
                span(input, u32_at(input, header + 20)? as u64, size as u64)?
            };
            let alignment = ((flags & pe::IMAGE_SCN_ALIGN_MASK) >> 20).saturating_sub(1);
            let offset = append(&mut output, data, 1u64 << alignment)?;
            put(
                &mut output,
                header + 20,
                u32::try_from(offset).map_err(error)?.to_le_bytes(),
            )?;
            put(
                &mut output,
                header + 16,
                u32::try_from(data.len()).map_err(error)?.to_le_bytes(),
            )?;
        }
        let relocations = u16_at(input, header + 32)?;
        if index == gc {
            put(&mut output, header + 24, 0u32.to_le_bytes())?;
            put(&mut output, header + 32, 0u16.to_le_bytes())?;
            flags &= !(pe::IMAGE_SCN_LNK_NRELOC_OVFL | pe::IMAGE_SCN_MEM_WRITE);
            flags |= pe::IMAGE_SCN_MEM_READ;
            put(&mut output, header + 36, flags.to_le_bytes())?;
        } else if relocations != 0 {
            let offset = u32_at(input, header + 24)? as u64;
            let count = if flags & pe::IMAGE_SCN_LNK_NRELOC_OVFL != 0 {
                u32_at(input, offset as usize)? as u64
            } else {
                relocations as u64
            };
            let offset = append(&mut output, span(input, offset, count * 10)?, 4)?;
            put(
                &mut output,
                header + 24,
                u32::try_from(offset).map_err(error)?.to_le_bytes(),
            )?;
        }
    }
    let mut index = 0;
    while index < symbol_count {
        let offset = index * symbol_size;
        let section = if big {
            u32_at(&symbols, offset + 12)?
        } else {
            u16_at(&symbols, offset + 12)? as u32
        };
        let auxiliaries = symbols[offset + symbol_size - 1] as usize;
        if section as usize == gc + 1 {
            let value =
                boundary_value(u32_at(&symbols, offset + 8)? as u64, 0, old_size, map.len())?;
            put(
                &mut symbols,
                offset + 8,
                u32::try_from(value).map_err(error)?.to_le_bytes(),
            )?;
            if symbols[offset + symbol_size - 2] == pe::IMAGE_SYM_CLASS_STATIC && auxiliaries != 0 {
                // Keep the section number and associative COMDAT owner unchanged.
                put(&mut symbols, offset, 0u32.to_le_bytes())?;
                put(&mut symbols, offset + 4, name_offset.to_le_bytes())?;
                put(
                    &mut symbols,
                    offset + symbol_size,
                    u32::try_from(map.len()).map_err(error)?.to_le_bytes(),
                )?;
                put(&mut symbols, offset + symbol_size + 4, 0u16.to_le_bytes())?;
            }
        }
        index += 1 + auxiliaries;
    }
    let offset = append(&mut output, &symbols, 4)?;
    put(
        &mut output,
        table_field,
        u32::try_from(offset).map_err(error)?.to_le_bytes(),
    )?;
    let size = u32::try_from(strings.len()).map_err(error)?;
    put(&mut strings, 0, size.to_le_bytes())?;
    append(&mut output, &strings, 1)?;
    Ok(output)
}
