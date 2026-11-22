//! Architecture-specific relocation uses shared by bridge and runtime readers.

use super::{VerifiedMachORelocationFormV1 as MachForm, VerifiedObjectRelocationFormV1 as Form};
use crate::link_object::{coff::CoffRelocationKind, elf::ElfRelocationKind};

impl Form {
    pub(in crate::link_object) fn is_tls_reference(self) -> bool {
        match self {
            Self::Coff {
                machine,
                kind,
                storage_width,
            } => matches!(
                CoffRelocationKind::classify(machine, kind, storage_width),
                Some(CoffRelocationKind::SectionRelative32)
            ),
            Self::MachO {
                machine: scoop_lir::ObjectMachineV1::Aarch64,
                form: MachForm::TlvpLoadPage21 | MachForm::TlvpLoadPageOffset12,
            }
            | Self::MachO {
                machine: scoop_lir::ObjectMachineV1::X86_64,
                form:
                    MachForm::X86_64PcRelative32 {
                        kind: super::MachOX86_64PcRelativeKindV1::Tlv,
                    },
            } => true,
            Self::ElfRela {
                machine,
                kind,
                storage_width,
                ..
            } => matches!(
                ElfRelocationKind::classify(machine, kind, storage_width),
                Some(ElfRelocationKind::TlsReference(_))
            ),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use object::elf::{R_X86_64_REX_GOTPCRELX, R_X86_64_TLSGD};

    #[test]
    fn amd64_tls_references_are_distinct_from_ordinary_addresses() {
        for _target in [
            scoop_target_data::LINUX_X86_64_GNU,
            scoop_target_data::LINUX_X86_64_MUSL,
        ] {
            let data = Form::ElfRela {
                machine: scoop_lir::ObjectMachineV1::X86_64,
                kind: R_X86_64_REX_GOTPCRELX,
                storage_width: 4,
                addend: -4,
            };
            let tls = Form::ElfRela {
                machine: scoop_lir::ObjectMachineV1::X86_64,
                kind: R_X86_64_TLSGD,
                storage_width: 4,
                addend: -4,
            };
            assert!(!data.is_tls_reference());
            assert!(tls.is_tls_reference());
            assert!(
                !Form::MachO {
                    machine: scoop_lir::ObjectMachineV1::Aarch64,
                    form: MachForm::Branch26,
                }
                .is_tls_reference()
            );
        }
    }
}
