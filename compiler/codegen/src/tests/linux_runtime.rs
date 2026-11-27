use std::path::{Path, PathBuf};
use std::process::Command;

use inkwell::OptimizationLevel;
use inkwell::context::Context;
use inkwell::memory_buffer::MemoryBuffer;
use inkwell::targets::FileType;

use super::platform_support::{native_float_environment_source, native_runtime_profile};

#[path = "../../../../runtime/tests/compact_frame_fixture.rs"]
mod compact_frame_fixture;
mod images;

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_owned()
}

fn checked(command: &mut Command) -> std::process::Output {
    let output = command.output().expect("execute Linux runtime probe");
    assert!(
        output.status.success(),
        "{command:?}:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn emit_frame_probe(workspace: &Path, object: &Path, triple: &str, opt: OptimizationLevel) {
    let target = scoop_target_data::named_target(triple).unwrap();
    let machine = crate::ValidatedBackendProfile::for_selection(target.selection)
        .unwrap()
        .create_qualification_target_machine(opt)
        .unwrap();
    let context = Context::create();
    let source =
        MemoryBuffer::create_from_file(&workspace.join("runtime/tests/linux_frames_test.ll"))
            .unwrap();
    let module = context.create_module_from_ir(source).unwrap();
    module.set_triple(&machine.get_triple());
    module.set_data_layout(&machine.get_target_data().get_data_layout());
    module.verify().unwrap();
    machine
        .write_to_file(&module, FileType::Object, object)
        .unwrap();
    compact_frame_fixture::write(object, &object.with_file_name("compact_frames.h"));
    checked(
        Command::new("objcopy")
            .args(["--remove-section", ".llvm_stackmaps"])
            .arg(object),
    );
}

#[test]
fn linux_anchors_relocate_nested_roots_across_stack_arguments() {
    let workspace = workspace();
    let root = workspace.join("target/linux-runtime-tests");
    std::fs::create_dir_all(&root).unwrap();
    let directory = tempfile::tempdir_in(root).unwrap();
    let profile = native_runtime_profile();
    for (libc, compiler, mode, flags) in [
        ("gnu", "gcc", "dynamic", ["-fPIE", "-pie"]),
        ("musl", "musl-gcc", "static", ["-static", "-no-pie"]),
        ("musl", "musl-gcc", "dynamic", ["-fPIE", "-pie"]),
    ] {
        for (level, optimization) in [
            ("0", OptimizationLevel::None),
            ("2", OptimizationLevel::Default),
        ] {
            let object = directory
                .path()
                .join(format!("frames-{libc}-{mode}-{level}.o"));
            emit_frame_probe(
                &workspace,
                &object,
                &format!("{}-unknown-linux-{libc}", std::env::consts::ARCH),
                optimization,
            );
            let binary = object.with_extension("exe");
            let mut command = Command::new(compiler);
            command
                .env("TMPDIR", directory.path())
                .args([
                    "-std=c11",
                    "-Wall",
                    "-Wextra",
                    "-Werror",
                    "-pthread",
                    "-ffunction-sections",
                    "-fdata-sections",
                    "-fno-omit-frame-pointer",
                    "-fno-optimize-sibling-calls",
                    "-Wl,--gc-sections",
                    "-Wl,-z,noexecstack",
                ])
                .args(profile.runtime_c_flags())
                .arg(format!("-O{level}"))
                .args(flags)
                .arg("-I")
                .arg(workspace.join("runtime/include"))
                .arg("-I")
                .arg(directory.path())
                .arg(&object);
            for source in [
                "runtime/tests/linux_frames_test.c",
                "runtime/src/platform/os/linux.c",
                "runtime/src/gc_binding/stackmap.c",
            ] {
                command.arg(workspace.join(source));
            }
            for source in profile
                .runtime_sources()
                .into_iter()
                .filter(|path| path.starts_with("runtime/src/platform/frame/"))
            {
                command.arg(workspace.join(source));
            }
            checked(command.arg("-o").arg(&binary));
            let output = checked(&mut Command::new(&binary));
            assert_eq!(
                output.stdout,
                b"Nested anchors, relocated roots and Linux VM passed\n"
            );
        }
    }
}

#[test]
fn linux_thread_publications_refresh_growing_main_stack_bounds() {
    let workspace = workspace();
    let root = workspace.join("target/linux-runtime-tests");
    std::fs::create_dir_all(&root).unwrap();
    let directory = tempfile::tempdir_in(root).unwrap();
    for (libc, compiler, flags) in [
        ("gnu", "gcc", ["-fPIE", "-pie"]),
        ("musl", "musl-gcc", ["-static", "-no-pie"]),
        ("musl", "musl-gcc", ["-fPIE", "-pie"]),
    ] {
        let triple = format!("{}-unknown-linux-{libc}", std::env::consts::ARCH);
        let scoop_toolchain::ValidatedFinalLinkProfile::Linux(link) =
            scoop_toolchain::ValidatedFinalLinkProfile::resolve(&triple).unwrap()
        else {
            unreachable!()
        };
        for optimization in ["-O0", "-O2"] {
            let binary = directory.path().join("growth");
            let mut command = Command::new(compiler);
            command
                .env("TMPDIR", directory.path())
                .args([
                    "-std=c11",
                    "-Wall",
                    "-Wextra",
                    "-Werror",
                    "-pthread",
                    "-fno-omit-frame-pointer",
                    "-fno-optimize-sibling-calls",
                    "-ffunction-sections",
                    optimization,
                ])
                .args(native_runtime_profile().runtime_c_flags())
                .args(flags)
                .arg("-I")
                .arg(workspace.join("runtime/include"))
                .arg("-I")
                .arg(workspace.join("sysroot/vendor/ryu"))
                .arg("-I")
                .arg(workspace.join("sysroot/vendor/libbacktrace"));
            for source in [
                "runtime/tests/linux_stack_growth_test.c",
                "runtime/src/platform/os/linux.c",
                native_float_environment_source(),
                "runtime/src/thread.c",
                "runtime/src/platform/primitives/posix.c",
                "runtime/src/gc/mutator.c",
                "runtime/src/thread/collection.c",
                "runtime/src/thread/transitions.c",
                "runtime/src/thread/roots.c",
            ] {
                command.arg(workspace.join(source));
            }
            command.args(["-Wl,--gc-sections", "-nodefaultlibs"]);
            link.append_system_libraries(&mut command);
            checked(command.arg("-o").arg(&binary));
            let output = checked(&mut Command::new(binary));
            assert_eq!(
                output.stdout,
                b"Linux stack growth across managed, native and callback entries passed\n"
            );
        }
    }
}

#[test]
fn compact_root_decoder_keeps_darwin_frame_alignment_in_its_adapter() {
    let workspace = workspace();
    let root = workspace.join("target/linux-runtime-tests");
    std::fs::create_dir_all(&root).unwrap();
    let directory = tempfile::tempdir_in(root).unwrap();
    let binary = directory.path().join("stackmap");
    let mut command = Command::new("gcc");
    command
        .env("TMPDIR", directory.path())
        .args(["-std=c11", "-Wall", "-Wextra", "-Werror"]);
    for source in [
        "runtime/tests/stackmap_test.c",
        "runtime/src/platform/frame/darwin_aarch64.c",
        "runtime/src/platform/frame/linux_aarch64.c",
        "runtime/src/gc_binding/stackmap.c",
    ] {
        command.arg(workspace.join(source));
    }
    checked(command.arg("-o").arg(&binary));
    checked(&mut Command::new(binary));
}
