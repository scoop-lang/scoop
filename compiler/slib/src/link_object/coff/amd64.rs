use object::pe::*;

use super::CoffRelocationKind::{
    self, Absolute64, NoOp, Other, SectionRelative7, SectionRelative32,
};

pub(super) fn relocation_kind(kind: u16) -> Option<CoffRelocationKind> {
    Some(match kind {
        IMAGE_REL_AMD64_ABSOLUTE => NoOp,
        IMAGE_REL_AMD64_ADDR64 => Absolute64,
        IMAGE_REL_AMD64_SECREL7 => SectionRelative7,
        IMAGE_REL_AMD64_SECREL => SectionRelative32,
        IMAGE_REL_AMD64_ADDR32
        | IMAGE_REL_AMD64_ADDR32NB
        | IMAGE_REL_AMD64_REL32
        | IMAGE_REL_AMD64_REL32_1
        | IMAGE_REL_AMD64_REL32_2
        | IMAGE_REL_AMD64_REL32_3
        | IMAGE_REL_AMD64_REL32_4
        | IMAGE_REL_AMD64_REL32_5
        | IMAGE_REL_AMD64_SSPAN32 => Other(4),
        IMAGE_REL_AMD64_SECTION => Other(2),
        _ => return None,
    })
}
