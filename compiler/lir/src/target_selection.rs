#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ValidatedLirTargetSelection {
    target: crate::LirTargetProfile,
    backend: crate::BackendProfile,
}

impl ValidatedLirTargetSelection {
    pub const fn new(target: crate::LirTargetProfile, backend: crate::BackendProfile) -> Self {
        let object_target_matches = matches!(
            (
                target.object_target(),
                backend.contract().artifact_inspection
            ),
            (
                crate::ObjectTargetProfileV1::MachOAarch64,
                crate::ObjectTargetProfileV1::MachOAarch64
            ) | (
                crate::ObjectTargetProfileV1::MachOX86_64,
                crate::ObjectTargetProfileV1::MachOX86_64
            ) | (
                crate::ObjectTargetProfileV1::ElfX86_64,
                crate::ObjectTargetProfileV1::ElfX86_64
            ) | (
                crate::ObjectTargetProfileV1::CoffAmd64,
                crate::ObjectTargetProfileV1::CoffAmd64
            )
        );
        if !object_target_matches {
            panic!("target and backend object profiles differ");
        }
        let architecture_matches = matches!(
            (target.architecture(), backend.contract().llvm_architecture),
            (
                crate::TargetArchitecture::Aarch64,
                crate::LlvmArchitecture::Aarch64
            ) | (
                crate::TargetArchitecture::X86_64,
                crate::LlvmArchitecture::X86
            )
        );
        if !architecture_matches {
            panic!("target and backend architectures differ");
        }
        let object_machine_matches = matches!(
            (target.architecture(), target.object_target().machine()),
            (
                crate::TargetArchitecture::Aarch64,
                crate::ObjectMachineV1::Aarch64
            ) | (
                crate::TargetArchitecture::X86_64,
                crate::ObjectMachineV1::X86_64
            )
        );
        if !object_machine_matches {
            panic!("target architecture and object machine differ");
        }
        Self { target, backend }
    }

    pub const fn target(self) -> crate::LirTargetProfile {
        self.target
    }

    pub const fn backend(self) -> crate::BackendProfile {
        self.backend
    }
}
