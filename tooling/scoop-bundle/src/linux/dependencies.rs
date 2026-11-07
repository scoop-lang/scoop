use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, ensure};

use crate::{
    cache::Inputs,
    deploy::{self, Plan},
    project::Input,
    tools::Tools,
};

pub struct Code {
    source: PathBuf,
    destination: PathBuf,
    libraries: Vec<String>,
    soname: Option<String>,
    rpaths: Vec<String>,
    dynamic: bool,
    entry: u64,
    changes: Vec<(String, PathBuf)>,
}

impl Code {
    fn library_name(&self) -> Result<&std::ffi::OsStr> {
        self.soname
            .as_deref()
            .map(Path::new)
            .unwrap_or(&self.destination)
            .file_name()
            .context("ELF library has no filename")
    }
}

fn inspect(source: &Path, destination: &Path) -> Result<Option<Code>> {
    let mut file = fs::File::open(source)?;
    let mut magic = [0; 4];
    if file.read(&mut magic)? != 4 || &magic != b"\x7fELF" {
        return Ok(None);
    }
    let bytes = fs::read(source)?;
    let elf = goblin::elf::Elf::parse(&bytes)
        .with_context(|| format!("read ELF {}", source.display()))?;
    ensure!(
        elf.is_64 && elf.header.e_machine == goblin::elf::header::EM_X86_64,
        "{} is not an x86_64 ELF image",
        source.display()
    );
    ensure!(
        matches!(
            elf.header.e_type,
            goblin::elf::header::ET_EXEC | goblin::elf::header::ET_DYN
        ),
        "{} is not a linked ELF image",
        source.display()
    );
    Ok(Some(Code {
        source: source.into(),
        destination: destination.into(),
        libraries: elf.libraries.iter().map(|v| (*v).into()).collect(),
        soname: elf.soname.map(str::to_owned),
        rpaths: elf
            .runpaths
            .iter()
            .chain(&elf.rpaths)
            .flat_map(|value| value.split(':'))
            .map(str::to_owned)
            .collect(),
        dynamic: elf.dynamic.is_some(),
        entry: elf.entry,
        changes: Vec::new(),
    }))
}

fn baseline(name: &str) -> bool {
    matches!(
        name,
        "libc.so.6"
            | "libm.so.6"
            | "libpthread.so.0"
            | "libdl.so.2"
            | "librt.so.1"
            | "libresolv.so.2"
            | "libutil.so.1"
            | "ld-linux-x86-64.so.2"
            | "libc.musl-x86_64.so.1"
    )
}

fn system(path: &Path) -> bool {
    ["/usr/lib", "/usr/lib64", "/lib", "/lib64"]
        .iter()
        .any(|prefix| path.starts_with(prefix))
}

fn expand_origin(directory: &str, origin: &Path) -> Option<PathBuf> {
    let normalized = directory.replace("${ORIGIN}", "$ORIGIN");
    let mut path = OsString::new();
    for (index, part) in normalized.split("$ORIGIN").enumerate() {
        if part.contains('$') {
            return None;
        }
        if index != 0 {
            path.push(origin);
        }
        path.push(part);
    }
    Some(path.into())
}

fn system_libraries(
    inputs: &mut Inputs,
    tools: &mut Tools<'_>,
) -> Result<BTreeMap<String, PathBuf>> {
    inputs.watch(Path::new("/etc/ld.so.cache"))?;
    let tool = tools.find("ldconfig", inputs)?;
    let output = Command::new(&tool)
        .arg("-p")
        .output()
        .context("query ldconfig")?;
    ensure!(
        output.status.success(),
        "ldconfig failed ({}): {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).context("ldconfig output must be UTF-8")?;
    let mut result = BTreeMap::new();
    for line in text.lines() {
        if let Some((left, right)) = line.split_once(" => ")
            && left.contains("x86-64")
            && let Some(name) = left.split_whitespace().next()
        {
            result
                .entry(name.into())
                .or_insert_with(|| PathBuf::from(right.trim()));
        }
    }
    Ok(result)
}

pub fn collect(
    input: &Input,
    plan: &mut Plan,
    appimage: bool,
    inputs: &mut Inputs,
    tools: &mut Tools<'_>,
) -> Result<Vec<Code>> {
    let settings = &input.settings;
    let library_root = PathBuf::from(format!("usr/lib/{}/lib", settings.config.identifier));
    for name in &settings.config.linux.appimage.system_libraries {
        ensure!(
            !name.is_empty()
                && !name.contains(['/', '\\', '\0'])
                && !name.chars().any(char::is_whitespace),
            "appimage.system-libraries requires SONAMEs"
        );
    }
    let mut files = Vec::<Code>::new();
    let mut scanned = BTreeSet::new();
    let mut system_cache = None;
    let mut position = 0;
    loop {
        for (source, destination) in plan.source_files() {
            if scanned.insert(destination.clone())
                && let Some(code) = inspect(&source, &destination)?
            {
                files.push(code);
            }
        }
        if position == files.len() {
            break;
        }
        let source = files[position].source.clone();
        let origin = source.parent().context("ELF input has no parent")?;
        for name in files[position].libraries.clone() {
            if appimage
                && (baseline(&name)
                    || settings
                        .config
                        .linux
                        .appimage
                        .system_libraries
                        .contains(&name))
            {
                continue;
            }
            let basename = Path::new(&name)
                .file_name()
                .context("empty ELF library name")?;
            let matches: Vec<_> = files
                .iter()
                .filter(|code| {
                    code.soname.as_deref() == Some(&name)
                        || code.source.file_name() == Some(basename)
                })
                .collect();
            let origins: BTreeSet<_> = matches
                .iter()
                .map(|code| code.source.canonicalize())
                .collect::<std::io::Result<_>>()?;
            ensure!(
                origins.len() <= 1,
                "{}: ambiguous private dependency {name}",
                source.display()
            );
            let destination = if let Some(code) = matches.first() {
                code.destination.clone()
            } else {
                let mut candidates: Vec<_> = settings
                    .config
                    .library_search_paths
                    .iter()
                    .map(|p| settings.path(p).join(basename))
                    .collect();
                for directory in &files[position].rpaths {
                    if let Some(directory) = expand_origin(directory, origin) {
                        candidates.push(origin.join(directory).join(basename));
                    }
                }
                if name.contains('/') {
                    candidates.push(origin.join(&name));
                }
                let mut resolved = None;
                for candidate in candidates {
                    inputs.watch(&candidate)?;
                    if candidate.is_file() {
                        resolved = Some(candidate.canonicalize()?);
                        break;
                    }
                }
                if resolved.is_none() {
                    if system_cache.is_none() {
                        system_cache = Some(system_libraries(inputs, tools)?);
                    }
                    if let Some(path) = system_cache
                        .as_ref()
                        .and_then(|libraries| libraries.get(&name))
                    {
                        inputs.watch(path)?;
                        resolved = Some(path.canonicalize()?);
                    }
                }
                let resolved = resolved.with_context(||format!("{}: missing ELF dependency {name}; declare private files/library-search-paths or install the build host's runtime library",source.display()))?;
                if !appimage && system(&resolved) {
                    continue;
                }
                let existing = plan
                    .source_files()
                    .into_iter()
                    .find_map(|(source, destination)| {
                        (source.canonicalize().ok().as_ref() == Some(&resolved))
                            .then_some(destination)
                    });
                if let Some(destination) = existing {
                    destination
                } else {
                    let destination =
                        library_root.join(resolved.file_name().context("library has no filename")?);
                    plan.add(&resolved, &destination, false, inputs)?;
                    destination
                }
            };
            files[position].changes.push((name, destination));
        }
        position += 1;
    }
    let primary = PathBuf::from(format!(
        "usr/lib/{}/bin/{}",
        settings.config.identifier, settings.name
    ));
    ensure!(
        files
            .iter()
            .any(|file| file.destination == primary && file.entry != 0),
        "main input is not an x86_64 ELF executable"
    );
    for file in &files {
        let alias = file.destination.with_file_name(file.library_name()?);
        if alias != file.destination {
            plan.symlink(
                alias,
                file.destination
                    .file_name()
                    .context("library has no filename")?,
            )?;
        }
    }
    Ok(files)
}

pub fn relocate(
    input: &Input,
    files: &[Code],
    root: &Path,
    inputs: &mut Inputs,
    tools: &mut Tools<'_>,
) -> Result<()> {
    let library_root = PathBuf::from(format!("usr/lib/{}/lib", input.settings.config.identifier));
    for file in files.iter().filter(|file| file.dynamic) {
        let parent = file
            .destination
            .parent()
            .context("ELF output has no directory")?;
        let mut directories = BTreeSet::from([library_root.clone()]);
        let mut arguments = Vec::<std::ffi::OsString>::new();
        for (name, destination) in &file.changes {
            let dependency = files
                .iter()
                .find(|code| &code.destination == destination)
                .context("missing deployed ELF dependency")?;
            directories.insert(
                destination
                    .parent()
                    .context("library has no directory")?
                    .into(),
            );
            arguments.extend([
                "--replace-needed".into(),
                name.into(),
                dependency.library_name()?.into(),
            ]);
        }
        let mut paths = OsString::new();
        for (index, directory) in directories.iter().enumerate() {
            if index != 0 {
                paths.push(":");
            }
            paths.push("$ORIGIN");
            let relative = deploy::relative(parent, directory);
            if !relative.as_os_str().is_empty() {
                paths.push("/");
                paths.push(relative);
            }
        }
        arguments.extend(["--set-rpath".into(), paths]);
        if file.soname.is_some() {
            arguments.extend(["--set-soname".into(), file.library_name()?.into()]);
        }
        let output = root.join(&file.destination);
        arguments.push(output.clone().into_os_string());
        #[cfg(unix)]
        let permissions = {
            use std::os::unix::fs::PermissionsExt;
            let permissions = fs::metadata(&output)?.permissions();
            fs::set_permissions(
                &output,
                fs::Permissions::from_mode(permissions.mode() | 0o200),
            )?;
            permissions
        };
        tools
            .run("patchelf", arguments, inputs)
            .with_context(|| format!("relocate {}", file.destination.display()))?;
        #[cfg(unix)]
        fs::set_permissions(&output, permissions)?;
    }
    Ok(())
}
