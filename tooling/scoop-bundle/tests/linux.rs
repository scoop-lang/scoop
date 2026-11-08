#![cfg(target_os = "linux")]

use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
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

fn compile(root: &Path) -> PathBuf {
    let vendor = root.join("vendor");
    fs::create_dir(&vendor).unwrap();
    fs::write(root.join("side.c"), "int side(void) { return 41; }\n").unwrap();
    fs::write(
        root.join("answer.c"),
        "int side(void); int answer(void) { return side() + 1; }\n",
    )
    .unwrap();
    fs::write(root.join("main.c"),r#"
#include <unistd.h>
#include <stdio.h>
#include <string.h>
#include <dlfcn.h>
int answer(void);
int main(int argc, char **argv) {
    void *library = dlopen("libAnswer.so.1", RTLD_NOW);
    if (!library || dlsym(library, "answer") != (void *)answer) return 9;
    char executable[4096], path[8192], message[256] = {0};
    ssize_t size = readlink("/proc/self/exe", executable, sizeof(executable)-1);
    if (size < 0) return 2;
    executable[size] = 0;
    *strrchr(executable, '/') = 0;
    snprintf(path, sizeof(path), "%s/../../../share/dev.scoop.bundle-test/assets/message.txt", executable);
    FILE *file = fopen(path, "r");
    if (!file) return 4;
    if (!fgets(message, sizeof(message), file)) return 5;
    fclose(file);
    printf("%s:%d:%s\n", message, answer(), argc > 1 ? argv[1] : "");
    return 0;
}
"#).unwrap();
    let side = vendor.join("libSide.so.1");
    successful(
        Command::new("cc")
            .args(["-shared", "-fPIC"])
            .arg(root.join("side.c"))
            .arg("-o")
            .arg(&side)
            .arg("-Wl,-soname,libSide.so.1"),
    );
    let answer = vendor.join("libAnswer.so.1");
    successful(
        Command::new("cc")
            .args(["-shared", "-fPIC"])
            .arg(root.join("answer.c"))
            .arg(&side)
            .arg("-o")
            .arg(&answer)
            .args([
                "-Wl,-soname,libAnswer.so.1",
                "-Xlinker",
                "-rpath",
                "-Xlinker",
            ])
            .arg(&vendor),
    );
    let executable = root.join("original program");
    successful(
        Command::new("cc")
            .arg(root.join("main.c"))
            .arg(answer)
            .arg("-o")
            .arg(&executable)
            .args(["-Xlinker", "-rpath", "-Xlinker"])
            .arg(vendor),
    );
    executable
}

#[test]
fn native_linux_packages_install_upgrade_remove_and_run_with_private_libraries() {
    let temporary = tempfile::Builder::new()
        .prefix("scoop bundle 中文 ")
        .tempdir()
        .unwrap();
    let root = temporary.path();
    let executable = compile(root);
    let original = fs::read(&executable).unwrap();
    fs::create_dir_all(root.join("assets/empty")).unwrap();
    fs::write(root.join("assets/message.txt"), "resource").unwrap();
    image::RgbaImage::from_pixel(256, 256, image::Rgba([32, 96, 160, 255]))
        .save(root.join("icon.png"))
        .unwrap();
    let configuration = r#"
[bundle]
schema = 1
identifier = "dev.scoop.bundle-test"
name = "bundle-test"
display-name = "Bundle 测试"
version = "1.2.3"
description = "Scoop bundle integration test"
license = "UNLICENSED"
icons = ["icon.png"]
library-search-paths = ["vendor"]
[[bundle.files]]
from = "assets"
to = "assets"
kind = "resource"
[[bundle.files]]
from = "vendor/libAnswer.so.1"
to = "nested/private-answer.so"
kind = "library"
[bundle.linux]
formats = ["deb", "rpm", "appimage"]
terminal = true
[bundle.linux.deb]
maintainer = "Scoop Tests <noreply@example.com>"
"#;
    let config = root.join("Bundle.toml");
    fs::write(&config, configuration).unwrap();
    let command = |formats: &[&str]| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_scoop-bundle"));
        command
            .current_dir(root)
            .arg("--executable")
            .arg(&executable)
            .args([
                "--config",
                "Bundle.toml",
                "--target",
                "x86_64-unknown-linux-gnu",
                "--message-format",
                "json",
            ]);
        for format in formats {
            command.args(["--format", format]);
        }
        command
    };
    let records = |output: Output| -> BTreeMap<String, Value> {
        String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).unwrap())
            .filter(|v| v["kind"] == "bundle")
            .map(|v| (v["format"].as_str().unwrap().to_owned(), v))
            .collect()
    };
    let first = records(successful(&mut command(&[])));
    assert_eq!(first.len(), 3);
    let packages: BTreeMap<_, _> = first
        .iter()
        .map(|(format, value)| {
            (
                format.as_str(),
                PathBuf::from(value["output"].as_str().unwrap()),
            )
        })
        .collect();
    let run = |program: &Path| {
        let output = successful(Command::new(program).arg("a b 中文").current_dir("/tmp"));
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            "resource:42:a b 中文\n"
        );
    };
    run(&packages["appimage"]);
    let deb_root = root.join("deb-install");
    fs::create_dir_all(deb_root.join("var/lib/dpkg")).unwrap();
    let dpkg = || {
        let mut command = Command::new("dpkg");
        command
            .arg(format!("--root={}", deb_root.display()))
            .arg("--force-not-root");
        command
    };
    successful(dpkg().arg("--install").arg(&packages["deb"]));
    run(&deb_root.join("usr/bin/bundle-test"));
    assert!(
        deb_root
            .join("usr/share/dev.scoop.bundle-test/assets/empty")
            .is_dir()
    );
    let rpm_root = root.join("rpm-install");
    let rpm_db = root.join("rpm-db");
    fs::create_dir(&rpm_root).unwrap();
    let rpm = || {
        let mut command = Command::new("rpm");
        command.arg("--dbpath").arg(&rpm_db);
        command
    };
    successful(rpm().arg("--initdb"));
    let rpm_install = |package: &Path| {
        successful(
            rpm()
                .args([
                    "--upgrade",
                    "--nodeps",
                    "--noscripts",
                    "--badreloc",
                    "--relocate",
                ])
                .arg(format!("/={}", rpm_root.display()))
                .arg(package),
        );
    };
    rpm_install(&packages["rpm"]);
    run(&rpm_root.join("usr/bin/bundle-test"));
    for value in records(successful(&mut command(&[]))).values() {
        assert_eq!(value["reused"], true);
    }
    fs::write(&config, configuration.replace("1.2.3", "1.2.4")).unwrap();
    let upgrade = records(successful(&mut command(&["deb", "rpm"])));
    successful(
        dpkg()
            .arg("--install")
            .arg(upgrade["deb"]["output"].as_str().unwrap()),
    );
    rpm_install(Path::new(upgrade["rpm"]["output"].as_str().unwrap()));
    run(&deb_root.join("usr/bin/bundle-test"));
    run(&rpm_root.join("usr/bin/bundle-test"));
    successful(dpkg().args(["--remove", "bundle-test"]));
    successful(rpm().args(["--erase", "--noscripts", "bundle-test"]));
    assert!(!deb_root.join("usr/bin/bundle-test").exists());
    assert!(!rpm_root.join("usr/bin/bundle-test").exists());
    assert_eq!(fs::read(&executable).unwrap(), original);
}
