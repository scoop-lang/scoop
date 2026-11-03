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
        let exception_model = target.exception_model();
        let unwind_image = target.object_target().unwind_image();
        if !unwind_image.supports_exception_model(exception_model) {
            panic!("target exception model and object unwind image differ");
        }
        let backend_contract = backend.contract();
        if !backend_contract
            .unwind_provider
            .supports_exception_model(exception_model)
        {
            panic!("target exception model and backend unwind provider differ");
        }
        let frame_machine_matches = matches!(
            (
                backend_contract.machine_frame_recovery.object_machine(),
                target.object_target().machine()
            ),
            (
                crate::ObjectMachineV1::Aarch64,
                crate::ObjectMachineV1::Aarch64
            ) | (
                crate::ObjectMachineV1::X86_64,
                crate::ObjectMachineV1::X86_64
            )
        );
        if !frame_machine_matches {
            panic!("backend frame recovery and object machine differ");
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
