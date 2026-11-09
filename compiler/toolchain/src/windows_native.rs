//! Native C/C++ compilers in the selected Windows SDK/CRT environment.
use std::path::{Path, PathBuf};
use std::process::Command;

use object::{Object, ObjectSection, ObjectSymbol};
use scoop_manifest::NativeSourceLanguage;
use scoop_process::CommandExt;

use crate::native_driver::{NativeDriverDiscovery, NativeDriverKind};
use crate::{ToolchainError, ValidatedCBridgeToolchainInvocation};

mod probe;

pub(crate) fn is_windows_target(c: &ValidatedCBridgeToolchainInvocation) -> bool {
    c.target_os() == scoop_target_data::TargetOs::Windows
}

pub(crate) fn paired_driver(
    c: &ValidatedCBridgeToolchainInvocation,
    language: NativeSourceLanguage,
    selected_c: Option<&Path>,
) -> PathBuf {
    if let Some(driver) = selected_c {
        if language == NativeSourceLanguage::C
            || (c.native_environment() == scoop_target_data::NativeEnvironment::MicrosoftUcrt
                && kind(driver).microsoft())
        {
            return driver.to_owned();
        }
        return driver.with_file_name(crate::host_executable_name("clang++"));
    }
    let name = if c.native_environment() == scoop_target_data::NativeEnvironment::MicrosoftUcrt {
        "clang-cl"
    } else if language == NativeSourceLanguage::Cxx {
        "clang++"
    } else {
        "clang"
    };
    c.compiler_driver()
        .with_file_name(crate::host_executable_name(name))
}

pub(crate) fn kind(driver: &Path) -> NativeDriverKind {
    let name = driver
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if name.eq_ignore_ascii_case("cl") {
        NativeDriverKind::Msvc
    } else if name.eq_ignore_ascii_case("clang-cl") {
        NativeDriverKind::ClangCl
    } else {
        NativeDriverKind::Gnu
    }
}

pub(crate) fn input(command: &mut Command, kind: NativeDriverKind, path: &Path) {
    let path = dunce::simplified(path);
    // Use Clang's GNU argument parser for filenames so /Users is not /U.
    if kind == NativeDriverKind::ClangCl {
        command.arg(joined("/clang:", path));
    } else {
        command.arg(path);
    }
}

pub(crate) fn command(
    c: &ValidatedCBridgeToolchainInvocation,
    driver: &Path,
    kind: NativeDriverKind,
    language: NativeSourceLanguage,
    preprocessed: bool,
) -> Command {
    let environment = c
        .windows_environment()
        .expect("selected Windows native environment");
    if !kind.microsoft() {
        let mut command = environment.bare_command(driver);
        if !preprocessed
            && language == NativeSourceLanguage::Cxx
            && c.native_environment() != scoop_target_data::NativeEnvironment::MicrosoftUcrt
        {
            // libc++ wrappers use include_next, so their roots precede the C SDK.
            environment.configure_mingw_cxx(&mut command);
        }
        environment.configure(
            &mut command,
            crate::windows_c::target_triple(c.target().id()).expect("validated Windows target"),
            c.native_environment() == scoop_target_data::NativeEnvironment::MicrosoftUcrt,
            preprocessed,
        );
        return command;
    }
    let mut command = environment.bare_command(driver);
    command.args([
        "/nologo",
        "/X",
        "/MD",
        "/utf-8",
        "/D_WIN32_WINNT=0x0a00",
        "/DWINVER=0x0a00",
    ]);
    if kind == NativeDriverKind::ClangCl {
        command.args([
            environment.machine.clang_cl_target_arg(),
            environment.machine.clang_cl_arch_arg(),
        ]);
    } else {
        command.arg("/Zc:preprocessor");
    }
    if language == NativeSourceLanguage::Cxx {
        command.arg("/EHsc");
    }
    if !preprocessed {
        for include in &environment.includes {
            command
                .arg(if kind == NativeDriverKind::ClangCl {
                    "/imsvc"
                } else {
                    "/I"
                })
                .arg(include);
        }
    }
    command
}

pub(crate) fn discover(
    c: &ValidatedCBridgeToolchainInvocation,
    language: NativeSourceLanguage,
    driver: &Path,
) -> Result<NativeDriverDiscovery, ToolchainError> {
    let environment = c.windows_environment().map_err(error)?;
    let driver = crate::paths::find_program(driver.as_os_str(), &environment.search_path)?;
    let kind = kind(&driver);
    if kind.microsoft()
        && c.native_environment() != scoop_target_data::NativeEnvironment::MicrosoftUcrt
    {
        return Err(error("cl/clang-cl require the Microsoft UCRT target"));
    }
    let directory = tempfile::Builder::new()
        .prefix("scoop-native-probe-")
        .tempdir()
        .map_err(error)?;
    let source = directory
        .path()
        .join(if language == NativeSourceLanguage::Cxx {
            "probe.cpp"
        } else {
            "probe.c"
        });
    let object = directory.path().join("probe.obj");
    std::fs::write(
        &source,
        probe::source(c.native_environment(), environment.machine, language),
    )
    .map_err(error)?;
    let mut compiler = command(c, &driver, kind, language, false);
    compiler.arg(if kind.microsoft() {
        if language == NativeSourceLanguage::Cxx {
            "/std:c++20"
        } else {
            "/std:c11"
        }
    } else if language == NativeSourceLanguage::Cxx {
        "-std=c++20"
    } else {
        "-std=c11"
    });
    if kind.microsoft() {
        compiler.arg("/c").arg(joined("/Fo", &object));
    } else {
        compiler.arg("-c").arg("-o").arg(&object);
    }
    input(&mut compiler, kind, &source);
    let output = compiler.scoop_output().map_err(error)?;
    if !output.status.success() {
        return Err(error(format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    let bytes = std::fs::read(object).map_err(error)?;
    let file = object::File::parse(bytes.as_slice()).map_err(error)?;
    if file.architecture() != environment.machine.object_architecture()
        || file.format() != object::BinaryFormat::Coff
        || file.kind() != object::ObjectKind::Relocatable
    {
        return Err(error(format!(
            "native compiler must emit {} COFF objects",
            environment.machine.name()
        )));
    }
    let symbol = file
        .symbol_by_name("scoop_native_configuration")
        .ok_or_else(|| error("missing native probe configuration"))?;
    let section = file
        .section_by_index(
            symbol
                .section_index()
                .ok_or_else(|| error("missing probe section"))?,
        )
        .map_err(error)?;
    let data = section
        .data_range(symbol.address(), 16)
        .map_err(error)?
        .ok_or_else(|| error("truncated native probe configuration"))?;
    Ok(NativeDriverDiscovery {
        driver: scoop_protocol::HostPathCarrier::from_path(&driver).map_err(error)?,
        configuration: *scoop_wire::sha256(data).as_array(),
        kind,
        family: if kind == NativeDriverKind::Msvc {
            crate::native_driver::NativeCompilerFamily::Msvc
        } else {
            crate::native_driver::NativeCompilerFamily::Clang
        },
    })
}

pub(crate) fn dependencies(
    command: &mut Command,
    kind: NativeDriverKind,
    path: &Path,
    target: &str,
) {
    match kind {
        NativeDriverKind::Msvc => {
            command.arg("/sourceDependencies").arg(path);
        }
        NativeDriverKind::ClangCl => {
            command
                .args(["/clang:-MD", "/clang:-MT"])
                .arg(format!("/clang:{target}"))
                .arg("/clang:-MF")
                .arg(joined("/clang:", path));
        }
        NativeDriverKind::Gnu => {
            command.args(["-MD", "-MT", target, "-MF"]).arg(path);
        }
    }
}

pub(crate) fn read_dependencies(
    kind: NativeDriverKind,
    path: &Path,
    target: &str,
) -> Result<Vec<PathBuf>, ToolchainError> {
    let text = std::fs::read_to_string(path).map_err(error)?;
    if kind == NativeDriverKind::Msvc {
        #[derive(serde::Deserialize)]
        struct Dependencies {
            #[serde(rename = "Data")]
            data: Source,
        }
        #[derive(serde::Deserialize)]
        struct Source {
            #[serde(rename = "Source")]
            source: PathBuf,
            #[serde(rename = "Includes")]
            includes: Vec<PathBuf>,
        }
        let dependencies: Dependencies =
            serde_json::from_str(text.trim_start_matches('\u{feff}')).map_err(error)?;
        Ok(std::iter::once(dependencies.data.source)
            .chain(dependencies.data.includes)
            .collect())
    } else {
        scoop_process::parse_make_dependencies(&text, target)
            .ok_or_else(|| error("invalid native compiler dependency output"))
    }
}

pub(crate) fn joined(prefix: &str, path: &Path) -> std::ffi::OsString {
    let mut value = std::ffi::OsString::from(prefix);
    value.push(path);
    value
}

fn error(value: impl std::fmt::Display) -> ToolchainError {
    ToolchainError(format!("Windows native compiler: {value}"))
}
