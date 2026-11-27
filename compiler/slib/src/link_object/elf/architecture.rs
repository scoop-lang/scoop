//! Implemented ELF machine rules shared by object readers and the final linker.

use object::Architecture;
use scoop_identity::TargetProfileId;
use scoop_lir::{ObjectRelocationProfileV1, ObjectTargetProfileV1};

use super::{ElfObjectError, ElfRelocationKind, aarch64, error, x86_64};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ElfArchitecture {
    Aarch64,
    X86_64,
}

impl ElfArchitecture {
    pub fn for_target(target: TargetProfileId) -> Result<Self, ElfObjectError> {
        Self::select(target).map(|(_, architecture)| architecture)
    }

    pub(super) fn select(
        target: TargetProfileId,
    ) -> Result<(ObjectTargetProfileV1, Self), ElfObjectError> {
        let profile =
            scoop_target_data::layout(target).ok_or_else(|| error("unsupported ELF target"))?;
        let object_target = profile.object_target();
        match object_target.relocation_profile() {
            ObjectRelocationProfileV1::ElfAarch64Rela => Ok((object_target, Self::Aarch64)),
            ObjectRelocationProfileV1::ElfX86_64Rela => Ok((object_target, Self::X86_64)),
            _ => Err(error("selected target does not support ELF")),
        }
    }

    pub fn object_architecture(self) -> Architecture {
        match self {
            Self::X86_64 => Architecture::X86_64,
            Self::Aarch64 => Architecture::Aarch64,
        }
    }

    pub(super) fn check_flags(self, flags: u32) -> Result<(), ElfObjectError> {
        match self {
            Self::X86_64 | Self::Aarch64 if flags == 0 => Ok(()),
            Self::X86_64 => Err(error("nonzero flags in amd64 ELF header")),
            Self::Aarch64 => Err(error("nonzero flags in AArch64 ELF header")),
        }
    }

    pub(super) fn relocation_width(self, kind: u32) -> Result<u8, ElfObjectError> {
        let (machine, relocation) = match self {
            Self::X86_64 => ("amd64", x86_64::relocation_kind(kind)),
            Self::Aarch64 => ("AArch64", aarch64::relocation_kind(kind)),
        };
        relocation
            .map(ElfRelocationKind::width)
            .ok_or_else(|| error(format!("unsupported {machine} ET_REL relocation {kind}")))
    }
}
