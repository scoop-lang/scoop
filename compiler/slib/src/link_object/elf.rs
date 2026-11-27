//! Shared ELF64 relocatable-file bounds and references, independent of LLVM.
use object::read::elf::{ElfFile64, FileHeader};
use object::{Object, ObjectKind, SectionIndex, SymbolIndex};
use scoop_identity::TargetProfileId;
use scoop_lir::{ObjectMachineV1, ObjectTargetProfileV1};

mod aarch64;
mod architecture;
pub use architecture::ElfArchitecture;
mod envelope;
mod relocations;
mod x86_64;
pub use envelope::validate_elf_object_envelope_v1;
pub(super) mod builtin;
#[cfg(all(test, target_os = "linux", target_arch = "x86_64"))]
mod tests;

#[derive(Clone, Copy)]
pub(super) enum ElfRelocationKind {
    Absolute64,
    TlsReference(u8),
    Other(u8),
}

impl ElfRelocationKind {
    pub(super) fn classify(machine: ObjectMachineV1, kind: u32, width: u8) -> Option<Self> {
        let kind = match machine {
            ObjectMachineV1::X86_64 => x86_64::relocation_kind(kind)?,
            ObjectMachineV1::Aarch64 => aarch64::relocation_kind(kind)?,
        };
        (kind.width() == width).then_some(kind)
    }

    const fn width(self) -> u8 {
        match self {
            Self::Absolute64 => 8,
            Self::TlsReference(width) | Self::Other(width) => width,
        }
    }
}

#[derive(Debug)]
pub struct ValidatedElfObject<'data> {
    object_target: ObjectTargetProfileV1,
    file: ElfFile64<'data>,
    relocations: Vec<ElfRelocation>,
}

#[derive(Clone, Copy, Debug)]
pub struct ElfRelocation {
    pub section: SectionIndex,
    pub offset: u64,
    pub symbol: SymbolIndex,
    pub addend: i64,
    pub kind: u32,
    pub width: u8,
}

impl<'data> ValidatedElfObject<'data> {
    pub fn read(bytes: &'data [u8], target: TargetProfileId) -> Result<Self, ElfObjectError> {
        let (object_target, architecture) = ElfArchitecture::select(target)?;
        let file = ElfFile64::parse(bytes).map_err(error)?;
        if !file.is_little_endian()
            || file.kind() != ObjectKind::Relocatable
            || file.architecture() != architecture.object_architecture()
        {
            return Err(error(
                "expected a little-endian ELF64 relocatable object for the selected target",
            ));
        }
        architecture.check_flags(file.elf_header().e_flags(file.endian()))?;
        let relocations = relocations::read(&file, architecture)?;
        Ok(Self {
            object_target,
            file,
            relocations,
        })
    }

    pub fn file(&self) -> &ElfFile64<'data> {
        &self.file
    }

    pub fn relocations(&self) -> &[ElfRelocation] {
        &self.relocations
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ElfObjectError(String);

impl std::fmt::Display for ElfObjectError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "ELF object: {}", self.0)
    }
}
impl std::error::Error for ElfObjectError {}

fn error(message: impl std::fmt::Display) -> ElfObjectError {
    ElfObjectError(message.to_string())
}
