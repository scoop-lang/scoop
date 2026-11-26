//! AArch64 LP64 RELA field extents in relocatable ELF objects (AAELF64).
use super::ElfRelocationKind;
use object::elf::*;

pub(super) fn relocation_kind(kind: u32) -> Option<ElfRelocationKind> {
    use ElfRelocationKind::{Absolute64, Other, TlsReference};
    Some(match kind {
        R_AARCH64_NONE => Other(0),
        R_AARCH64_ABS64 => Absolute64,
        R_AARCH64_PREL64 | R_AARCH64_GOTREL64 => Other(8),
        R_AARCH64_ABS32 | R_AARCH64_PREL32 | R_AARCH64_GOTREL32 => Other(4),
        R_AARCH64_ABS16 | R_AARCH64_PREL16 => Other(2),
        // Every relocation in these blocks applies to one A64 instruction.
        R_AARCH64_MOVW_UABS_G0..=R_AARCH64_CONDBR19
        | R_AARCH64_JUMP26..=R_AARCH64_MOVW_PREL_G3
        | R_AARCH64_LDST128_ABS_LO12_NC..=R_AARCH64_MOVW_GOTOFF_G3
        | R_AARCH64_GOT_LD_PREL19..=R_AARCH64_LD64_GOTPAGE_LO15 => Other(4),
        R_AARCH64_TLSGD_ADR_PREL21..=R_AARCH64_TLSLD_LDST128_DTPREL_LO12_NC => TlsReference(4),
        _ => return None,
    })
}
