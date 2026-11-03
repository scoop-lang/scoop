//! Architecture-specific relocation uses shared by bridge and runtime readers.

use super::{VerifiedMachORelocationFormV1 as MachForm, VerifiedObjectRelocationFormV1 as Form};
use object::elf::*;

impl Form {
    pub(in crate::link_object) fn is_tls_reference(self) -> bool {
        matches!(
            self,
            Self::Coff {
                machine: scoop_lir::ObjectMachineV1::X86_64,
                kind: object::pe::IMAGE_REL_AMD64_SECREL,
                storage_width: 4,
            } | Self::MachO {
                machine: scoop_lir::ObjectMachineV1::Aarch64,
                form: MachForm::TlvpLoadPage21 | MachForm::TlvpLoadPageOffset12,
            } | Self::MachO {
                machine: scoop_lir::ObjectMachineV1::X86_64,
                form: MachForm::X86_64PcRelative32 {
                    kind: super::MachOX86_64PcRelativeKindV1::Tlv,
                },
            } | Self::ElfRela {
                machine: scoop_lir::ObjectMachineV1::X86_64,
                kind: R_X86_64_TLSGD
                    | R_X86_64_TLSLD
                    | R_X86_64_DTPOFF32
                    | R_X86_64_GOTTPOFF
                    | R_X86_64_TPOFF32
                    | R_X86_64_GOTPC32_TLSDESC,
                storage_width: 4,
                ..
            } | Self::ElfRela {
                machine: scoop_lir::ObjectMachineV1::X86_64,
                kind: R_X86_64_DTPOFF64 | R_X86_64_TPOFF64,
                storage_width: 8,
                ..
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
