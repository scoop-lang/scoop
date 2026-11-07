mod args;
mod cache;
mod config;
mod deploy;
mod host_path;
mod linux;
mod macos;
mod project;
mod target;
mod tools;
mod windows;

#[cfg(test)]
mod tests;

pub use args::Args;

use std::{fs, path::PathBuf};

use anyhow::{Context, Result, bail, ensure};
use serde_json::json;

pub fn run(args: &Args) -> Result<()> {
    let input = project::prepare(args)?;
    let settings = &input.settings;
    let mut formats = args.formats.clone();
    if formats.is_empty() {
        formats = match input.target.os.as_str() {
            "linux" => settings.config.linux.formats.clone(),
            "windows" => settings.config.windows.formats.clone(),
            _ => settings.config.macos.formats.clone(),
        };
    }
    if formats.is_empty() {
        formats.push(
            match input.target.os.as_str() {
                "linux" => "deb",
                "windows" => "nsis",
                _ => "app",
            }
            .into(),
        );
    }
    let mut selected = std::collections::BTreeSet::new();
    formats.retain(|format| selected.insert(format.clone()));
    let mut failures = Vec::new();
    for format in formats {
        match bundle(&input, &format, args.force) {
            Ok((path, reused)) => {
                if args.json() {
                    println!(
                        "{}",
                        json!({"schema":1,"kind":"bundle","format":format,"target":input.target.triple,"profile":input.profile,"executable":host_path::value(&input.executable),"output":host_path::value(&path),"reused":reused})
                    );
                } else {
                    println!(
                        "{} {} ({format})",
                        if reused { "reused" } else { "bundled" },
                        path.display()
                    );
                }
            }
            Err(error) => {
                eprintln!("{format}: {error:#}");
                if args.json() {
                    println!(
                        "{}",
                        json!({"schema":1,"kind":"bundle-error","format":format,"target":input.target.triple,"message":format!("{error:#}")})
                    );
                }
                failures.push(format);
            }
        }
    }
    ensure!(
        failures.is_empty(),
        "failed formats: {}",
        failures.join(", ")
    );
    Ok(())
}

fn bundle(input: &project::Input, format: &str, force: bool) -> Result<(PathBuf, bool)> {
    ensure!(
        matches!(
            (input.target.os.as_str(), format),
            ("darwin", "app") | ("linux", "deb" | "rpm" | "appimage") | ("windows", "nsis" | "msi")
        ),
        "format {format:?} is unavailable for {}",
        input.target.triple
    );
    let settings = &input.settings;
    let parent = input
        .output_root
        .join(&input.target.triple)
        .join(&input.profile)
        .join(&settings.config.identifier);
    fs::create_dir_all(&parent)?;
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(parent.join(format!(".{format}.lock")))?;
    lock.lock()?;
    let destination = parent.join(format);
    let key = json!({"schema":1,"config":settings.config,"source":host_path::value(&settings.source),"name":settings.name,"version":settings.version.to_string(),"display_name":settings.display_name,"target":input.target.triple,"profile":input.profile,"format":format,"executable":host_path::value(&input.executable),"path":std::env::var_os("PATH").map(|v|host_path::value(std::path::Path::new(&v)))});
    if !force && let Some(path) = cache::hit(&destination, &key)? {
        return Ok((path, true));
    }
    let scratch = tempfile::Builder::new()
        .prefix(&format!(".{format}-"))
        .tempdir_in(&parent)?;
    let publish = scratch.path().join("publish");
    fs::create_dir(&publish)?;
    let mut inputs = cache::Inputs::default();
    inputs.watch(&std::env::current_exe()?)?;
    inputs.watch(&settings.source)?;
    for path in &settings.config.library_search_paths {
        inputs.watch(&settings.path(path))?;
    }
    let mut tools = tools::Tools::new(settings);
    let artifact = match input.target.os.as_str() {
        "windows" => windows::build(
            input,
            format,
            &publish,
            scratch.path(),
            &mut inputs,
            &mut tools,
        )?,
        "linux" => linux::build(
            input,
            format,
            &publish,
            scratch.path(),
            &mut inputs,
            &mut tools,
        )?,
        _ => macos::build(input, &publish, scratch.path(), &mut inputs, &mut tools)?,
    };
    inputs.save(&publish, key, artifact.clone())?;
    let previous = scratch.path().join("previous");
    if destination.exists() {
        fs::rename(&destination, &previous).context("move previous bundle for publication")?;
    }
    if let Err(error) = fs::rename(&publish, &destination) {
        if previous.exists() {
            fs::rename(&previous, &destination)
                .context("restore previous bundle after publication failure")?;
        }
        bail!("publish {}: {error}", destination.display());
    }
    Ok((destination.join(artifact), false))
}
