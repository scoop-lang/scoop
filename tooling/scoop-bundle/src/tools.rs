use std::{
    collections::BTreeMap,
    ffi::OsStr,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, bail, ensure};

use crate::{cache::Inputs, config::Loaded};

pub struct Tools<'a> {
    settings: &'a Loaded,
    resolved: BTreeMap<String, PathBuf>,
}

impl<'a> Tools<'a> {
    pub fn new(settings: &'a Loaded) -> Self {
        Self {
            settings,
            resolved: BTreeMap::new(),
        }
    }

    pub fn find(&mut self, name: &str, inputs: &mut Inputs) -> Result<PathBuf> {
        if let Some(path) = self.resolved.get(name) {
            return Ok(path.clone());
        }
        let path = if let Some(path) = self.settings.config.tools.get(name) {
            self.settings.path(path)
        } else {
            let suffix = if cfg!(windows) { ".exe" } else { "" };
            let path = std::env::var_os("PATH").unwrap_or_default();
            let mut found = None;
            for directory in std::env::split_paths(&path) {
                let candidate = directory.join(format!("{name}{suffix}"));
                inputs.watch(&candidate)?;
                if candidate.is_file() {
                    found = Some(candidate);
                    break;
                }
            }
            found.with_context(|| {
                format!(
                    "required tool {name:?} is not installed; set bundle.tools.{name} to its path"
                )
            })?
        };
        inputs.watch(&path)?;
        let path = path
            .canonicalize()
            .with_context(|| format!("find {name}: {}", path.display()))?;
        inputs.watch(&path)?;
        self.resolved.insert(name.into(), path.clone());
        Ok(path)
    }

    pub fn run<I, S>(&mut self, name: &str, arguments: I, inputs: &mut Inputs) -> Result<()>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let path = self.find(name, inputs)?;
        run(Command::new(&path).args(arguments), name).map(|_| ())
    }
}

pub fn run(command: &mut Command, step: &str) -> Result<std::process::Output> {
    let output = command.output().with_context(|| {
        format!(
            "{step}: execute {}",
            command.get_program().to_string_lossy()
        )
    })?;
    if !output.stdout.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&output.stdout));
    }
    if !output.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
    }
    ensure!(output.status.success(), "{step} failed ({})", output.status);
    Ok(output)
}

pub fn executable(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))?;
    }
    if !path.is_file() {
        bail!("missing executable {}", path.display());
    }
    Ok(())
}
