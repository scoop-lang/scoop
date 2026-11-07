use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail, ensure};

use crate::{
    cache::Inputs,
    config::{FileKind, Loaded, relative_destination},
    target::Target,
};

#[derive(Clone, Debug)]
enum Entry {
    Directory { explicit: bool },
    File { source: PathBuf, executable: bool },
    Link { source: PathBuf },
    Generated(Vec<u8>),
}

pub struct Plan {
    entries: BTreeMap<PathBuf, Entry>,
    names: BTreeMap<String, PathBuf>,
    insensitive: bool,
}

impl Plan {
    pub fn new(target: &Target) -> Self {
        Self {
            entries: BTreeMap::new(),
            names: BTreeMap::new(),
            insensitive: target.os != "linux",
        }
    }

    fn insert(&mut self, destination: PathBuf, entry: Entry) -> Result<()> {
        if destination.as_os_str().is_empty() {
            return Ok(());
        }
        if let Some(parent) = destination.parent() {
            self.directory(parent, false)?;
        }
        if self.insensitive {
            let key = destination.to_string_lossy().to_lowercase();
            if let Some(existing) = self.names.get(&key) {
                ensure!(
                    existing == &destination,
                    "package names conflict: {} and {}",
                    existing.display(),
                    destination.display()
                );
            }
            self.names.insert(key, destination.clone());
        }
        ensure!(
            !self.entries.contains_key(&destination),
            "duplicate package destination {}",
            destination.display()
        );
        self.entries.insert(destination, entry);
        Ok(())
    }

    pub fn directory(&mut self, path: &Path, explicit: bool) -> Result<()> {
        if path.as_os_str().is_empty() {
            return Ok(());
        }
        if let Some(existing) = self.entries.get_mut(path) {
            match existing {
                Entry::Directory {
                    explicit: was_explicit,
                } if !explicit || !*was_explicit => {
                    *was_explicit |= explicit;
                    return Ok(());
                }
                _ => bail!("conflicting package destination {}", path.display()),
            }
        }
        self.insert(path.into(), Entry::Directory { explicit })
    }

    pub fn generated(&mut self, path: impl Into<PathBuf>, data: impl Into<Vec<u8>>) -> Result<()> {
        self.insert(path.into(), Entry::Generated(data.into()))
    }

    pub fn add(
        &mut self,
        source: &Path,
        destination: &Path,
        executable: bool,
        inputs: &mut Inputs,
    ) -> Result<()> {
        inputs.watch(source)?;
        let metadata = fs::symlink_metadata(source)
            .with_context(|| format!("{} -> {}", source.display(), destination.display()))?;
        if metadata.is_symlink() {
            self.insert(
                destination.into(),
                Entry::Link {
                    source: source.into(),
                },
            )
        } else if metadata.is_dir() {
            self.directory(destination, true)?;
            for item in walkdir::WalkDir::new(source)
                .min_depth(1)
                .sort_by_file_name()
                .follow_links(false)
            {
                let item = item?;
                inputs.watch(item.path())?;
                let target = destination.join(item.path().strip_prefix(source)?);
                if item.file_type().is_symlink() {
                    self.insert(
                        target,
                        Entry::Link {
                            source: item.path().into(),
                        },
                    )?;
                } else if item.file_type().is_dir() {
                    self.directory(&target, true)?;
                } else if item.file_type().is_file() {
                    self.insert(
                        target,
                        Entry::File {
                            source: item.path().into(),
                            executable,
                        },
                    )?;
                } else {
                    bail!(
                        "{} is not a regular file, directory or symlink",
                        item.path().display()
                    );
                }
            }
            Ok(())
        } else if metadata.is_file() {
            self.insert(
                destination.into(),
                Entry::File {
                    source: source.into(),
                    executable,
                },
            )
        } else {
            bail!(
                "{} is not a regular file, directory or symlink",
                source.display()
            );
        }
    }

    pub fn source_files(&self) -> Vec<(PathBuf, PathBuf)> {
        self.entries
            .iter()
            .filter_map(|(path, entry)| match entry {
                Entry::File { source, .. } => Some((source.clone(), path.clone())),
                _ => None,
            })
            .collect()
    }

    pub fn write(&self, root: &Path) -> Result<()> {
        for (destination, entry) in &self.entries {
            let destination = root.join(destination);
            match entry {
                Entry::Directory { .. } => fs::create_dir_all(&destination)?,
                Entry::File { source, executable } => {
                    let mut input = fs::File::open(source)
                        .with_context(|| format!("read {}", source.display()))?;
                    let mut output = fs::OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(&destination)
                        .with_context(|| {
                            format!("{} -> {}", source.display(), destination.display())
                        })?;
                    std::io::copy(&mut input, &mut output)?;
                    let permissions = input.metadata()?.permissions();
                    fs::set_permissions(&destination, permissions)?;
                    if *executable {
                        crate::tools::executable(&destination)?;
                    }
                }
                Entry::Link { source } => {
                    let target = fs::read_link(source)?;
                    #[cfg(unix)]
                    std::os::unix::fs::symlink(target, &destination)?;
                    #[cfg(windows)]
                    bail!(
                        "installer payload cannot express symlink {} -> {}",
                        source.display(),
                        target.display()
                    );
                }
                Entry::Generated(bytes) => {
                    let mut file = fs::OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(&destination)?;
                    file.write_all(bytes)?;
                }
            }
        }
        Ok(())
    }
}

pub fn directory(settings: &Loaded, target: &Target, kind: FileKind) -> Result<PathBuf> {
    let id = &settings.config.identifier;
    let value = match (target.os.as_str(), kind) {
        (_, FileKind::File) => String::new(),
        ("darwin", FileKind::Resource) => "Contents/Resources".into(),
        ("darwin", FileKind::Executable) => "Contents/MacOS".into(),
        ("darwin", FileKind::Library | FileKind::Framework) => "Contents/Frameworks".into(),
        ("linux", FileKind::Resource) => format!("usr/share/{id}"),
        ("linux", FileKind::Executable) => format!("usr/lib/{id}/bin"),
        ("linux", FileKind::Library) => format!("usr/lib/{id}/lib"),
        ("windows", FileKind::Resource) => "resources".into(),
        ("windows", FileKind::Executable | FileKind::Library) => String::new(),
        _ => bail!("framework files require a Darwin target"),
    };
    Ok(value.into())
}

pub fn payload(
    settings: &Loaded,
    target: &Target,
    executable: &Path,
    inputs: &mut Inputs,
) -> Result<Plan> {
    let mut plan = Plan::new(target);
    let executable_name = format!(
        "{}{}",
        settings.name,
        if target.os == "windows" { ".exe" } else { "" }
    );
    inputs.watch(executable)?;
    let executable = executable
        .canonicalize()
        .with_context(|| format!("locate executable {}", executable.display()))?;
    plan.add(
        &executable,
        &directory(settings, target, FileKind::Executable)?.join(executable_name),
        true,
        inputs,
    )?;
    for (index, item) in settings
        .config
        .files
        .iter()
        .enumerate()
        .filter(|(_, item)| item.matches(target))
    {
        let destination =
            directory(settings, target, item.kind)?.join(relative_destination(&item.to)?);
        plan.add(
            &settings.path(&item.from),
            &destination,
            item.kind == FileKind::Executable,
            inputs,
        )
        .with_context(|| format!("{}: bundle.files[{index}]", settings.source.display()))?;
        if let Some(path) = &item.macos.entitlements {
            inputs.watch(&settings.path(path))?;
        }
    }
    Ok(plan)
}

pub fn relative(from: &Path, to: &Path) -> PathBuf {
    let a: Vec<_> = from.components().collect();
    let b: Vec<_> = to.components().collect();
    let common = a.iter().zip(&b).take_while(|(a, b)| a == b).count();
    let mut result = PathBuf::new();
    for _ in common..a.len() {
        result.push("..");
    }
    for component in &b[common..] {
        result.push(component);
    }
    result
}
