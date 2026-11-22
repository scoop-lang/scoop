//! Implemented ELF machine rules shared by object readers and the final linker.

use object::Architecture;
use scoop_identity::TargetProfileId;
use scoop_lir::{ObjectRelocationProfileV1, ObjectTargetProfileV1};

use super::{ElfObjectError, ElfRelocationKind, error, x86_64};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ElfArchitecture {
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
            ObjectRelocationProfileV1::ElfX86_64Rela => Ok((object_target, Self::X86_64)),
            _ => Err(error("selected target does not support ELF")),
        }
    }

    pub fn object_architecture(self) -> Architecture {
        match self {
            Self::X86_64 => Architecture::X86_64,
        }
    }

    pub(super) fn check_flags(self, flags: u32) -> Result<(), ElfObjectError> {
        match self {
            Self::X86_64 if flags == 0 => Ok(()),
            Self::X86_64 => Err(error("nonzero flags in amd64 ELF header")),
        }
    }

    pub(super) fn relocation_width(self, kind: u32) -> Result<u8, ElfObjectError> {
        match self {
            Self::X86_64 => x86_64::relocation_kind(kind)
                .map(ElfRelocationKind::width)
                .ok_or_else(|| error(format!("unsupported amd64 ET_REL relocation {kind}"))),
        }
    }
}
