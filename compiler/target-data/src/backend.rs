use scoop_identity::BackendProfileId;
use scoop_lir::{
    BackendDebugInformationFormatV1, BackendLsdaEncodingsV1, BackendProfile,
    BackendProfileContract, MachineFrameRecoveryV1, ObjectTargetProfileV1, UnwindProviderV1,
};

const fn id(name: &str) -> BackendProfileId {
    match BackendProfileId::new(name, 1) {
        Ok(id) => id,
        Err(_) => panic!("invalid registered backend identity"),
    }
}

pub const LLVM_22_1_DARWIN_AARCH64: BackendProfile = BackendProfile::new(
    &id("llvm-22-1-darwin-aarch64"),
    &BackendProfileContract {
        llvm_api_version: [22, 1],
        inkwell_binding: [0, 10, 1],
        cpu: "generic",
        features: "",
        llvm_architecture: scoop_lir::LlvmArchitecture::Aarch64,
        unwind_provider: UnwindProviderV1::DarwinLibSystem,
        artifact_inspection: ObjectTargetProfileV1::MachOAarch64,
        lsda: BackendLsdaEncodingsV1 {
            lp_start: 0xff,
            type_table: 0x9b,
            call_site: 0x01,
            omitted_type_table: None,
        },
        machine_frame_recovery: MachineFrameRecoveryV1::DarwinAarch64Fixed,
        red_zone: scoop_lir::BackendRedZoneV1::Enabled,
        debug_information: BackendDebugInformationFormatV1::Dwarf5,
    },
);

pub const LLVM_22_1_DARWIN_X86_64: BackendProfile = BackendProfile::new(
    &id("llvm-22-1-darwin-x86-64"),
    &BackendProfileContract {
        llvm_api_version: [22, 1],
        inkwell_binding: [0, 10, 1],
        cpu: "x86-64",
        features: "",
        llvm_architecture: scoop_lir::LlvmArchitecture::X86,
        unwind_provider: UnwindProviderV1::DarwinLibSystem,
        artifact_inspection: ObjectTargetProfileV1::MachOX86_64,
        lsda: BackendLsdaEncodingsV1 {
            lp_start: 0xff,
            type_table: 0x9b,
            call_site: 0x01,
            omitted_type_table: Some(0xff),
        },
        machine_frame_recovery: MachineFrameRecoveryV1::SysvAmd64Fixed,
        red_zone: scoop_lir::BackendRedZoneV1::Disabled,
        debug_information: BackendDebugInformationFormatV1::Dwarf5,
    },
);

pub const LLVM_22_1_LINUX_X86_64: BackendProfile = BackendProfile::new(
    &id("llvm-22-1-linux-x86-64"),
    &BackendProfileContract {
        llvm_api_version: [22, 1],
        inkwell_binding: [0, 10, 1],
        cpu: "x86-64",
        features: "",
        llvm_architecture: scoop_lir::LlvmArchitecture::X86,
        unwind_provider: UnwindProviderV1::LlvmLibunwindDwarf,
        artifact_inspection: ObjectTargetProfileV1::ElfX86_64,
        lsda: BackendLsdaEncodingsV1 {
            lp_start: 0xff,
            type_table: 0x9b,
            call_site: 0x01,
            omitted_type_table: Some(0xff),
        },
        machine_frame_recovery: MachineFrameRecoveryV1::SysvAmd64Fixed,
        red_zone: scoop_lir::BackendRedZoneV1::Disabled,
        debug_information: BackendDebugInformationFormatV1::Dwarf5,
    },
);

pub const LLVM_22_1_WINDOWS_X86_64: BackendProfile = BackendProfile::new(
    &id("llvm-22-1-windows-x86-64"),
    &BackendProfileContract {
        llvm_api_version: [22, 1],
        inkwell_binding: [0, 10, 1],
        cpu: "x86-64",
        features: "",
        llvm_architecture: scoop_lir::LlvmArchitecture::X86,
        unwind_provider: UnwindProviderV1::LlvmLibunwindWindowsSeh,
        artifact_inspection: ObjectTargetProfileV1::CoffAmd64,
        lsda: BackendLsdaEncodingsV1 {
            lp_start: 0xff,
            type_table: 0x00,
            call_site: 0x01,
            omitted_type_table: Some(0xff),
        },
        machine_frame_recovery: MachineFrameRecoveryV1::WindowsAmd64VirtualUnwind,
        red_zone: scoop_lir::BackendRedZoneV1::Disabled,
        debug_information: BackendDebugInformationFormatV1::CodeView,
    },
);
