#![cfg(target_os = "macos")]

use std::{fs, os::unix::fs::symlink, path::PathBuf, process::Command};

fn run(command: &mut Command) -> std::process::Output {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{command:?}\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

#[test]
fn explicit_private_framework_list_deploys_a_swift_static_bridge() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path();
    let vendor = root.join("vendor");
    let framework = vendor.join("Answer.framework");
    let version = framework.join("Versions/A");
    for directory in ["Headers", "Modules", "Resources"] {
        fs::create_dir_all(version.join(directory)).unwrap();
    }
    fs::write(root.join("answer.c"), "int answer(void) { return 41; }\n").unwrap();
    fs::write(version.join("Headers/Answer.h"), "int answer(void);\n").unwrap();
    fs::write(
        version.join("Modules/module.modulemap"),
        "framework module Answer { umbrella header \"Answer.h\" export * }\n",
    )
    .unwrap();
    fs::write(version.join("Resources/Info.plist"), r#"<?xml version="1.0"?><plist version="1.0"><dict><key>CFBundleIdentifier</key><string>dev.scoop.answer</string><key>CFBundleExecutable</key><string>Answer</string><key>CFBundlePackageType</key><string>FMWK</string></dict></plist>"#).unwrap();
    symlink("A", framework.join("Versions/Current")).unwrap();
    for name in ["Answer", "Headers", "Modules", "Resources"] {
        symlink(format!("Versions/Current/{name}"), framework.join(name)).unwrap();
    }
    run(Command::new("/usr/bin/xcrun")
        .arg("clang")
        .arg("-dynamiclib")
        .arg(root.join("answer.c"))
        .args([
            "-Wl,-install_name,@rpath/Answer.framework/Versions/A/Answer",
            "-Wl,-headerpad_max_install_names",
            "-o",
        ])
        .arg(version.join("Answer")));
    let package = root.join("bridge");
    fs::create_dir_all(package.join("Sources/Bridge")).unwrap();
    fs::write(package.join("Package.swift"), r#"// swift-tools-version: 6.3
import PackageDescription
import Foundation
let vendor = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent().appendingPathComponent("vendor").path
let package = Package(name: "Bridge", products: [.library(name: "Bridge", type: .static, targets: ["Bridge"])], targets: [.target(name: "Bridge", swiftSettings: [.unsafeFlags(["-F", vendor])])])
"#).unwrap();
    fs::write(package.join("Sources/Bridge/Bridge.swift"), r#"import Foundation
import Answer
@c(bridge_value)
public func bridgeValue() -> Int32 { answer() + (ProcessInfo.processInfo.processorCount > 0 ? 1 : 0) }
"#).unwrap();
    let build = || {
        let mut command = Command::new("/usr/bin/xcrun");
        command
            .args([
                "swift",
                "build",
                "--build-system",
                "swiftbuild",
                "--configuration",
                "release",
                "--product",
                "Bridge",
                "--package-path",
            ])
            .arg(&package)
            .arg("--scratch-path")
            .arg(root.join("swift-work"));
        command
    };
    run(&mut build());
    let bin = PathBuf::from(
        String::from_utf8(run(build().arg("--show-bin-path")).stdout)
            .unwrap()
            .trim(),
    );
    fs::write(root.join("main.c"), "#include <stdio.h>\nint bridge_value(void); int main(void) { printf(\"%d\\n\", bridge_value()); return 0; }\n").unwrap();
    run(Command::new("/usr/bin/xcrun")
        .args(["clang", "-c"])
        .arg(root.join("main.c"))
        .arg("-o")
        .arg(root.join("main.o")));
    let executable = root.join("program");
    run(Command::new("/usr/bin/xcrun")
        .arg("swiftc")
        .arg(root.join("main.o"))
        .arg(bin.join("libBridge.a"))
        .arg("-F")
        .arg(&vendor)
        .args(["-framework", "Answer", "-Xlinker", "-rpath", "-Xlinker"])
        .arg(&vendor)
        .args(["-Xlinker", "-headerpad_max_install_names", "-o"])
        .arg(&executable));
    assert_eq!(run(&mut Command::new(&executable)).stdout, b"42\n");
    let original_program = fs::read(&executable).unwrap();
    let original_framework = fs::read(version.join("Answer")).unwrap();
    fs::write(
        root.join("Bundle.toml"),
        r#"[bundle]
schema = 1
identifier = "dev.scoop.swift-framework"
name = "swift-framework"
version = "1.0.0"
[[bundle.files]]
from = "vendor/Answer.framework"
to = "Answer.framework"
kind = "framework"
"#,
    )
    .unwrap();
    let bundle = || {
        let mut command = Command::new(env!("CARGO_BIN_EXE_scoop-bundle"));
        command
            .arg("--executable")
            .arg(&executable)
            .arg("--config")
            .arg(root.join("Bundle.toml"))
            .args([
                "--target",
                if cfg!(target_arch = "aarch64") {
                    "aarch64-apple-darwin"
                } else {
                    "x86_64-apple-darwin"
                },
                "--message-format",
                "json",
            ]);
        command
    };
    let record = |output: std::process::Output| -> serde_json::Value {
        String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .find(|record| record["kind"] == "bundle")
            .unwrap()
    };
    let first = record(run(&mut bundle()));
    let app = PathBuf::from(first["output"].as_str().unwrap());
    assert_eq!(record(run(&mut bundle()))["reused"], true);
    let copied = root.join("Copied.app");
    run(Command::new("/usr/bin/ditto").arg(&app).arg(&copied));
    assert_eq!(
        run(Command::new(copied.join("Contents/MacOS/swift-framework"))
            .env_remove("DYLD_LIBRARY_PATH")
            .env_remove("DYLD_FRAMEWORK_PATH"))
        .stdout,
        b"42\n"
    );
    let private = fs::read_dir(copied.join("Contents/Frameworks"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect::<Vec<_>>();
    assert_eq!(private, ["Answer.framework"]);
    let commands = run(Command::new("/usr/bin/otool")
        .arg("-l")
        .arg(copied.join("Contents/MacOS/swift-framework")));
    let commands = String::from_utf8(commands.stdout).unwrap();
    assert!(commands.contains("@loader_path/../Frameworks"));
    assert!(!commands.contains(vendor.to_str().unwrap()));
    run(Command::new("/usr/bin/codesign")
        .args(["--verify", "--deep", "--strict"])
        .arg(&copied));
    assert_eq!(fs::read(&executable).unwrap(), original_program);
    assert_eq!(
        fs::read(version.join("Answer")).unwrap(),
        original_framework
    );
}
