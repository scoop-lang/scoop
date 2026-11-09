//! Native archive format and buffered writing shared by runtime and final linking.
use std::borrow::Cow;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use ar_archive_writer::{ArchiveKind, NewArchiveMember};
use scoop_lir::{LirTargetProfile, NativeObjectFormat};

use crate::{LinkError, error};

#[cfg(test)]
mod tests;

pub(crate) fn prepare_object(
    target: LirTargetProfile,
    bytes: Cow<'_, [u8]>,
) -> Result<Cow<'_, [u8]>, LinkError> {
    if target.native_object_format() == NativeObjectFormat::MachO64 {
        Ok(Cow::Owned(
            scoop_slib::remove_empty_macho_code_labels(&bytes).map_err(error)?,
        ))
    } else {
        Ok(bytes)
    }
}

pub(crate) fn format(target: LirTargetProfile) -> (ArchiveKind, &'static str) {
    match target.native_object_format() {
        NativeObjectFormat::MachO64 => (ArchiveKind::Darwin, "a"),
        NativeObjectFormat::Elf64 => (ArchiveKind::Gnu, "a"),
        NativeObjectFormat::Coff64 => (ArchiveKind::Coff, "lib"),
    }
}

pub(crate) fn write(
    path: &Path,
    kind: ArchiveKind,
    members: &[NewArchiveMember<'_>],
) -> Result<(), LinkError> {
    let mut writer = BufWriter::new(File::create(path).map_err(error)?);
    ar_archive_writer::write_archive_to_stream(&mut writer, members, kind, false, None)
        .map_err(error)?;
    writer.flush().map_err(error)
}
