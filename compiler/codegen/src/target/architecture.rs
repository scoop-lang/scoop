//! LLVM target initialization owned only by the selected LLVM architecture.

use inkwell::targets::{InitializationConfig, Target};
use scoop_lir::LlvmArchitecture;

pub(super) fn initialize(architecture: LlvmArchitecture) {
    let config = InitializationConfig::default();
    match architecture {
        LlvmArchitecture::Aarch64 => Target::initialize_aarch64(&config),
        LlvmArchitecture::X86 => Target::initialize_x86(&config),
    }
}
