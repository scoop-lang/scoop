//! COFF bounds, symbols and relocations, including the bigobj container.

use std::num::NonZeroU32;

use object::read::coff::{CoffBigFile, CoffFile, CoffHeader};
use object::{Architecture, FileKind, Object, ObjectKind, ObjectSection, SectionIndex};
use scoop_identity::TargetProfileId;
use scoop_lir::{ObjectMachineV1, ObjectRelocationProfileV1, ObjectTargetProfileV1};

use super::object_envelope::*;

mod amd64;
pub(crate) mod builtin;
mod relocations;
mod symbols;
#[cfg(all(test, target_os = "windows"))]
mod tests;

#[derive(Clone, Copy)]
pub(super) enum CoffRelocationKind {
    NoOp,
    Absolute64,
    SectionRelative32,
    SectionRelative7,
    Other(u8),
}

impl CoffRelocationKind {
    fn decode(machine: ObjectMachineV1, kind: u16) -> Option<Self> {
        match machine {
            ObjectMachineV1::X86_64 => amd64::relocation_kind(kind),
            ObjectMachineV1::Aarch64 => None,
        }
    }

    pub(super) fn classify(machine: ObjectMachineV1, kind: u16, width: u8) -> Option<Self> {
        let kind = Self::decode(machine, kind)?;
        (kind.storage_width() == Some(width)).then_some(kind)
    }

    const fn storage_width(self) -> Option<u8> {
        match self {
            Self::NoOp => None,
            Self::Absolute64 => Some(8),
            Self::SectionRelative32 => Some(4),
            Self::SectionRelative7 => Some(1),
            Self::Other(width) => Some(width),
        }
    }
}

pub fn validate_coff_object_envelope_v1(
    bytes: &[u8],
    target: TargetProfileId,
) -> Result<ValidatedObjectEnvelopeV1, CoffObjectError> {
    let profile =
        scoop_target_data::layout(target).ok_or_else(|| error("unsupported COFF target"))?;
    let object_target = profile.object_target();
    if object_target.relocation_profile() != ObjectRelocationProfileV1::CoffAmd64 {
        return Err(error(
            "unsupported COFF relocation profile for the selected target",
        ));
    }
    match FileKind::parse(bytes).map_err(error)? {
        FileKind::Coff => envelope(
            bytes,
            &CoffFile::<&[u8]>::parse(bytes).map_err(error)?,
            target,
            object_target,
        ),
        FileKind::CoffBig => envelope(
            bytes,
            &CoffBigFile::parse(bytes).map_err(error)?,
            target,
            object_target,
        ),
        _ => Err(error("expected a COFF relocatable object or bigobj")),
    }
}

fn envelope<'data, H: CoffHeader>(
    bytes: &[u8],
    file: &CoffFile<'data, &'data [u8], H>,
    target: TargetProfileId,
    object_target: ObjectTargetProfileV1,
) -> Result<ValidatedObjectEnvelopeV1, CoffObjectError> {
    let machine = object_target.machine();
    let architecture = match machine {
        ObjectMachineV1::Aarch64 => Architecture::Aarch64,
        ObjectMachineV1::X86_64 => Architecture::X86_64,
    };
    if file.architecture() != architecture || file.kind() != ObjectKind::Relocatable {
        return Err(error(
            "expected a COFF relocatable object for the selected machine",
        ));
    }
    let mut sections = Vec::new();
    for section in file.sections() {
        // ObjectSection::relocations suppresses malformed-table errors. Read
        // the raw table in relocations::read instead, preserving I/O failures.
        section.data().map_err(error)?;
        sections.push(ObservedObjectSectionV1 {
            segment_name: Vec::new(),
            section_name: section.name_bytes().map_err(error)?.to_vec(),
            virtual_address: section.address(),
            file_offset: section.file_range().map(|range| range.0),
            flags: ObjectSectionFlagsV1::Coff {
                characteristics: section
                    .coff_section()
                    .characteristics
                    .get(object::LittleEndian),
            },
            byte_size: section.size(),
            alignment_power: section.align().max(1).trailing_zeros(),
        });
    }
    let symbols = symbols::read(file, &sections)?;
    let relocations = relocations::read(file, &symbols, machine)?;
    Ok(ValidatedObjectEnvelopeV1 {
        byte_length: bytes.len() as u64,
        content_digest: scoop_wire::sha256(bytes),
        format: ObjectEnvelopeFormatV1::new(target, object_target),
        sections,
        symbols,
        relocations,
    })
}

fn ordinal(index: SectionIndex) -> Result<NonZeroU32, CoffObjectError> {
    NonZeroU32::new(u32::try_from(index.0).map_err(error)?)
        .ok_or_else(|| error("zero COFF section number"))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoffObjectError(String);

impl std::fmt::Display for CoffObjectError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "COFF object: {}", self.0)
    }
}
impl std::error::Error for CoffObjectError {}

fn error(message: impl std::fmt::Display) -> CoffObjectError {
    CoffObjectError(message.to_string())
}
