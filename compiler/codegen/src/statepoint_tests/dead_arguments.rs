use super::*;
use inkwell::targets::FileType;
use scoop_lir::OptimizationMode;

#[test]
fn returning_call_keeps_argument_root_through_target_memcmp_expansion() {
    for target in [
        scoop_target_data::DARWIN_AARCH64_ID,
        scoop_target_data::DARWIN_X86_64_ID,
        scoop_target_data::LINUX_X86_64_GNU_ID,
        scoop_target_data::LINUX_X86_64_MUSL_ID,
        scoop_target_data::WINDOWS_X86_64_MSVC_UCRT_ID,
    ] {
        for mode in [OptimizationMode::Debug, OptimizationMode::Release] {
            let profile = ValidatedBackendProfile::for_selection(
                scoop_target_data::selection(target).unwrap(),
            )
            .unwrap()
            .with_optimization(mode);
            let machine = profile.create_target_machine().unwrap();
            let context = Context::create();
            let llvm = parse(
                &context,
                r#"
declare i32 @memcmp(ptr, ptr, i64) #1
declare void @consume(ptr addrspace(1), ptr)
define i1 @f(ptr addrspace(1) %object, ptr %itab, ptr %other) gc "statepoint-example" {
entry:
  %comparison = call i32 @memcmp(ptr %itab, ptr %other, i64 16) #1
  %equal = icmp eq i32 %comparison, 0
  call void @consume(ptr addrspace(1) %object, ptr %itab) #0
  ret i1 %equal
}

attributes #0 = { noduplicate "statepoint-id"="7" }
attributes #1 = { "gc-leaf-function" }
"#,
            );
            llvm.set_triple(&machine.get_triple());
            llvm.set_data_layout(&machine.get_target_data().get_data_layout());
            let expected = manifest(Some((7, ExpectedStatepoint::Relocating(Vec::new().into()))));
            let plan = crate::statepoint::rewrite(&llvm, &machine, &expected, profile).unwrap();
            llvm.verify().unwrap();
            assert_eq!(plan.root_count(7), Some(1));
            let directory = tempfile::tempdir().unwrap();
            let object = directory.path().join("dead-argument.o");
            machine
                .write_to_file(&llvm, FileType::Object, &object)
                .unwrap();
            assert_machine_root_counts(&object, &plan);
        }
    }
}

#[test]
fn duplicate_argument_and_continuation_loads_share_one_physical_root() {
    for target in [
        scoop_target_data::DARWIN_AARCH64_ID,
        scoop_target_data::LINUX_X86_64_GNU_ID,
        scoop_target_data::LINUX_X86_64_MUSL_ID,
    ] {
        for mode in [OptimizationMode::Debug, OptimizationMode::Release] {
            let profile = ValidatedBackendProfile::for_selection(
                scoop_target_data::selection(target).unwrap(),
            )
            .unwrap()
            .with_optimization(mode);
            let machine = profile.create_target_machine().unwrap();
            let context = Context::create();
            let llvm = parse(
                &context,
                r#"
declare void @consume(ptr addrspace(1))
declare void @llvm.fake.use(...)
define void @f(ptr %storage) gc "statepoint-example" {
entry:
  %argument = load ptr addrspace(1), ptr %storage
  %continuation = load ptr addrspace(1), ptr %storage
  call void @consume(ptr addrspace(1) %argument) #0
  call void (...) @llvm.fake.use(ptr addrspace(1) %continuation) #1, !scoop.statepoint-root-identity !0
  store volatile ptr addrspace(1) %continuation, ptr %storage
  ret void
}
attributes #0 = { noduplicate "statepoint-id"="7" }
attributes #1 = { noduplicate }
!0 = !{i64 7, i64 0, i64 0, i64 0}
"#,
            );
            llvm.set_triple(&machine.get_triple());
            llvm.set_data_layout(&machine.get_target_data().get_data_layout());
            let plan = crate::statepoint::rewrite(
                &llvm,
                &machine,
                &manifest(Some((7, one_root()))),
                profile,
            )
            .unwrap();
            assert_eq!(plan.root_count(7), Some(1));
            let directory = tempfile::tempdir().unwrap();
            let object = directory.path().join("shared-argument.o");
            machine
                .write_to_file(&llvm, FileType::Object, &object)
                .unwrap();
            assert_machine_root_counts(&object, &plan);
        }
    }
}
