//! Replace the temporary LLVM stackmap of one callable with its runtime GC map.
//!
//! Each callable is already an independent object member. Repacking its existing
//! sections preserves section/symbol indices, COMDAT associations and EH data.

use std::fmt;

use super::{CanonicalStackmapLocationV1, ScoopStackmapRecordV1};

pub(in crate::link_object) mod codec;
mod coff;
mod elf;
mod macho;

#[derive(Debug)]
pub struct CompactGcMapError(String);

impl fmt::Display for CompactGcMapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "cannot produce compact callable GC map: {}", self.0)
    }
}

impl std::error::Error for CompactGcMapError {}

pub fn compact_callable_gc_object_v1(
    target: scoop_lir::LirTargetProfile,
    bytes: &[u8],
    records: &[&ScoopStackmapRecordV1],
) -> Result<Vec<u8>, CompactGcMapError> {
    let frame = scoop_target_data::selection(target.id())
        .ok_or_else(|| error("unknown GC frame profile"))?
        .backend()
        .contract()
        .machine_frame_recovery;
    let map = codec::encode(frame, records.iter().map(|record| record.normalized()))?;
    match object::FileKind::parse(bytes).map_err(error)? {
        object::FileKind::MachO64 => macho::replace(bytes, &map),
        object::FileKind::Elf64 => elf::replace(bytes, &map),
        object::FileKind::Coff | object::FileKind::CoffBig => coff::replace(bytes, &map),
        kind => Err(error(format!("unexpected callable object format {kind:?}"))),
    }
}

fn error(message: impl fmt::Display) -> CompactGcMapError {
    CompactGcMapError(message.to_string())
}

fn field<const N: usize>(bytes: &[u8], offset: usize) -> Result<[u8; N], CompactGcMapError> {
    bytes
        .get(
            offset
                ..offset
                    .checked_add(N)
                    .ok_or_else(|| error("field offset overflows"))?,
        )
        .and_then(|value| value.try_into().ok())
        .ok_or_else(|| error("truncated object field"))
}

fn u16_at(bytes: &[u8], offset: usize) -> Result<u16, CompactGcMapError> {
    field(bytes, offset).map(u16::from_le_bytes)
}

fn u32_at(bytes: &[u8], offset: usize) -> Result<u32, CompactGcMapError> {
    field(bytes, offset).map(u32::from_le_bytes)
}

fn u64_at(bytes: &[u8], offset: usize) -> Result<u64, CompactGcMapError> {
    field(bytes, offset).map(u64::from_le_bytes)
}

fn put<const N: usize>(
    bytes: &mut [u8],
    offset: usize,
    value: [u8; N],
) -> Result<(), CompactGcMapError> {
    bytes
        .get_mut(
            offset
                ..offset
                    .checked_add(N)
                    .ok_or_else(|| error("field offset overflows"))?,
        )
        .ok_or_else(|| error("truncated object field"))?
        .copy_from_slice(&value);
    Ok(())
}

fn span(bytes: &[u8], offset: u64, length: u64) -> Result<&[u8], CompactGcMapError> {
    let start = usize::try_from(offset).map_err(error)?;
    let length = usize::try_from(length).map_err(error)?;
    bytes
        .get(
            start
                ..start
                    .checked_add(length)
                    .ok_or_else(|| error("object span overflows"))?,
        )
        .ok_or_else(|| error("truncated object span"))
}

fn append(bytes: &mut Vec<u8>, data: &[u8], alignment: u64) -> Result<u64, CompactGcMapError> {
    let alignment = usize::try_from(alignment.max(1)).map_err(error)?;
    if !alignment.is_power_of_two() {
        return Err(error("object alignment is not a power of two"));
    }
    let start = bytes
        .len()
        .checked_add(alignment - 1)
        .ok_or_else(|| error("object alignment overflows"))?
        & !(alignment - 1);
    let end = start
        .checked_add(data.len())
        .ok_or_else(|| error("object size overflows"))?;
    bytes.resize(end, 0);
    bytes[start..end].copy_from_slice(data);
    Ok(start as u64)
}

fn boundary_value(
    value: u64,
    start: u64,
    old_size: u64,
    new_size: usize,
) -> Result<u64, CompactGcMapError> {
    if value == start {
        Ok(start)
    } else if start.checked_add(old_size) == Some(value) {
        start
            .checked_add(new_size as u64)
            .ok_or_else(|| error("GC boundary overflows"))
    } else {
        Err(error("LLVM stackmap has an interior symbol"))
    }
}
