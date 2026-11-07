#![cfg(target_os = "macos")]

use serde_json::Value;
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn successful(command: &mut Command) -> Output {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{command:?}\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn record(output: &Output) -> Value {
    String::from_utf8(output.stdout.clone())
        .unwrap()
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .find(|value| value["kind"] == "bundle")
        .unwrap()
}

fn compile(root: &Path) -> PathBuf {
    let framework = root.join("vendor/Answer.framework");
    let version = framework.join("Versions/A");
    fs::create_dir_all(version.join("Resources")).unwrap();
    fs::write(root.join("side.c"), "int side(void) { return 41; }\n").unwrap();
    fs::write(
        root.join("answer.c"),
        "int side(void); int answer(void) { return side() + 1; }\n",
    )
    .unwrap();
    fs::write(
        root.join("main.c"),
        r#"
#include <mach-o/dyld.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
int answer(void);
int main(int argc, char **argv) {
    char executable[4096], path[8192], message[256] = {0};
    uint32_t size = sizeof(executable);
    if (_NSGetExecutablePath(executable, &size)) return 2;
    char *resolved = realpath(executable, NULL);
    if (!resolved) return 3;
    *strrchr(resolved, '/') = 0;
    snprintf(path, sizeof(path), "%s/../Resources/assets/message.txt", resolved);
    free(resolved);
    FILE *file = fopen(path, "r");
    if (!file) return 4;
    if (!fgets(message, sizeof(message), file)) return 5;
    fclose(file);
    printf("%s:%d:%s\n", message, answer(), argc > 1 ? argv[1] : "");
    return 0;
}
"#,
    )
    .unwrap();
    let side = root.join("vendor/libSide.dylib");
    successful(
        Command::new("clang")
            .args(["-arch", "arm64", "-arch", "x86_64"])
            .arg("-dynamiclib")
            .arg(root.join("side.c"))
            .arg("-o")
            .arg(&side)
            .args(["-Xlinker", "-install_name", "-Xlinker"])
            .arg(&side)
            .arg("-Wl,-headerpad_max_install_names"),
    );
    successful(
        Command::new("clang")
            .args(["-arch", "arm64", "-arch", "x86_64"])
            .arg("-dynamiclib")
            .arg(root.join("answer.c"))
            .arg(&side)
            .arg("-o")
            .arg(version.join("Answer"))
            .args([
                "-Wl,-install_name,@rpath/Answer.framework/Versions/A/Answer",
                "-Wl,-headerpad_max_install_names",
            ]),
    );
    fs::write(version.join("Resources/Info.plist"),r#"<?xml version="1.0" encoding="UTF-8"?><plist version="1.0"><dict><key>CFBundleIdentifier</key><string>dev.scoop.answer</string><key>CFBundleExecutable</key><string>Answer</string><key>CFBundlePackageType</key><string>FMWK</string><key>CFBundleVersion</key><string>1</string></dict></plist>"#).unwrap();
    symlink("A", framework.join("Versions/Current")).unwrap();
    symlink("Versions/Current/Answer", framework.join("Answer")).unwrap();
    symlink("Versions/Current/Resources", framework.join("Resources")).unwrap();
    let executable = root.join("original program");
    successful(
        Command::new("clang")
            .args(["-arch", "arm64", "-arch", "x86_64"])
            .arg(root.join("main.c"))
            .arg("-F")
            .arg(root.join("vendor"))
            .args(["-framework", "Answer", "-Xlinker", "-rpath", "-Xlinker"])
            .arg(root.join("vendor"))
            .arg("-Wl,-headerpad_max_install_names")
            .arg("-o")
            .arg(&executable),
    );
    executable
}

#[test]
fn real_app_preserves_frameworks_relocates_transitive_libraries_and_publishes_only_on_success() {
    let temporary = tempfile::Builder::new()
        .prefix("scoop bundle 中文 ")
        .tempdir()
        .unwrap();
    let root = temporary.path();
    let executable = compile(root);
    let original = fs::read(&executable).unwrap();
    let framework_source = root.join("vendor/Answer.framework/Versions/A/Answer");
    let original_framework = fs::read(&framework_source).unwrap();
    fs::create_dir(root.join("assets")).unwrap();
    fs::write(root.join("assets/message.txt"), "resource").unwrap();
    let image = image::RgbaImage::from_pixel(512, 512, image::Rgba([32, 96, 160, 255]));
    image.save(root.join("icon.png")).unwrap();
    fs::create_dir(root.join("tools")).unwrap();
    let signing_tool = root.join("tools/codesign");
    fs::write(&signing_tool,"#!/bin/sh\nprintf 'call\\n' >> \"$(dirname \"$0\")/calls\"\nexec /usr/bin/codesign \"$@\"\n").unwrap();
    fs::set_permissions(&signing_tool, fs::Permissions::from_mode(0o755)).unwrap();
    let configuration = r#"
[bundle]
schema = 1
identifier = "dev.scoop.bundle-test"
name = "bundle-test"
display-name = "Bundle 测试"
version = "1.2.3"
icons = ["icon.png"]
library-search-paths = ["vendor"]
[[bundle.files]]
from = "assets"
to = "assets"
kind = "resource"
[[bundle.files]]
from = "vendor/Answer.framework"
to = "Answer.framework"
kind = "framework"
when = { os = "darwin" }
[bundle.tools]
codesign = "tools/codesign"
"#;
    fs::write(root.join("Bundle.toml"), configuration).unwrap();
    let target = if cfg!(target_arch = "aarch64") {
        "aarch64-apple-darwin"
    } else {
        "x86_64-apple-darwin"
    };
    let command = || {
        let mut command = Command::new(env!("CARGO_BIN_EXE_scoop-bundle"));
        command
            .current_dir(root)
            .arg("--executable")
            .arg(&executable)
            .args([
                "--config",
                "Bundle.toml",
                "--target",
                target,
                "--message-format",
                "json",
            ]);
        command
    };
    let first = record(&successful(&mut command()));
    assert_eq!(first["reused"], false);
    let app = PathBuf::from(first["output"].as_str().unwrap());
    for relative in [
        "Contents/MacOS/bundle-test",
        "Contents/Frameworks/Answer.framework/Versions/A/Answer",
        "Contents/Frameworks/libSide.dylib",
    ] {
        let architectures = successful(Command::new("lipo").arg("-archs").arg(app.join(relative)));
        assert_eq!(
            String::from_utf8(architectures.stdout).unwrap().trim(),
            if cfg!(target_arch = "aarch64") {
                "arm64"
            } else {
                "x86_64"
            }
        );
    }
    assert_eq!(
        fs::read_link(app.join("Contents/Frameworks/Answer.framework/Versions/Current")).unwrap(),
        Path::new("A")
    );
    assert!(app.join("Contents/Frameworks/libSide.dylib").is_file());
    assert!(app.join("Contents/Resources/AppIcon.icns").is_file());
    let run = |expected: &str| {
        let result = successful(
            Command::new(app.join("Contents/MacOS/bundle-test"))
                .arg("a b 中文")
                .current_dir("/tmp"),
        );
        assert_eq!(
            String::from_utf8(result.stdout).unwrap(),
            format!("{expected}:42:a b 中文\n")
        );
    };
    run("resource");
    let count = fs::read(root.join("tools/calls")).unwrap();
    assert_eq!(record(&successful(&mut command()))["reused"], true);
    assert_eq!(fs::read(root.join("tools/calls")).unwrap(), count);
    fs::write(root.join("assets/message.txt"), "updated resource").unwrap();
    assert_eq!(record(&successful(&mut command()))["reused"], false);
    run("updated resource");
    fs::write(root.join("assets/new-file.txt"), "new directory member").unwrap();
    assert_eq!(record(&successful(&mut command()))["reused"], false);
    assert!(app.join("Contents/Resources/assets/new-file.txt").is_file());
    let before = fs::read(app.join("Contents/MacOS/bundle-test")).unwrap();
    fs::write(&signing_tool, "#!/bin/sh\nexit 7\n").unwrap();
    let failure = command().output().unwrap();
    assert!(!failure.status.success());
    assert!(String::from_utf8_lossy(&failure.stderr).contains("codesign failed"));
    assert_eq!(
        fs::read(app.join("Contents/MacOS/bundle-test")).unwrap(),
        before
    );
    run("updated resource");
    assert_eq!(fs::read(&executable).unwrap(), original);
    assert_eq!(fs::read(&framework_source).unwrap(), original_framework);
    assert!(
        fs::read_dir(app.parent().unwrap().parent().unwrap())
            .unwrap()
            .all(|entry| !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".app-"))
    );
}
