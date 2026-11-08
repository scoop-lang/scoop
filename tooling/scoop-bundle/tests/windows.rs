#![cfg(windows)]

use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    os::windows::process::CommandExt,
    path::{Path, PathBuf},
    process::{Command, Output},
    thread,
    time::{Duration, Instant},
};

fn successful(command: &mut Command) -> Output {
    let output = command.output().unwrap();
    if !output.status.success() {
        let arguments = command.get_args().collect::<Vec<_>>();
        if let Some(pair) = arguments.windows(2).find(|pair| pair[0] == "/l*vx!")
            && let Ok(bytes) = fs::read(pair[1])
        {
            let text = String::from_utf16_lossy(
                &bytes
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|bytes| u16::from_le_bytes(*bytes))
                    .collect::<Vec<_>>(),
            );
            eprintln!("Windows Installer log:\n{text}");
        }
    }
    assert!(
        output.status.success(),
        "{command:?}: {}\n{}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn msiexec(root: &Path, label: &str) -> Command {
    let mut command = Command::new("msiexec.exe");
    command.arg("/l*vx!").arg(root.join(format!("{label}.log")));
    command
}

fn powershell(script: &str) -> String {
    let output = successful(Command::new("powershell.exe").args([
        "-NoLogo",
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        &format!("[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false); {script}"),
    ]));
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn product_code(identifier: &str) -> String {
    powershell(&format!(
        "if (Test-Path -LiteralPath 'HKCU:\\Software\\ScoopBundle\\{identifier}') {{ (Get-ItemProperty -LiteralPath 'HKCU:\\Software\\ScoopBundle\\{identifier}').ProductCode }}; exit 0"
    ))
}

fn verify_signature(path: &Path, thumbprint: &str) {
    let path = path.to_str().unwrap().replace('\'', "''");
    assert_eq!(
        powershell(&format!(
            "$signature = Get-AuthenticodeSignature -LiteralPath '{path}'; '{{0}}:{{1}}' -f $signature.Status, $signature.SignerCertificate.Thumbprint"
        )),
        format!("Valid:{}", thumbprint.to_ascii_uppercase())
    );
}

fn verify_payload_signatures(root: &Path, thumbprint: &str) {
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|value| {
            value.eq_ignore_ascii_case("exe") || value.eq_ignore_ascii_case("dll")
        }) {
            verify_signature(&path, thumbprint);
        }
    }
}

fn compile(root: &Path) -> PathBuf {
    let vendor = root.join("vendor");
    fs::create_dir(&vendor).unwrap();
    for (name, code, libraries) in [
        (
            "Side",
            "__declspec(dllexport) int side(void) { return 41; }",
            &[][..],
        ),
        (
            "Answer",
            "__declspec(dllimport) int side(void); __declspec(dllexport) int answer(void) { return side()+1; }",
            &["Side"][..],
        ),
        (
            "Delay",
            "__declspec(dllexport) int delayed(void) { return 5; }",
            &[][..],
        ),
        (
            "ForwardTarget",
            "__declspec(dllexport) int forwarded(void) { return 6; }",
            &[][..],
        ),
    ] {
        let source = root.join(format!("{name}.c"));
        fs::write(&source, code).unwrap();
        let mut command = Command::new("clang-cl");
        command
            .current_dir(root)
            .args(["/nologo", "/MD", "/LD"])
            .arg(source)
            .arg("/link")
            .arg(format!(
                "/OUT:{}",
                vendor.join(format!("{name}.dll")).display()
            ))
            .arg(format!(
                "/IMPLIB:{}",
                vendor.join(format!("{name}.lib")).display()
            ));
        for library in libraries {
            command.arg(vendor.join(format!("{library}.lib")));
        }
        successful(&mut command);
    }
    let definition = root.join("Forward.def");
    fs::write(
        &definition,
        "LIBRARY Forward\nEXPORTS\n forwarded=ForwardTarget.forwarded\n",
    )
    .unwrap();
    let forward_source = root.join("Forward.c");
    fs::write(&forward_source, "int forward_data = 0;\n").unwrap();
    successful(
        Command::new("clang-cl")
            .current_dir(root)
            .args(["/nologo", "/c", "/Zl"])
            .arg(forward_source),
    );
    successful(
        Command::new("lld-link")
            .args(["/DLL", "/NOENTRY", "/MACHINE:X64"])
            .arg(root.join("Forward.obj"))
            .arg(format!("/DEF:{}", definition.display()))
            .arg(format!("/OUT:{}", vendor.join("Forward.dll").display()))
            .arg(format!("/IMPLIB:{}", vendor.join("Forward.lib").display())),
    );
    let source = root.join("main.c");
    fs::write(&source, r#"
#define UNICODE
#include <windows.h>
#include <stdio.h>
#include <wchar.h>
__declspec(dllimport) int answer(void);
__declspec(dllimport) int delayed(void);
__declspec(dllimport) int forwarded(void);
int wmain(int argc, wchar_t **argv) {
    wchar_t path[32768]; char message[256] = {0}, argument[4096] = {0};
    if (!GetModuleFileNameW(NULL, path, 32768)) return 2;
    wchar_t *slash = wcsrchr(path, L'\\');
    if (!slash) return 3;
    wcscpy_s(slash + 1, 32768 - (slash + 1 - path), L"resources\\assets\\message.txt");
    FILE *file = NULL;
    if (_wfopen_s(&file, path, L"rb") || !file) return 4;
    if (!fgets(message, sizeof(message), file)) return 5;
    fclose(file);
    if (argc > 1 && !WideCharToMultiByte(CP_UTF8, 0, argv[1], -1, argument, sizeof(argument), NULL, NULL)) return 6;
    printf("%s:%d:%d:%d:%s\n", message, answer(), delayed(), forwarded(), argument);
    return 0;
}
"#).unwrap();
    let executable = root.join("original program.exe");
    successful(
        Command::new("clang-cl")
            .current_dir(root)
            .args(["/nologo", "/MD"])
            .arg(source)
            .arg("/link")
            .arg(format!("/OUT:{}", executable.display()))
            .arg(vendor.join("Answer.lib"))
            .arg(vendor.join("Delay.lib"))
            .arg(vendor.join("Forward.lib"))
            .args(["delayimp.lib", "/DELAYLOAD:Delay.dll"]),
    );
    executable
}

fn wait_removed(path: &Path) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while path.exists() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(100));
    }
    assert!(
        !path.exists(),
        "uninstall did not remove {}",
        path.display()
    );
}

struct Installations {
    nsis: PathBuf,
    identifier: String,
}
impl Drop for Installations {
    fn drop(&mut self) {
        let uninstaller = self.nsis.join("uninstall.exe");
        if uninstaller.is_file() {
            let _ = Command::new(&uninstaller).arg("/S").status();
        }
        let code = product_code(&self.identifier);
        if !code.is_empty() {
            let _ = Command::new("msiexec.exe")
                .args(["/x", &code, "/qn", "/norestart"])
                .status();
        }
    }
}

#[test]
fn installers_deploy_dlls_and_resources_upgrade_repair_and_preserve_user_files() {
    let temporary = tempfile::Builder::new()
        .prefix("scoop bundle 中文 ")
        .tempdir()
        .unwrap();
    let root = temporary.path();
    let executable = compile(root);
    let original = fs::read(&executable).unwrap();
    fs::create_dir_all(root.join("assets/empty")).unwrap();
    fs::write(root.join("assets/message.txt"), "resource").unwrap();
    fs::write(root.join("assets/obsolete.txt"), "first version only").unwrap();
    fs::write(root.join("assets/$literal.txt"), "dollar filename").unwrap();
    fs::write(root.join("assets/${NSISDIR}.txt"), "literal macro filename").unwrap();
    fs::write(
        root.join("assets/dos-header.txt"),
        "MZ is also ordinary text",
    )
    .unwrap();
    image::RgbaImage::from_pixel(256, 256, image::Rgba([32, 96, 160, 255]))
        .save(root.join("icon.png"))
        .unwrap();
    let identifier = format!("dev.scoop.bundle-test-{}", std::process::id());
    let upgrade = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_DNS, identifier.as_bytes());
    let mut configuration = format!(
        r#"
[bundle]
schema = 1
identifier = "{identifier}"
name = "bundle-test"
display-name = "Bundle 中文 $(literal)"
version = "1.2.3"
publisher = "Scoop Tests & Friends"
icons = ["icon.png"]
library-search-paths = ["vendor"]
[[bundle.files]]
from = "assets"
to = "assets"
kind = "resource"
[[bundle.files]]
from = "vendor/Answer.dll"
to = "Answer.dll"
kind = "library"
[bundle.windows]
formats = ["nsis", "msi"]
vc-runtime = "system"
[bundle.windows.msi]
upgrade-code = "{upgrade}"
"#
    );
    let signing = std::env::var("SCOOP_BUNDLE_TEST_SIGNING_THUMBPRINT").ok();
    if let Some(thumbprint) = &signing {
        let timestamp = std::env::var("SCOOP_BUNDLE_TEST_TIMESTAMP_URL")
            .expect("signed installer tests require SCOOP_BUNDLE_TEST_TIMESTAMP_URL");
        configuration.push_str(&format!(
            "\n[bundle.windows.signing]\ncertificate-thumbprint = {}\ntimestamp-url = {}\n",
            toml::Value::String(thumbprint.clone()),
            toml::Value::String(timestamp),
        ));
    }
    let config = root.join("Bundle.toml");
    fs::write(&config, &configuration).unwrap();
    let bundle = || {
        let mut command = Command::new(env!("CARGO_BIN_EXE_scoop-bundle"));
        command
            .current_dir(root)
            .arg("--executable")
            .arg(&executable)
            .args([
                "--config",
                "Bundle.toml",
                "--target",
                "x86_64-pc-windows-msvc-ucrt",
                "--message-format",
                "json",
            ]);
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
    let first = records(successful(&mut bundle()));
    assert_eq!(first.len(), 2);
    let mut packages = BTreeMap::new();
    for (format, record) in &first {
        let saved = root.join(format!(
            "first.{}",
            if format == "nsis" { "exe" } else { "msi" }
        ));
        fs::copy(record["output"].as_str().unwrap(), &saved).unwrap();
        if let Some(thumbprint) = &signing {
            verify_signature(&saved, thumbprint);
        }
        packages.insert(format.as_str(), saved);
    }
    for record in records(successful(&mut bundle())).values() {
        assert_eq!(record["reused"], true);
    }
    fs::write(&config, configuration.replace("1.2.3", "1.2.4")).unwrap();
    fs::write(root.join("assets/message.txt"), "updated").unwrap();
    fs::remove_file(root.join("assets/obsolete.txt")).unwrap();
    let second = records(successful(&mut bundle()));
    let nsis_root = root.join("installed NSIS 中文");
    let msi_root = root.join("installed MSI 中文");
    let _cleanup = Installations {
        nsis: nsis_root.clone(),
        identifier: identifier.clone(),
    };
    let run = |installed: &Path, message: &str| {
        let output = successful(
            Command::new(installed.join("bundle-test.exe"))
                .current_dir(std::env::temp_dir())
                .arg("a b 中文"),
        );
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            format!("{message}:42:5:6:a b 中文\r\n")
        );
        for library in [
            "Answer.dll",
            "Side.dll",
            "Delay.dll",
            "Forward.dll",
            "ForwardTarget.dll",
        ] {
            assert!(installed.join(library).is_file(), "{library}");
        }
        assert!(installed.join("resources/assets/empty").is_dir());
        assert!(installed.join("resources/assets/$literal.txt").is_file());
        assert!(installed.join("resources/assets/${NSISDIR}.txt").is_file());
    };
    successful(
        Command::new(&packages["nsis"])
            .arg("/S")
            .raw_arg(format!("/D={}", nsis_root.display())),
    );
    run(&nsis_root, "resource");
    if let Some(thumbprint) = &signing {
        verify_payload_signatures(&nsis_root, thumbprint);
    }
    fs::write(nsis_root.join("user-data.txt"), "keep me").unwrap();
    let denied = msiexec(root, "reject-format")
        .arg("/i")
        .arg(&packages["msi"])
        .args(["/qn", "/norestart"])
        .output()
        .unwrap();
    assert!(
        !denied.status.success(),
        "MSI must reject replacing an NSIS installation"
    );
    successful(Command::new(second["nsis"]["output"].as_str().unwrap()).arg("/S"));
    run(&nsis_root, "updated");
    assert!(!nsis_root.join("resources/assets/obsolete.txt").exists());
    assert!(nsis_root.join("user-data.txt").is_file());
    successful(Command::new(nsis_root.join("uninstall.exe")).arg("/S"));
    wait_removed(&nsis_root.join("uninstall.exe"));
    assert!(!nsis_root.join("bundle-test.exe").exists());
    assert!(nsis_root.join("user-data.txt").is_file());
    successful(
        msiexec(root, "install")
            .arg("/i")
            .arg(&packages["msi"])
            .raw_arg(format!("INSTALLFOLDER=\"{}\"", msi_root.display()))
            .args(["/qn", "/norestart"]),
    );
    run(&msi_root, "resource");
    if let Some(thumbprint) = &signing {
        verify_payload_signatures(&msi_root, thumbprint);
    }
    let first_product = product_code(&identifier);
    assert!(!first_product.is_empty());
    fs::write(msi_root.join("user-data.txt"), "keep me").unwrap();
    fs::remove_file(msi_root.join("resources/assets/message.txt")).unwrap();
    successful(
        msiexec(root, "repair")
            .arg("/fa")
            .arg(&first_product)
            .args(["/qn", "/norestart"]),
    );
    run(&msi_root, "resource");
    successful(
        msiexec(root, "upgrade")
            .arg("/i")
            .arg(second["msi"]["output"].as_str().unwrap())
            .args(["/qn", "/norestart"]),
    );
    run(&msi_root, "updated");
    assert!(!msi_root.join("resources/assets/obsolete.txt").exists());
    assert!(msi_root.join("user-data.txt").is_file());
    let next_product = product_code(&identifier);
    assert_ne!(first_product, next_product);
    successful(msiexec(root, "uninstall").args(["/x", &next_product, "/qn", "/norestart"]));
    assert!(!msi_root.join("bundle-test.exe").exists());
    assert!(msi_root.join("user-data.txt").is_file());
    assert_eq!(fs::read(&executable).unwrap(), original);
}
