//! Linker options from selected Mach-O members, not every archive member.
use object::{macho, read::macho::MachOFile64};
use scoop_identity::{CanonicalNativeLibraryName, NativeLibraryGrouping, NativeLibraryKind};

use super::*;

pub(crate) fn read(
    bytes: &[u8],
    profile: &ValidatedFinalLinkProfile,
) -> Result<Vec<NativeLinkRequirementKey>, LinkError> {
    if !matches!(profile, ValidatedFinalLinkProfile::Darwin(_)) {
        return Ok(Vec::new());
    }
    let malformed = |cause| error(scoop_source::DiagnosticCode::E2305, cause);
    let file: MachOFile64<'_> = MachOFile64::parse(bytes).map_err(|e| malformed(e.to_string()))?;
    let mut commands = file
        .macho_load_commands()
        .map_err(|e| malformed(e.to_string()))?;
    let mut result = Vec::new();
    while let Some(command) = commands.next().map_err(|e| malformed(e.to_string()))? {
        if command.cmd() != macho::LC_LINKER_OPTION {
            continue;
        }
        let data = command.raw_data();
        let count = data
            .get(8..12)
            .ok_or_else(|| malformed("invalid LC_LINKER_OPTION header".into()))?;
        let count = u32::from_le_bytes(count.try_into().expect("four-byte option count"));
        let mut remaining = &data[12..];
        let mut args = Vec::new();
        for _ in 0..count {
            let end = remaining
                .iter()
                .position(|b| *b == 0)
                .ok_or_else(|| malformed("unterminated LC_LINKER_OPTION argument".into()))?;
            args.push(
                std::str::from_utf8(&remaining[..end]).map_err(|e| malformed(e.to_string()))?,
            );
            remaining = &remaining[end + 1..];
        }
        let (name, kind) = match args.as_slice() {
            [flag] if flag.starts_with("-l") && flag.len() > 2 => {
                (&flag[2..], NativeLibraryKind::TargetDefault)
            }
            ["-framework", name] => (*name, NativeLibraryKind::Framework),
            _ => return Err(malformed(format!("unsupported LC_LINKER_OPTION {args:?}"))),
        };
        result.push(NativeLinkRequirementKey::for_target(
            profile.target().wire_id(),
            CanonicalNativeLibraryName::new(name).map_err(|e| malformed(e.to_string()))?,
            kind,
            NativeLibraryGrouping::Independent,
        ));
    }
    Ok(result)
}
