use scoop_lir::NativeSymbolNormalization;

#[test]
fn backend_machine_policies_are_explicit_catalog_contracts() {
    assert_eq!(
        crate::LLVM_22_1_DARWIN_AARCH64.contract().red_zone,
        scoop_lir::BackendRedZoneV1::Enabled
    );
    for backend in [
        crate::LLVM_22_1_DARWIN_X86_64,
        crate::LLVM_22_1_LINUX_X86_64,
        crate::LLVM_22_1_WINDOWS_X86_64,
    ] {
        assert_eq!(
            backend.contract().red_zone,
            scoop_lir::BackendRedZoneV1::Disabled
        );
    }
}

#[test]
fn spelling_and_host_selection_use_the_complete_catalog() {
    for target in crate::TARGETS {
        assert_eq!(crate::named_target(target.canonical()), Some(target));
        assert_eq!(
            target.architecture,
            target.selection.target().architecture(),
            "{}",
            target.canonical()
        );
        assert_eq!(
            target.selection.target().object_target(),
            target.selection.backend().contract().artifact_inspection,
            "{}",
            target.canonical()
        );
        assert!(matches!(
            (
                target.architecture,
                target.selection.target().object_target().machine()
            ),
            (
                scoop_lir::TargetArchitecture::Aarch64,
                scoop_lir::ObjectMachineV1::Aarch64
            ) | (
                scoop_lir::TargetArchitecture::X86_64,
                scoop_lir::ObjectMachineV1::X86_64
            )
        ));
        let exception_model = target.selection.target().exception_model();
        let unwind_image = target.selection.target().object_target().unwind_image();
        assert!(unwind_image.supports_exception_model(exception_model));
        assert!(
            target
                .selection
                .backend()
                .contract()
                .unwind_provider
                .supports_exception_model(exception_model)
        );
        assert!(matches!(
            (target.os, target.codegen_deployment, target.final_link),
            (
                crate::TargetOs::Darwin,
                crate::CodegenDeploymentProfile::Darwin,
                crate::FinalLinkProfile::Darwin
            ) | (
                crate::TargetOs::Linux,
                crate::CodegenDeploymentProfile::Linux,
                crate::FinalLinkProfile::Linux
            ) | (
                crate::TargetOs::Windows,
                crate::CodegenDeploymentProfile::Windows,
                crate::FinalLinkProfile::Windows
            )
        ));
        let expected_debug_information = match target.os {
            crate::TargetOs::Darwin | crate::TargetOs::Linux => {
                scoop_lir::BackendDebugInformationFormatV1::Dwarf5
            }
            crate::TargetOs::Windows => scoop_lir::BackendDebugInformationFormatV1::CodeView,
        };
        assert_eq!(
            target.selection.backend().contract().debug_information,
            expected_debug_information,
            "{}",
            target.canonical()
        );
        for alias in target.aliases {
            assert_eq!(crate::named_target(alias), Some(target));
        }
    }
    for alias in ["arm64-apple-darwin25.6.0", "aarch64-apple-macosx14.0.0"] {
        assert_eq!(
            crate::named_target(alias).unwrap().selection,
            crate::DARWIN_AARCH64_LLVM_22_1
        );
    }
    for invalid in [
        "arm64-apple-darwin25.invalid",
        "arm64-apple-ios",
        "x86_64-pc-windows-msvc",
        "x86_64-w64-windows-gnu",
    ] {
        assert!(crate::named_target(invalid).is_none());
    }
    let windows = crate::host("windows", "x86_64", "").unwrap();
    assert_eq!(windows.rust_target, "x86_64-pc-windows-gnullvm");
    assert_eq!(
        windows.default_target,
        crate::WINDOWS_X86_64_MSVC_UCRT
            .contract()
            .canonical_triple()
    );
    assert_eq!(
        crate::host("linux", "x86_64", "musl")
            .unwrap()
            .default_target,
        crate::LINUX_X86_64_MUSL.contract().canonical_triple()
    );
    assert!(crate::host("linux", "aarch64", "").is_none());
    for alias in ["x86_64-apple-darwin27.0.1", "x86_64-apple-macosx27.0.1"] {
        assert_eq!(
            crate::named_target(alias).unwrap().selection,
            crate::DARWIN_X86_64_LLVM_22_1
        );
    }
    assert_eq!(
        crate::host("macos", "x86_64", "").unwrap().default_target,
        "x86_64-apple-darwin"
    );
    assert_eq!(
        crate::host("macos", "aarch64", "").unwrap().default_target,
        "aarch64-apple-darwin"
    );

    let darwin = crate::named_target("aarch64-apple-darwin").unwrap();
    assert_eq!(darwin.os, crate::TargetOs::Darwin);
    assert_eq!(darwin.source_os(), "darwin");
    assert_eq!(darwin.architecture, scoop_lir::TargetArchitecture::Aarch64);
    assert_eq!(darwin.source_arch(), "aarch64");

    let linux = crate::named_target("x86_64-unknown-linux-gnu").unwrap();
    assert_eq!(linux.os, crate::TargetOs::Linux);
    assert_eq!(linux.source_os(), "linux");
    assert_eq!(linux.architecture, scoop_lir::TargetArchitecture::X86_64);
    assert_eq!(linux.source_arch(), "x86_64");

    let windows = crate::named_target("x86_64-pc-windows-msvc-ucrt").unwrap();
    assert_eq!(windows.os, crate::TargetOs::Windows);
    assert_eq!(windows.source_os(), "windows");
    assert_eq!(windows.architecture, scoop_lir::TargetArchitecture::X86_64);
    assert_eq!(windows.source_arch(), "x86_64");
}

#[test]
fn windows_environments_keep_distinct_identity_and_seh_frame_recovery() {
    let profiles = [
        crate::WINDOWS_X86_64_MSVC_UCRT,
        crate::WINDOWS_X86_64_MINGW_MSVCRT,
        crate::WINDOWS_X86_64_MINGW_UCRT,
    ];
    let fingerprints = profiles.map(|profile| profile.fingerprint().unwrap().to_string());
    assert_eq!(
        fingerprints
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        3
    );
    for profile in profiles {
        let selection = crate::selection(profile.id()).unwrap();
        assert_eq!(selection.backend(), crate::LLVM_22_1_WINDOWS_X86_64);
        assert_eq!(
            profile.native_object_format(),
            scoop_lir::NativeObjectFormat::Coff64
        );
        assert_eq!(
            profile.contract().native_symbol_normalization(),
            NativeSymbolNormalization::CoffIdentity
        );
        assert_eq!(
            profile.contract().scoop_abi_classifier(),
            scoop_lir::ScoopAbiClassifier::Win64CallerCopyAndIndirectInterfaceResult
        );
        assert_eq!(
            profile.object_target().unwind_image(),
            scoop_lir::ObjectUnwindImageV1::CoffAmd64PdataXdata
        );
        assert!(matches!(
            selection.backend().contract().machine_frame_recovery,
            scoop_lir::MachineFrameRecoveryV1::WindowsAmd64VirtualUnwind
        ));
        assert_eq!(
            scoop_lir::TargetEhSupportV1::ScoopPersonality.logical_symbol_for_target(profile),
            "scoop_eh_personality_seh"
        );
    }
}

#[test]
fn linux_libcs_have_distinct_targets_and_share_the_x86_backend() {
    let gnu = crate::selection(crate::LINUX_X86_64_GNU_ID)
        .expect("implemented target selected by this consumer");
    let musl = crate::selection(crate::LINUX_X86_64_MUSL_ID)
        .expect("implemented target selected by this consumer");
    assert_eq!(gnu.backend(), musl.backend());
    assert_ne!(
        gnu.backend().wire_id(),
        crate::LLVM_22_1_DARWIN_AARCH64.wire_id()
    );
    assert_ne!(
        gnu.backend().fingerprint().unwrap(),
        crate::LLVM_22_1_DARWIN_AARCH64.fingerprint().unwrap()
    );
    assert_ne!(gnu.target().wire_id(), musl.target().wire_id());
    assert_ne!(
        gnu.target().fingerprint().unwrap(),
        musl.target().fingerprint().unwrap()
    );
    for profile in [gnu.target(), musl.target()] {
        assert_eq!(
            profile.object_target().unwind_image(),
            scoop_lir::ObjectUnwindImageV1::ElfX86_64EhFrame
        );
        assert_eq!(
            profile.contract().native_symbol_normalization(),
            NativeSymbolNormalization::ElfIdentity
        );
        assert_eq!(profile.contract().stack_alignment_bytes(), 16);
        assert_eq!(profile.managed_pointer_layout().size_bytes(), 8);
        assert_eq!(
            profile.native_object_format().wire_id(),
            scoop_identity::ObjectFormatId::elf_relocatable()
        );
        assert_eq!(
            profile
                .contract()
                .native_symbol_normalization()
                .compiler_generated_object_symbol("_entry"),
            "_entry"
        );
    }
}
