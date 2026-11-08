use std::{
    collections::BTreeSet,
    ffi::OsStr,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail, ensure};
use goblin::mach::{Mach, MachO, SingleArch, load_command::CommandVariant};

use crate::{
    cache::Inputs,
    deploy::{self, Plan},
    project::Input,
    tools::Tools,
};

#[derive(Debug)]
pub struct Code {
    pub source: PathBuf,
    pub destination: PathBuf,
    pub libraries: Vec<String>,
    pub rpaths: Vec<String>,
    pub dylib: bool,
    pub executable: bool,
    pub signed: bool,
    pub thin_arch: Option<&'static str>,
    pub minimum: u32,
    pub changes: Vec<(String, PathBuf)>,
}

pub struct Collected {
    pub files: Vec<Code>,
    pub minimum: u32,
}

fn describe(mach: MachO<'_>, source: &Path, destination: &Path, cpu: u32) -> Result<Code> {
    ensure!(
        mach.header.cputype == cpu,
        "{} has the wrong machine architecture",
        source.display()
    );
    ensure!(mach.is_64, "{} is not a 64-bit Mach-O", source.display());
    ensure!(
        matches!(
            mach.header.filetype,
            goblin::mach::header::MH_EXECUTE
                | goblin::mach::header::MH_DYLIB
                | goblin::mach::header::MH_BUNDLE
        ),
        "{} is not a linked Mach-O image",
        source.display()
    );
    let mut minimum = 0;
    for command in &mach.load_commands {
        match command.command {
            CommandVariant::VersionMinMacosx(value) => minimum = minimum.max(value.version),
            CommandVariant::BuildVersion(value) => {
                ensure!(
                    value.platform == 1,
                    "{} is not built for macOS",
                    source.display()
                );
                minimum = minimum.max(value.minos);
            }
            _ => (),
        }
    }
    Ok(Code {
        source: source.into(),
        destination: destination.into(),
        libraries: mach
            .libs
            .iter()
            .filter(|v| **v != "self")
            .map(|v| (*v).into())
            .collect(),
        rpaths: mach.rpaths.iter().map(|v| (*v).into()).collect(),
        dylib: mach.header.filetype == goblin::mach::header::MH_DYLIB,
        executable: mach.header.filetype == goblin::mach::header::MH_EXECUTE,
        signed: mach
            .load_commands
            .iter()
            .any(|command| matches!(command.command, CommandVariant::CodeSignature(_))),
        thin_arch: None,
        minimum,
        changes: Vec::new(),
    })
}

fn inspect(source: &Path, destination: &Path, arch: &str) -> Result<Option<Code>> {
    let mut file = fs::File::open(source)?;
    let mut magic = [0u8; 4];
    if file.read(&mut magic)? != 4
        || !matches!(
            u32::from_be_bytes(magic),
            0xfeedface | 0xfeedfacf | 0xcefaedfe | 0xcffaedfe | 0xcafebabe | 0xbebafeca
        )
    {
        return Ok(None);
    }
    let bytes = fs::read(source)?;
    let cpu = if arch == "aarch64" {
        goblin::mach::constants::cputype::CPU_TYPE_ARM64
    } else {
        goblin::mach::constants::cputype::CPU_TYPE_X86_64
    };
    match Mach::parse(&bytes).with_context(|| format!("read Mach-O {}", source.display()))? {
        Mach::Binary(mach) => describe(mach, source, destination, cpu).map(Some),
        Mach::Fat(fat) => {
            for entry in &fat {
                if let SingleArch::MachO(mach) = entry?
                    && mach.header.cputype == cpu
                {
                    let mut code = describe(mach, source, destination, cpu)?;
                    code.thin_arch = Some(if arch == "aarch64" { "arm64" } else { "x86_64" });
                    return Ok(Some(code));
                }
            }
            bail!("{} has no {arch} Mach-O slice", source.display());
        }
    }
}

fn system(name: &str) -> bool {
    name.starts_with("/System/Library/") || name.starts_with("/usr/lib/")
}

fn expand(name: &str, loader: &Path, executable: &Path) -> Option<PathBuf> {
    if let Some(suffix) = name.strip_prefix("@loader_path/") {
        Some(loader.join(suffix))
    } else if name == "@loader_path" {
        Some(loader.into())
    } else if let Some(suffix) = name.strip_prefix("@executable_path/") {
        Some(executable.join(suffix))
    } else if name == "@executable_path" {
        Some(executable.into())
    } else if name.starts_with('@') {
        None
    } else {
        Some(loader.join(name))
    }
}

fn framework(path: &Path) -> Option<(PathBuf, PathBuf)> {
    let mut root = PathBuf::new();
    for component in path.components() {
        root.push(component);
        if component
            .as_os_str()
            .to_string_lossy()
            .ends_with(".framework")
        {
            return Some((root.clone(), path.strip_prefix(&root).ok()?.into()));
        }
    }
    None
}

pub fn collect(input: &Input, plan: &mut Plan, inputs: &mut Inputs) -> Result<Collected> {
    let settings = &input.settings;
    let primary = input.executable.canonicalize()?;
    let executable = primary.parent().context("executable has no directory")?;
    let mut files = Vec::<Code>::new();
    let mut scanned = BTreeSet::new();
    let mut position = 0;
    loop {
        for (source, destination) in plan.source_files() {
            if !scanned.insert(destination.clone()) {
                continue;
            }
            if let Some(code) = inspect(&source, &destination, &input.target.arch)? {
                files.push(code);
            }
        }
        if position == files.len() {
            break;
        }
        let origin = files[position].source.clone();
        let loader = origin.parent().context("code has no parent directory")?;
        let mut paths: Vec<PathBuf> = files[position]
            .rpaths
            .iter()
            .filter_map(|p| expand(p, loader, executable))
            .collect();
        for main in files
            .iter()
            .filter(|code| code.destination == Path::new("Contents/MacOS").join(&settings.name))
        {
            paths.extend(
                main.rpaths
                    .iter()
                    .filter_map(|p| expand(p, executable, executable)),
            );
        }
        for name in files[position].libraries.clone() {
            if system(&name) {
                continue;
            }
            let suffix = name.strip_prefix("@rpath/").unwrap_or(&name);
            let basename = Path::new(suffix)
                .file_name()
                .context("invalid Mach-O dependency name")?;
            let matches: Vec<_> = files
                .iter()
                .filter(|code| code.dylib && code.source.file_name() == Some(basename))
                .collect();
            let originals: BTreeSet<_> = matches
                .iter()
                .map(|code| code.source.canonicalize())
                .collect::<std::io::Result<_>>()?;
            ensure!(
                originals.len() <= 1,
                "{}: ambiguous private dependency {name}",
                origin.display()
            );
            let destination = if let Some(code) = matches.first() {
                code.destination.clone()
            } else {
                let mut candidates = Vec::new();
                for directory in &settings.config.library_search_paths {
                    let directory = settings.path(directory);
                    candidates.push(directory.join(suffix.trim_start_matches('/')));
                    candidates.push(directory.join(basename));
                    if let Some((root, tail)) = framework(Path::new(&name))
                        && let Some(name) = root.file_name()
                    {
                        candidates.push(directory.join(name).join(tail));
                    }
                }
                if name.starts_with("@rpath/") {
                    candidates.extend(paths.iter().map(|path| path.join(suffix)));
                } else if let Some(path) = expand(&name, loader, executable) {
                    candidates.push(path);
                }
                let mut resolved = None;
                for candidate in candidates {
                    inputs.watch(&candidate)?;
                    if candidate.is_file() {
                        resolved = Some(candidate.canonicalize()?);
                        break;
                    }
                }
                let resolved = resolved.with_context(||format!("{}: missing private dependency {name}; use library-search-paths or a file mapping",origin.display()))?;
                inputs.watch(&resolved)?;
                ensure!(
                    resolved.extension().is_none_or(|v| v != "tbd"),
                    "{} is a link stub, not a runtime library",
                    resolved.display()
                );
                let existing = plan
                    .source_files()
                    .into_iter()
                    .find_map(|(source, destination)| {
                        (source.canonicalize().ok().as_ref() == Some(&resolved))
                            .then_some(destination)
                    });
                if let Some(destination) = existing {
                    destination
                } else if let Some((root, tail)) = framework(&resolved) {
                    let destination = Path::new("Contents/Frameworks")
                        .join(root.file_name().context("invalid framework name")?);
                    plan.add(&root, &destination, false, inputs)?;
                    destination.join(tail)
                } else {
                    let destination = Path::new("Contents/Frameworks")
                        .join(resolved.file_name().context("invalid library file name")?);
                    plan.add(&resolved, &destination, false, inputs)?;
                    destination
                }
            };
            files[position].changes.push((name, destination));
        }
        position += 1;
    }
    ensure!(
        files.iter().any(|code| code.executable
            && code.destination == Path::new("Contents/MacOS").join(&settings.name)),
        "main executable is not a compatible Mach-O executable image"
    );
    let minimum = files.iter().map(|code| code.minimum).max().unwrap_or(0);
    Ok(Collected { files, minimum })
}

pub fn relocate(
    code: &Collected,
    app: &Path,
    inputs: &mut Inputs,
    tools: &mut Tools<'_>,
) -> Result<()> {
    for code in &code.files {
        let staged = app.join(&code.destination);
        if let Some(arch) = code.thin_arch {
            tools
                .run(
                    "lipo",
                    [
                        OsStr::new("-thin"),
                        OsStr::new(arch),
                        staged.as_os_str(),
                        OsStr::new("-output"),
                        staged.as_os_str(),
                    ],
                    inputs,
                )
                .with_context(|| {
                    format!("select {arch} slice in {}", code.destination.display())
                })?;
        }
        let mut arguments = Vec::<std::ffi::OsString>::new();
        for (name, destination) in &code.changes {
            let relative = deploy::relative(
                code.destination.parent().context("code has no directory")?,
                destination,
            );
            arguments.extend([
                "-change".into(),
                name.into(),
                format!("@loader_path/{}", relative.display()).into(),
            ]);
        }
        for path in code.rpaths.iter().collect::<BTreeSet<_>>() {
            arguments.extend(["-delete_rpath".into(), path.into()]);
        }
        if code.dylib {
            let relative = deploy::relative(Path::new("Contents/Frameworks"), &code.destination);
            arguments.extend([
                "-id".into(),
                format!("@rpath/{}", relative.display()).into(),
            ]);
        }
        let relative = deploy::relative(
            code.destination.parent().context("code has no directory")?,
            Path::new("Contents/Frameworks"),
        );
        arguments.extend([
            "-add_rpath".into(),
            format!("@loader_path/{}", relative.display()).into(),
            staged.into_os_string(),
        ]);
        tools
            .run("install_name_tool", arguments, inputs)
            .with_context(|| format!("relocate {}", code.destination.display()))?;
    }
    Ok(())
}
