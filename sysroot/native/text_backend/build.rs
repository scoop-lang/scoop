use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo output directory"));
    compile_system_backend(&out);
    expose_punycode(&out);
}

fn expose_punycode(out: &Path) {
    // Compile the locked RFC 3492 module on its own, with its upstream license.
    let source = "crates/idna-1.1.0/src/punycode.rs";
    let archive = "../../vendor/text_backend/crates.tar.gz";
    println!("cargo:rerun-if-changed={archive}");
    let status = Command::new("tar")
        .args(["-xzf", archive, "-C"])
        .arg(out)
        .arg(source)
        .status()
        .expect("extract vendored idna Punycode");
    assert!(
        status.success(),
        "extracting {source} from {archive} failed"
    );
    fs::write(
        out.join("punycode_module.rs"),
        format!("#[allow(dead_code)]\n#[cfg_attr(test, allow(clippy::manual_repeat_n))]\n#[path = \"{source}\"]\nmod upstream;\n"),
    )
    .expect("write Punycode module path");
}

fn compile_system_backend(out: &Path) {
    for name in ["CC", "SDKROOT", "MACOSX_DEPLOYMENT_TARGET"] {
        println!("cargo:rerun-if-env-changed={name}");
    }
    println!("cargo:rerun-if-changed=src/darwin");
    println!("cargo:rerun-if-changed=include/scoop_text_backend.h");
    let compiler = env::var_os("CC").unwrap_or_else(|| "/usr/bin/clang".into());
    if Path::new(&compiler).is_absolute() {
        println!("cargo:rerun-if-changed={}", Path::new(&compiler).display());
    }
    let sdk = env::var_os("SDKROOT").map(PathBuf::from);
    if let Some(sdk) = &sdk {
        println!(
            "cargo:rerun-if-changed={}",
            sdk.join("SDKSettings.json").display()
        );
    }
    let target = env::var("TARGET").expect("Cargo target triple");
    let mut objects = Vec::new();
    for name in ["domain", "unicode", "memory"] {
        let object = out.join(format!("{name}.o"));
        let mut cmd = Command::new(&compiler);
        cmd.args([
            "-target",
            &target,
            "-std=c11",
            "-Os",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-Iinclude",
        ]);
        if let Some(sdk) = &sdk {
            cmd.arg("-isysroot").arg(sdk);
        }
        let status = cmd
            .arg("-c")
            .arg(format!("src/darwin/{name}.c"))
            .arg("-o")
            .arg(&object)
            .status()
            .expect("compile macOS text backend");
        assert!(status.success(), "compiling {name}.c failed");
        objects.push(object);
    }
    let archive = out.join("libscoop_unicode_darwin.a");
    let status = Command::new("/usr/bin/libtool")
        .args(["-static", "-o"])
        .arg(&archive)
        .args(&objects)
        .status()
        .expect("archive macOS text backend");
    assert!(status.success(), "archiving macOS text backend failed");
    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=scoop_unicode_darwin");
    println!("cargo:rustc-link-lib=dylib=icucore");
    println!("cargo:rustc-link-lib=framework=CoreFoundation");
}
