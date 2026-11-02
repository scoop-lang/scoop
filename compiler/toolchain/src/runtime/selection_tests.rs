use std::collections::BTreeSet;

use scoop_lir::LirTargetProfile;

use super::ValidatedRuntimeBuildProfile;

struct ExpectedComponents {
    profile: &'static str,
    os: &'static str,
    image: &'static str,
    arch: &'static str,
    frame: &'static str,
    no_red_zone: bool,
}

fn assert_components(target: LirTargetProfile, expected: ExpectedComponents) {
    let profile = ValidatedRuntimeBuildProfile::for_target(target).unwrap();
    let sources = profile.runtime_sources();
    let unique_sources: BTreeSet<_> = sources.iter().copied().collect();
    assert_eq!(unique_sources.len(), sources.len());
    for expected_source in [
        expected.profile,
        expected.os,
        expected.image,
        expected.arch,
        expected.frame,
    ] {
        assert!(
            unique_sources.contains(expected_source),
            "{} does not select {expected_source}",
            profile.canonical_triple()
        );
    }
    assert_eq!(
        profile.runtime_c_flags().contains(&"-mno-red-zone"),
        expected.no_red_zone
    );
}

#[test]
fn current_targets_compose_runtime_from_independent_facts() {
    assert_components(
        scoop_target_data::DARWIN_AARCH64,
        ExpectedComponents {
            profile: "runtime/src/platform/profiles/darwin_aarch64.c",
            os: "runtime/src/platform/os/darwin.c",
            image: "runtime/src/platform/image/macho.c",
            arch: "runtime/src/platform/arch/aarch64_float_environment.c",
            frame: "runtime/src/platform/frame/darwin_aarch64.c",
            no_red_zone: false,
        },
    );
    assert_components(
        scoop_target_data::DARWIN_X86_64,
        ExpectedComponents {
            profile: "runtime/src/platform/profiles/darwin_x86_64.c",
            os: "runtime/src/platform/os/darwin.c",
            image: "runtime/src/platform/image/macho.c",
            arch: "runtime/src/platform/arch/x86_64_float_environment.c",
            frame: "runtime/src/platform/frame/sysv_amd64.c",
            no_red_zone: true,
        },
    );
    for target in [
        scoop_target_data::LINUX_X86_64_GNU,
        scoop_target_data::LINUX_X86_64_MUSL,
    ] {
        assert_components(
            target,
            ExpectedComponents {
                profile: "runtime/src/platform/profiles/linux_x86_64.c",
                os: "runtime/src/platform/os/linux.c",
                image: "runtime/src/platform/image/elf.c",
                arch: "runtime/src/platform/arch/x86_64_float_environment.c",
                frame: "runtime/src/platform/frame/sysv_amd64.c",
                no_red_zone: true,
            },
        );
    }
    for target in [
        scoop_target_data::WINDOWS_X86_64_MSVC_UCRT,
        scoop_target_data::WINDOWS_X86_64_MINGW_MSVCRT,
        scoop_target_data::WINDOWS_X86_64_MINGW_UCRT,
    ] {
        assert_components(
            target,
            ExpectedComponents {
                profile: "runtime/src/platform/profiles/windows_x86_64.c",
                os: "runtime/src/platform/os/windows.c",
                image: "runtime/src/platform/image/pe.c",
                arch: "runtime/src/platform/arch/x86_64_float_environment.c",
                frame: "runtime/src/platform/frame/windows_seh.c",
                no_red_zone: true,
            },
        );
    }
}

#[test]
fn libc_environment_does_not_change_runtime_component_selection() {
    let linux_gnu =
        ValidatedRuntimeBuildProfile::for_target(scoop_target_data::LINUX_X86_64_GNU).unwrap();
    let linux_musl =
        ValidatedRuntimeBuildProfile::for_target(scoop_target_data::LINUX_X86_64_MUSL).unwrap();
    assert_eq!(linux_gnu.runtime_sources(), linux_musl.runtime_sources());
    assert_eq!(linux_gnu.runtime_c_flags(), linux_musl.runtime_c_flags());

    let windows_targets = [
        scoop_target_data::WINDOWS_X86_64_MSVC_UCRT,
        scoop_target_data::WINDOWS_X86_64_MINGW_MSVCRT,
        scoop_target_data::WINDOWS_X86_64_MINGW_UCRT,
    ];
    let baseline = ValidatedRuntimeBuildProfile::for_target(windows_targets[0]).unwrap();
    for target in windows_targets.into_iter().skip(1) {
        let candidate = ValidatedRuntimeBuildProfile::for_target(target).unwrap();
        assert_eq!(baseline.runtime_sources(), candidate.runtime_sources());
        assert_eq!(baseline.runtime_c_flags(), candidate.runtime_c_flags());
    }
}
