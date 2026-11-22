//! Locate each generated function's native unwind records without rewriting them.

use std::collections::BTreeMap;

use object::read::elf::ElfFile64;
use object::{
    LittleEndian, Object, ObjectSection, ObjectSymbol, Relocation, RelocationEncoding,
    RelocationKind, RelocationTarget, SectionIndex, SectionKind,
};

use crate::CodegenError;

type Function = (usize, u64);
pub(super) type Ranges = BTreeMap<Function, (u64, u64)>;

fn error(message: impl std::fmt::Display) -> CodegenError {
    CodegenError::internal(format!("generated-C unwind: {message}"))
        .with_code(scoop_source::DiagnosticCode::ICE2302)
}

fn unsigned(data: &[u8], offset: usize, size: usize) -> Result<u64, CodegenError> {
    let bytes = data
        .get(offset..offset + size)
        .ok_or_else(|| error("truncated record"))?;
    let mut value = [0; 8];
    value[..size].copy_from_slice(bytes);
    Ok(u64::from_le_bytes(value))
}

pub(super) fn elf_function(
    file: &ElfFile64<'_, LittleEndian>,
    relocations: &[Relocation],
) -> Result<Function, CodegenError> {
    // The ELF reader decodes the relocation in the file's machine namespace.
    let [relocation] = relocations else {
        return Err(error("ELF FDE initial location requires one relocation"));
    };
    if relocation.kind() != RelocationKind::Relative
        || relocation.encoding() != RelocationEncoding::Generic
        || relocation.has_implicit_addend()
    {
        return Err(error(format!(
            "unsupported ELF FDE initial-location relocation {:?}",
            relocation.flags()
        )));
    }
    let RelocationTarget::Symbol(index) = relocation.target() else {
        return Err(error("ELF FDE initial location requires a symbol"));
    };
    let symbol = file.symbol_by_index(index).map_err(error)?;
    let section = symbol
        .section_index()
        .ok_or_else(|| error("undefined ELF FDE target"))?;
    if file.section_by_index(section).map_err(error)?.kind() != SectionKind::Text {
        return Err(error("ELF FDE target is outside text"));
    }
    // The unwinder adds the field address back to S + A - P.
    let address = symbol
        .address()
        .checked_add_signed(relocation.addend())
        .ok_or_else(|| error("ELF FDE target address overflows"))?;
    Ok((section.0, address))
}

/// Mach-O's local section relocations store object addresses.
pub(super) fn macho_function<'data, O: Object<'data>>(
    file: &O,
    section: &impl ObjectSection<'data>,
    offset: u64,
    relocations: &[Relocation],
) -> Result<Function, CodegenError> {
    for relocation in relocations {
        let (target, address, section_symbol) = match relocation.target() {
            RelocationTarget::Symbol(index) => {
                let symbol = file.symbol_by_index(index).map_err(error)?;
                let Some(target) = symbol.section_index() else {
                    continue;
                };
                (
                    target,
                    symbol.address(),
                    symbol.kind() == object::SymbolKind::Section,
                )
            }
            RelocationTarget::Section(target) => (target, 0, true),
            _ => continue,
        };
        if file.section_by_index(target).map_err(error)?.kind() != SectionKind::Text {
            continue;
        }
        let address = if relocation.has_implicit_addend() {
            if section_symbol {
                unsigned(section.data().map_err(error)?, offset as usize, 8)?
            } else {
                address
            }
        } else {
            address.wrapping_add_signed(relocation.addend())
        };
        return Ok((target.0, address));
    }
    // The x86-64 Mach-O assembler resolves its PC-relative FDE pointers locally.
    let value = unsigned(section.data().map_err(error)?, offset as usize, 8)?;
    let address = section.address().wrapping_add(offset).wrapping_add(value);
    if let Some(text) = file.sections().find(|text| {
        text.kind() == SectionKind::Text
            && address >= text.address()
            && address < text.address() + text.size()
    }) {
        return Ok((text.index().0, address));
    }
    Err(error("function has no resolvable unwind initial location"))
}

fn relocation_sites<'data>(section: &impl ObjectSection<'data>) -> BTreeMap<u64, Vec<Relocation>> {
    let mut sites = BTreeMap::<_, Vec<_>>::new();
    for (offset, relocation) in section.relocations() {
        sites.entry(offset).or_default().push(relocation);
    }
    sites
}

pub(super) fn eh_frame<'data>(
    section: &impl ObjectSection<'data>,
    mut function: impl FnMut(u64, &[Relocation]) -> Result<Function, CodegenError>,
) -> Result<Ranges, CodegenError> {
    let data = section.data().map_err(error)?;
    let sites = relocation_sites(section);
    let mut ranges = Ranges::new();
    let mut offset = 0;
    let mut owned_start = 0;
    let mut last = None;
    while offset < data.len() {
        let length = unsigned(data, offset, 4)?;
        if length == 0 {
            break;
        }
        let (length, header, id_size) = if length == u64::from(u32::MAX) {
            (unsigned(data, offset + 4, 8)?, 12, 8)
        } else {
            (length, 4, 4)
        };
        let end = offset
            .checked_add(header)
            .and_then(|start| start.checked_add(length as usize))
            .filter(|end| *end <= data.len())
            .ok_or_else(|| error("invalid EH record length"))?;
        if length < id_size as u64 {
            return Err(error("truncated EH record identifier"));
        }
        if unsigned(data, offset + header, id_size)? != 0 {
            let site = (offset + header + id_size) as u64;
            let key = function(site, sites.get(&site).map(Vec::as_slice).unwrap_or(&[]))?;
            if ranges
                .insert(key, (owned_start as u64, end as u64))
                .is_some()
            {
                return Err(error("multiple disjoint FDEs for one generated bridge"));
            }
            owned_start = end;
            last = Some(key);
        }
        offset = end;
    }
    if let Some(key) = last {
        ranges.get_mut(&key).expect("last FDE was inserted").1 = data.len() as u64;
    }
    Ok(ranges)
}

pub(super) fn macho_compact<'data, O: Object<'data>>(
    file: &O,
    section: &impl ObjectSection<'data>,
) -> Result<Ranges, CodegenError> {
    if !section.size().is_multiple_of(32) {
        return Err(error("invalid 64-bit compact-unwind record length"));
    }
    let sites = relocation_sites(section);
    (0..section.size())
        .step_by(32)
        .map(|offset| {
            Ok((
                macho_function(
                    file,
                    section,
                    offset,
                    sites.get(&offset).map(Vec::as_slice).unwrap_or(&[]),
                )?,
                (offset, offset + 32),
            ))
        })
        .collect()
}

pub(super) fn require(
    ranges: &Ranges,
    section: SectionIndex,
    address: u64,
) -> Result<(u64, u64), CodegenError> {
    ranges.get(&(section.0, address)).copied().ok_or_else(|| {
        error(format!(
            "no record for function in section {} at {address}",
            section.0
        ))
    })
}

pub(super) struct WindowsUnwind {
    pub table: SectionIndex,
    pub info: SectionIndex,
}

pub(super) fn windows<'data, O: Object<'data>>(
    file: &O,
) -> Result<BTreeMap<usize, WindowsUnwind>, CodegenError> {
    let mut output = BTreeMap::new();
    for table in file.sections().filter(|section| {
        section
            .name()
            .is_ok_and(|name| name == ".pdata" || name.starts_with(".pdata$"))
    }) {
        if table.size() != 12 {
            return Err(error(
                "generated bridge needs one runtime-function record per section",
            ));
        }
        let target = |offset| -> Result<SectionIndex, CodegenError> {
            let relocation = table
                .relocations()
                .find(|(site, _)| *site == offset)
                .ok_or_else(|| error("missing Windows unwind relocation"))?
                .1;
            match relocation.target() {
                RelocationTarget::Section(index) => Ok(index),
                RelocationTarget::Symbol(index) => file
                    .symbol_by_index(index)
                    .map_err(error)?
                    .section_index()
                    .ok_or_else(|| error("undefined Windows unwind target")),
                _ => Err(error("invalid Windows unwind target")),
            }
        };
        output.insert(
            target(0)?.0,
            WindowsUnwind {
                table: table.index(),
                info: target(8)?,
            },
        );
    }
    Ok(output)
}

/// Physical extents of the compiler's global definitions, including alignment
/// padding before the next definition in the same section.
pub(super) fn primary_ends<'data, O: Object<'data>>(
    file: &O,
) -> Result<BTreeMap<Function, u64>, CodegenError> {
    let mut sections = BTreeMap::<usize, Vec<u64>>::new();
    for symbol in file.symbols().filter(|symbol| symbol.is_global()) {
        if let Some(section) = symbol.section_index() {
            sections
                .entry(section.0)
                .or_default()
                .push(symbol.address());
        }
    }
    let mut ends = BTreeMap::new();
    for (index, mut starts) in sections {
        starts.sort_unstable();
        starts.dedup();
        let section = file.section_by_index(SectionIndex(index)).map_err(error)?;
        let section_end = section.address() + section.size();
        for (start, end) in starts.iter().copied().zip(
            starts
                .iter()
                .copied()
                .skip(1)
                .chain(std::iter::once(section_end)),
        ) {
            ends.insert((index, start), end);
        }
    }
    Ok(ends)
}
