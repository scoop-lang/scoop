//! Metadata pointer semantics without losing the native relocation form.

use super::{
    VerifiedMachORelocationFormV1 as MachForm, VerifiedMachORelocationShapeV1 as MachShape,
    VerifiedObjectRelocationFormV1 as Form, VerifiedObjectRelocationShapeV1 as Shape,
};

impl Form {
    pub fn is_absolute64(self) -> bool {
        matches!(
            self,
            Self::MachO {
                form: MachForm::Unsigned64,
                ..
            } | Self::Coff {
                kind: object::pe::IMAGE_REL_AMD64_ADDR64,
                storage_width: 8,
                ..
            } | Self::ElfRela {
                kind: object::elf::R_X86_64_64,
                storage_width: 8,
                ..
            }
        )
    }

    /// COFF and Mach-O store the addend in-place; ELF RELA stores it separately.
    pub fn absolute64_addend(self, encoded_field: u64) -> Option<i128> {
        match self {
            Self::MachO {
                form: MachForm::Unsigned64,
                ..
            } => Some(i128::from(encoded_field)),
            Self::Coff {
                kind: object::pe::IMAGE_REL_AMD64_ADDR64,
                storage_width: 8,
                ..
            } => Some(i128::from(encoded_field as i64)),
            Self::ElfRela {
                kind: object::elf::R_X86_64_64,
                storage_width: 8,
                addend,
                ..
            } => Some(i128::from(addend)),
            _ => None,
        }
    }

    pub fn absolute64_address(self, encoded_field: u64, base: u64) -> Option<u64> {
        u64::try_from(i128::from(base) + self.absolute64_addend(encoded_field)?).ok()
    }
}

impl Shape {
    /// Resolve an absolute pointer within this object, in section coordinates.
    pub fn absolute64_local_address(
        &self,
        encoded_field: u64,
    ) -> Option<(std::num::NonZeroU32, u64)> {
        use super::VerifiedRelocationTargetV1 as Target;
        let (section, base) = match self.absolute64_target()? {
            Target::LocalDefinition {
                section_ordinal,
                value,
                ..
            } => (*section_ordinal, *value),
            Target::SectionBase {
                section_ordinal, ..
            } => (*section_ordinal, 0),
            _ => return None,
        };
        Some((
            section,
            self.form().absolute64_address(encoded_field, base)?,
        ))
    }

    pub fn absolute64_target(&self) -> Option<&super::VerifiedRelocationTargetV1> {
        match self {
            Self::MachO {
                shape: MachShape::Unsigned64 { target },
                ..
            }
            | Self::Coff {
                kind: object::pe::IMAGE_REL_AMD64_ADDR64,
                storage_width: 8,
                target,
                ..
            }
            | Self::ElfRela {
                kind: object::elf::R_X86_64_64,
                storage_width: 8,
                target,
                ..
            } => Some(target),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coff_absolute_pointers_use_signed_in_place_addends() {
        let absolute = Form::Coff {
            machine: scoop_lir::ObjectMachineV1::X86_64,
            kind: object::pe::IMAGE_REL_AMD64_ADDR64,
            storage_width: 8,
        };
        assert!(absolute.is_absolute64());
        assert_eq!(absolute.absolute64_address(17, 24), Some(41));
        assert_eq!(absolute.absolute64_address(-17i64 as u64, 24), Some(7));
        assert_eq!(absolute.absolute64_address(-17i64 as u64, 0), None);
        assert!(
            !Form::Coff {
                machine: scoop_lir::ObjectMachineV1::X86_64,
                kind: object::pe::IMAGE_REL_AMD64_ADDR32NB,
                storage_width: 4
            }
            .is_absolute64()
        );
    }

    #[test]
    fn rela_signed_addends_are_separate_from_the_relocation_field() {
        let absolute = |addend| Form::ElfRela {
            machine: scoop_lir::ObjectMachineV1::X86_64,
            kind: object::elf::R_X86_64_64,
            addend,
            storage_width: 8,
        };
        assert_eq!(absolute(-8).absolute64_address(0, 24), Some(16));
        assert_eq!(absolute(17).absolute64_address(0, 24), Some(41));
        assert_eq!(absolute(0).absolute64_addend(17), Some(0));
        assert_eq!(absolute(-8).absolute64_address(0, 0), None);
        assert_eq!(absolute(1).absolute64_address(0, u64::MAX), None);
        let macho = Form::MachO {
            machine: scoop_lir::ObjectMachineV1::Aarch64,
            form: MachForm::Unsigned64,
        };
        assert_eq!(macho.absolute64_address(17, 24), Some(41));
        assert_eq!(macho.absolute64_address(0, u64::MAX), Some(u64::MAX));
        for kind in [
            object::elf::R_X86_64_PC64,
            object::elf::R_X86_64_GOT64,
            object::elf::R_X86_64_TPOFF64,
        ] {
            assert!(
                !Form::ElfRela {
                    machine: scoop_lir::ObjectMachineV1::X86_64,
                    kind,
                    addend: 0,
                    storage_width: 8
                }
                .is_absolute64()
            );
        }
    }
}
