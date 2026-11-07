use std::{collections::BTreeMap, path::PathBuf, process::Command};

use anyhow::{Context, Result, bail, ensure};
use serde_json::Value;

use crate::{
    Args,
    config::{self, Config, Loaded},
    host_path,
    target::Target,
};

pub struct Input {
    pub settings: Loaded,
    pub executable: PathBuf,
    pub target: Target,
    pub profile: String,
    pub output_root: PathBuf,
}

pub fn prepare(args: &Args) -> Result<Input> {
    if let Some(executable) = &args.executable {
        let source = host_path::absolute(args.config.as_ref().context("--config is required")?)?;
        let settings = Loaded::finish(
            config::parse_external(&source)?,
            source.clone(),
            None,
            None,
            &BTreeMap::new(),
        )
        .with_context(|| source.display().to_string())?;
        let target = Target::parse(args.target.as_deref().context("--target is required")?)?;
        target.require_host()?;
        return Ok(Input {
            settings,
            executable: host_path::absolute(executable)?,
            target,
            profile: "prebuilt".into(),
            output_root: output_root(args, std::env::current_dir()?.join("target"))?,
        });
    }
    let own = std::env::current_exe()?;
    let installed_root = own
        .parent()
        .and_then(|p| p.parent())
        .context("cannot locate bundle installation")?;
    let sysroot = host_path::absolute(
        &args
            .sysroot
            .clone()
            .or_else(|| std::env::var_os("SCOOP_SYSROOT").map(PathBuf::from))
            .unwrap_or_else(|| installed_root.into()),
    )?;
    let scoop = match &args.scoop {
        Some(path) => host_path::absolute(path)?,
        None => installed_root
            .join("bin")
            .join(if cfg!(windows) { "sco.exe" } else { "scoop" }),
    };
    let base = || {
        let mut command = Command::new(&scoop);
        command.arg("--sysroot").arg(&sysroot);
        if let Some(pm) = &args.package_manager {
            command.arg("--package-manager").arg(pm);
        }
        command
    };
    let mut members = base();
    members.args(["pm", "members", "--json"]);
    if let Some(root) = &args.root_input {
        members.arg(root);
    }
    if let Some(package) = &args.package {
        members.args(["-p", package]);
    }
    let result = members
        .output()
        .with_context(|| format!("execute {} pm members", scoop.display()))?;
    ensure!(
        result.status.success(),
        "Scoop member selection failed ({}): {}",
        result.status,
        String::from_utf8_lossy(&result.stderr)
    );
    let members: Value =
        serde_json::from_slice(&result.stdout).context("invalid PM members JSON")?;
    ensure!(
        members["format"] == 6,
        "scoop-bundle requires PM members protocol 6; update the paired Scoop tools"
    );
    let selected: Vec<_> = members["members"]
        .as_array()
        .context("PM members is not an array")?
        .iter()
        .filter(|m| m["selected"] == true)
        .collect();
    ensure!(
        selected.len() == 1,
        "bundle requires exactly one selected Cone; use -p group:name"
    );
    let member = selected[0];
    ensure!(
        member["kind"] == "executable",
        "bundle requires an executable Cone"
    );
    let coordinate = member["coordinate"]
        .as_str()
        .context("invalid Cone coordinate")?;
    let parts: Vec<_> = coordinate.split(':').collect();
    ensure!(parts.len() == 3, "invalid Cone coordinate {coordinate:?}");
    let manifest = host_path::decode(&member["manifest"])?;
    let metadata = serde_json::from_value::<BTreeMap<String, String>>(member["metadata"].clone())?;
    let (config, source) = match &args.config {
        Some(path) => {
            let path = host_path::absolute(path)?;
            (config::parse_external(&path)?, path)
        }
        None => {
            let table = &member["bundle"]["table"];
            ensure!(
                table.is_object(),
                "{}: missing [bundle] configuration",
                manifest.display()
            );
            if let Some(reference) = table.get("config") {
                ensure!(
                    table.as_object().is_some_and(|t| t.len() == 1),
                    "{}: bundle.config cannot be combined with inline fields",
                    manifest.display()
                );
                let path = manifest
                    .parent()
                    .context("manifest has no directory")?
                    .join(
                        reference
                            .as_str()
                            .context("bundle.config must be a path string")?,
                    );
                (config::parse_external(&path)?, path)
            } else {
                (
                    serde_json::from_value::<Config>(table.clone()).with_context(|| {
                        format!(
                            "{}: [bundle] at {}",
                            manifest.display(),
                            member["bundle"]["span"]
                        )
                    })?,
                    manifest.clone(),
                )
            }
        }
    };
    let settings = Loaded::finish(
        config,
        source.clone(),
        Some(parts[1]),
        Some(parts[2]),
        &metadata,
    )
    .with_context(|| source.display().to_string())?;
    let profile = args.profile.unwrap_or_default().as_str();
    let target_dir = match &args.target_dir {
        Some(path) => host_path::absolute(path)?,
        None => host_path::decode(&members["root"])?.join("target"),
    };
    let mut build = base();
    build
        .arg("build")
        .arg(&manifest)
        .args(["--profile", profile, "--message-format", "json"])
        .arg("--target-dir")
        .arg(&target_dir);
    if let Some(target) = &args.target {
        build.args(["--target", target]);
    }
    for (flag, enabled) in [
        ("--locked", args.locked),
        ("--offline", args.offline),
        ("--frozen", args.frozen),
    ] {
        if enabled {
            build.arg(flag);
        }
    }
    if let Some(jobs) = args.jobs {
        build.arg("-j").arg(jobs.to_string());
    }
    let output = build
        .output()
        .with_context(|| format!("execute {} build", scoop.display()))?;
    let mut success = None;
    for line in output
        .stderr
        .split(|&byte| byte == b'\n')
        .filter(|line| !line.is_empty())
    {
        if let Ok(record) = serde_json::from_slice::<Value>(line)
            && record["kind"] == "executable"
            && record["root"]["coordinate"]["group"] == parts[0]
            && record["root"]["coordinate"]["name"] == parts[1]
            && record["root"]["coordinate"]["version"] == parts[2]
        {
            success = Some(record);
        } else {
            eprintln!("{}", String::from_utf8_lossy(line));
        }
    }
    if !output.stdout.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&output.stdout));
    }
    if !output.status.success() {
        bail!(
            "Scoop build failed ({}); no bundle was generated",
            output.status
        );
    }
    let success = success.context("Scoop build did not return the selected executable record")?;
    let target = Target::parse(
        success["target"]
            .as_str()
            .context("build JSON has no canonical target")?,
    )?;
    target.require_host()?;
    ensure!(
        success["profile"] == profile,
        "build profile does not match the request"
    );
    Ok(Input {
        settings,
        executable: host_path::decode(&success["output"])?,
        target,
        profile: profile.into(),
        output_root: output_root(args, target_dir)?,
    })
}

fn output_root(args: &Args, target: PathBuf) -> Result<PathBuf> {
    host_path::absolute(
        &args
            .output_dir
            .clone()
            .unwrap_or_else(|| args.target_dir.clone().unwrap_or(target).join("bundle")),
    )
}
