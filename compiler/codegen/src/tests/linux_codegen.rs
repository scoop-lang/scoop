//! These tests cross-emit ELF using the linked LLVM on every compiler host.

use inkwell::module::Linkage;
use object::{Object, ObjectSection};

use super::*;

#[cfg(target_os = "linux")]
mod bridges;
mod objects;
#[cfg(target_os = "linux")]
mod slib_eh;
#[cfg(target_os = "linux")]
mod slib_image;
#[cfg(target_os = "linux")]
mod slib_local_relocations;
#[cfg(target_os = "linux")]
mod slib_metadata;
#[cfg(target_os = "linux")]
mod slib_metadata_fixture;
#[cfg(target_os = "linux")]
mod slib_requirements;
#[cfg(target_os = "linux")]
mod slib_stackmaps;
#[cfg(target_os = "linux")]
mod slib_support;
#[cfg(target_os = "linux")]
mod slib_types;

#[cfg(target_os = "linux")]
fn native_linux_targets() -> impl Iterator<Item = scoop_lir::LirTargetProfile> {
    let host = scoop_target_data::host("linux", std::env::consts::ARCH, "").unwrap();
    host.distribution_targets.iter().map(|name| {
        scoop_target_data::named_target(name)
            .unwrap()
            .selection
            .target()
    })
}

fn for_target(mut module: Module, target: scoop_lir::TargetProfileId) -> Module {
    module.meta = string_metadata_for(
        scoop_target_data::layout(target).expect("implemented target selected by this consumer"),
    );
    module
}

fn emit(module: &Module, optimization: OptimizationLevel, separate_sections: bool, path: &Path) {
    let profile = ValidatedBackendProfile::for_selection(
        scoop_target_data::selection(module.meta.target_profile.id())
            .expect("implemented target selected by this consumer"),
    )
    .expect("Linux backend")
    .with_optimization(if optimization == OptimizationLevel::None {
        scoop_lir::OptimizationMode::Debug
    } else {
        scoop_lir::OptimizationMode::Release
    });
    let machine = profile
        .create_qualification_target_machine(optimization)
        .expect("target machine");
    let context = Context::create();
    let llvm = emit_llvm_module(&context, module, &machine, profile).expect("emit actual LIR");
    if separate_sections {
        for function in llvm
            .get_functions()
            .filter(|function| function.count_basic_blocks() != 0)
        {
            let name = function.get_name().to_str().unwrap();
            function.set_linkage(Linkage::WeakODR);
            function
                .as_global_value()
                .set_comdat(llvm.get_or_insert_comdat(name));
        }
    }
    let expected = statepoint::expectations(module).unwrap();
    crate::metadata_sections::place_immutable_metadata(&llvm, profile);
    llvm.verify().expect("valid LLVM IR");
    statepoint::rewrite(&llvm, &machine, &expected, profile).expect("RS4GC");
    llvm.verify().expect("valid rewritten IR");

    machine
        .write_to_file(&llvm, FileType::Object, path)
        .expect("emit ELF");
}

#[test]
fn linux_statepoints_and_exceptions_at_o0_and_o2() {
    let directory = tempfile::tempdir().unwrap();
    for (target, arch) in [
        (
            scoop_target_data::LINUX_X86_64_GNU_ID,
            object::Architecture::X86_64,
        ),
        (
            scoop_target_data::LINUX_X86_64_MUSL_ID,
            object::Architecture::X86_64,
        ),
        (
            scoop_target_data::LINUX_AARCH64_GNU_ID,
            object::Architecture::Aarch64,
        ),
        (
            scoop_target_data::LINUX_AARCH64_MUSL_ID,
            object::Architecture::Aarch64,
        ),
    ] {
        for optimization in [OptimizationLevel::None, OptimizationLevel::Default] {
            for separate_sections in [false, true] {
                for module in [
                    moving_gc::qualification::stackmap_qualification_module(),
                    exceptions_module(),
                ] {
                    let module = for_target(module, target);
                    let path = directory.path().join("code.o");
                    emit(&module, optimization, separate_sections, &path);
                    let bytes = std::fs::read(&path).unwrap();
                    let object = object::File::parse(bytes.as_slice()).unwrap();
                    assert_eq!(object.architecture(), arch);
                    assert_eq!(object.kind(), object::ObjectKind::Relocatable);
                    assert!(object.section_by_name(".llvm_stackmaps").unwrap().size() > 16);
                    assert!(object.section_by_name(".eh_frame").is_some());
                }
            }
        }
    }
}

#[test]
fn linux_requires_noredzone_on_rewritten_functions() {
    for target in [
        scoop_target_data::LINUX_X86_64_GNU_ID,
        scoop_target_data::LINUX_AARCH64_GNU_ID,
    ] {
        let module = for_target(
            moving_gc::qualification::stackmap_qualification_module(),
            target,
        );
        let profile = ValidatedBackendProfile::for_selection(
            scoop_target_data::selection(module.meta.target_profile.id())
                .expect("implemented target selected by this consumer"),
        )
        .unwrap();
        let machine = profile.create_target_machine().unwrap();
        let context = Context::create();
        let llvm = emit_llvm_module(&context, &module, &machine, profile).unwrap();
        statepoint::rewrite(
            &llvm,
            &machine,
            &statepoint::expectations(&module).unwrap(),
            profile,
        )
        .unwrap();
        let function = llvm.get_function(module.functions[0].symbol()).unwrap();
        assert!(
            function
                .get_enum_attribute(
                    AttributeLoc::Function,
                    Attribute::get_named_enum_kind_id("noredzone"),
                )
                .is_some()
        );
    }
}
