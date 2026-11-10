use std::ffi::OsString;

use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WindowsCEnvironment {
    pub(crate) machine: crate::windows_machine::WindowsToolchainMachine,
    pub(crate) native_sysroot: Option<PathBuf>,
    pub(crate) includes: Vec<PathBuf>,
    pub(crate) libraries: Vec<PathBuf>,
    pub(crate) search_path: OsString,
    pub(crate) system_root: Option<OsString>,
    pub(crate) bundled_mingw: Option<BundledMinGw>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub(crate) struct BundledMinGw {
    #[serde(with = "crate::build_record::values::path")]
    pub(crate) resource_dir: PathBuf,
    #[serde(with = "crate::build_record::values::paths")]
    pub(crate) cxx_includes: Vec<PathBuf>,
}

impl WindowsCEnvironment {
    pub(super) fn resolve(
        target: LirTargetProfile,
        compiler: &Path,
        native_sysroot: Option<&Path>,
        bundled_llvm: Option<&Path>,
    ) -> Result<Self, ToolchainError> {
        let machine = crate::windows_machine::WindowsToolchainMachine::select(target)?;
        let mut environment = Self {
            machine,
            native_sysroot: native_sysroot
                .map(std::path::absolute)
                .transpose()
                .map_err(error)?,
            includes: Vec::new(),
            libraries: Vec::new(),
            search_path: std::env::var_os("PATH").unwrap_or_default(),
            system_root: std::env::var_os("SystemRoot"),
            bundled_mingw: None,
        };
        if crate::windows_c::native_environment(target.id())?
            != scoop_target_data::NativeEnvironment::MicrosoftUcrt
        {
            if environment.native_sysroot.is_none() {
                environment.native_sysroot = Some(compiler.parent().and_then(Path::parent)
                    .ok_or_else(|| error("cannot infer LLVM-MinGW prefix; provide an explicit native sysroot"))?.to_owned());
            }
            let root = environment
                .native_sysroot
                .as_ref()
                .expect("native prefix was selected");
            environment.libraries.extend([
                root.join(machine.llvm_mingw_target_directory()).join("lib"),
                root.join("lib"),
            ]);
            if let Some(llvm) = bundled_llvm {
                let shared = llvm.with_file_name("mingw").join("include");
                environment.includes = vec![
                    root.join("include"),
                    llvm.join("lib/clang/22/include"),
                    shared.clone(),
                ];
                let resources = BundledMinGw {
                    resource_dir: root.join("lib/clang/22"),
                    cxx_includes: vec![root.join("include/c++/v1"), shared.join("c++/v1")],
                };
                environment.bundled_mingw = Some(resources);
            }
        } else if let Some(root) = &environment.native_sysroot {
            let config = root.join("msvc.toml");
            let contents = std::fs::read_to_string(&config)
                .map_err(|err| error(format!("{}: {err}", config.display())))?;
            let MsvcConfig { msvc } = toml::from_str(&contents).map_err(error)?;
            let tools = root.join(msvc.vc_tools);
            let sdk = root.join(msvc.windows_sdk);
            let include = sdk.join("Include").join(&msvc.sdk_version);
            let lib = sdk.join("Lib").join(&msvc.sdk_version);
            environment.includes = vec![
                tools.join("include"),
                include.join("ucrt"),
                include.join("shared"),
                include.join("um"),
                include.join("winrt"),
            ];
            environment.libraries = vec![
                tools.join("lib").join(machine.msvc_library_directory()),
                lib.join("ucrt").join(machine.msvc_library_directory()),
                lib.join("um").join(machine.msvc_library_directory()),
            ];
        } else {
            environment.discover_microsoft()?;
        }
        Ok(environment)
    }

    #[cfg(windows)]
    fn discover_microsoft(&mut self) -> Result<(), ToolchainError> {
        let tool = find_msvc_tools::find_tool(self.machine.msvc_discovery_target(), "cl.exe")
            .ok_or_else(|| error("no Microsoft C++ Build Tools and Windows SDK found; provide an explicit native sysroot"))?;
        // find-msvc-tools reuses an initialized developer shell without
        // returning overrides. Capture its effective environment before the
        // compiler command clears inherited variables.
        let inherited = ["INCLUDE", "LIB", "PATH"]
            .into_iter()
            .filter_map(|name| std::env::var_os(name).map(|value| (OsString::from(name), value)));
        for (name, value) in inherited.chain(tool.env().into_iter().cloned()) {
            if name.eq_ignore_ascii_case("INCLUDE") {
                self.includes = search_directories(&value)?;
            } else if name.eq_ignore_ascii_case("LIB") {
                self.libraries = search_directories(&value)?;
            } else if name.eq_ignore_ascii_case("PATH") {
                self.search_path = value;
            }
        }
        Ok(())
    }

    #[cfg(not(windows))]
    fn discover_microsoft(&mut self) -> Result<(), ToolchainError> {
        Err(error(
            "Microsoft cross-compilation requires a native sysroot containing msvc.toml",
        ))
    }

    pub(crate) fn command(&self, compiler: &Path, triple: &str, microsoft: bool) -> Command {
        let mut command = self.bare_command(compiler);
        self.configure(&mut command, triple, microsoft, false);
        command
    }

    pub(crate) fn bare_command(&self, compiler: &Path) -> Command {
        let mut command = Command::new(compiler);
        command
            .env_clear()
            .env("LC_ALL", "C")
            .env("LANG", "C")
            .env("TZ", "UTC")
            .env("PATH", &self.search_path)
            .env("TEMP", std::env::temp_dir())
            .env("TMP", std::env::temp_dir())
            .env("TMPDIR", std::env::temp_dir());
        if let Some(root) = &self.system_root {
            command.env("SystemRoot", root);
        }
        command
    }

    pub(crate) fn configure(
        &self,
        command: &mut Command,
        triple: &str,
        microsoft: bool,
        preprocessed: bool,
    ) {
        command.args([
            "-target",
            triple,
            self.machine.clang_arch_flag(),
            "-funwind-tables",
            "-fasynchronous-unwind-tables",
        ]);
        if !preprocessed {
            command.args(["-D_WIN32_WINNT=0x0a00", "-DWINVER=0x0a00"]);
        }
        if microsoft {
            command.arg("-fms-runtime-lib=dll");
            if !preprocessed {
                command.arg("-nostdlibinc");
                for include in &self.includes {
                    // Preserve Clang's builtin atomic/intrinsic headers before
                    // the explicitly selected SDK and CRT, as in driver discovery.
                    command.arg("-idirafter").arg(dunce::simplified(include));
                }
            }
        } else if let Some(root) = &self.native_sysroot {
            // Clang joins sysroot children with '/', which is not a separator
            // in Windows verbatim paths returned by canonicalize.
            command.arg("--sysroot").arg(dunce::simplified(root));
            if let Some(resources) = &self.bundled_mingw {
                command
                    .arg("-resource-dir")
                    .arg(dunce::simplified(&resources.resource_dir));
                if !preprocessed {
                    command.arg("-nostdinc");
                    for include in &self.includes {
                        command.arg("-isystem").arg(dunce::simplified(include));
                    }
                }
            }
        }
    }

    pub(crate) fn configure_mingw_cxx(&self, command: &mut Command) {
        if let Some(resources) = &self.bundled_mingw {
            // Explicit headers make -stdlib unused during object compilation;
            // the final linker still selects libc++.
            command.arg("-nostdinc++");
            for include in &resources.cxx_includes {
                command.arg("-isystem").arg(dunce::simplified(include));
            }
        } else {
            command.arg("-stdlib=libc++");
        }
    }
}

#[cfg(windows)]
fn search_directories(value: &std::ffi::OsStr) -> Result<Vec<PathBuf>, ToolchainError> {
    std::env::split_paths(value)
        .filter(|path| !path.as_os_str().is_empty())
        .map(|path| std::path::absolute(path).map_err(error))
        .collect()
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct MsvcConfig {
    msvc: MsvcResources,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct MsvcResources {
    vc_tools: PathBuf,
    windows_sdk: PathBuf,
    sdk_version: String,
}
