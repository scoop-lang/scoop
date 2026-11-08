use std::{
    collections::BTreeMap,
    ffi::OsString,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, ensure};

use super::dependencies::Collected;
use crate::{
    cache::Inputs,
    config::{FileKind, SigningMode, relative_destination},
    deploy,
    project::Input,
    tools::Tools,
};

pub fn sign(
    input: &Input,
    app: &Path,
    code: &Collected,
    inputs: &mut Inputs,
    tools: &mut Tools<'_>,
) -> Result<()> {
    let settings = &input.settings;
    let signing = &settings.config.macos.signing;
    let identity = match signing.mode {
        SigningMode::None => {
            ensure!(
                signing.identity.is_none()
                    && signing.entitlements.is_none()
                    && signing.hardened_runtime.is_none()
                    && signing.timestamp.is_none(),
                "signing.mode = none cannot have signing options"
            );
            ensure!(
                settings
                    .config
                    .files
                    .iter()
                    .filter(|v| v.matches(&input.target))
                    .all(|v| v.macos.entitlements.is_none()),
                "signing.mode = none cannot have helper entitlements"
            );
            for code in code.files.iter().filter(|v| v.signed) {
                tools.run(
                    "codesign",
                    [
                        "--remove-signature".as_ref(),
                        app.join(&code.destination).as_os_str(),
                    ],
                    inputs,
                )?;
            }
            return Ok(());
        }
        SigningMode::AdHoc => {
            ensure!(
                signing.identity.is_none(),
                "ad-hoc signing does not accept identity"
            );
            ensure!(
                signing.timestamp != Some(true),
                "ad-hoc signing cannot request a trusted timestamp"
            );
            "-"
        }
        SigningMode::Identity => signing
            .identity
            .as_deref()
            .filter(|v| !v.trim().is_empty() && *v != "-")
            .context("bundle.macos.signing.identity is required in identity mode")?,
    };
    let mut entitlements = BTreeMap::<PathBuf, PathBuf>::new();
    for item in settings
        .config
        .files
        .iter()
        .filter(|v| v.matches(&input.target))
    {
        if let Some(path) = &item.macos.entitlements {
            let target = deploy::directory(settings, &input.target, FileKind::Executable)?
                .join(relative_destination(&item.to)?);
            ensure!(
                app.join(&target).is_file(),
                "helper entitlements require a single executable: {}",
                target.display()
            );
            entitlements.insert(target, settings.path(path));
        }
    }
    let mut targets: Vec<_> = code
        .files
        .iter()
        .filter(|v| v.destination != Path::new("Contents/MacOS").join(&settings.name))
        .map(|v| v.destination.clone())
        .collect();
    for item in walkdir::WalkDir::new(app).min_depth(1).follow_links(false) {
        let item = item?;
        if item.file_type().is_dir()
            && item
                .path()
                .extension()
                .is_some_and(|e| matches!(e.to_str(), Some("framework" | "app" | "xpc" | "bundle")))
        {
            targets.push(item.path().strip_prefix(app)?.into());
        }
    }
    targets.sort_by_key(|p| std::cmp::Reverse(p.components().count()));
    targets.dedup();
    targets.push(PathBuf::new());
    for target in targets {
        let mut arguments: Vec<OsString> = vec!["--force".into(), "--sign".into(), identity.into()];
        if signing
            .hardened_runtime
            .unwrap_or(signing.mode == SigningMode::Identity)
        {
            arguments.extend(["--options".into(), "runtime".into()]);
        }
        arguments.push(
            if signing
                .timestamp
                .unwrap_or(signing.mode == SigningMode::Identity)
            {
                "--timestamp"
            } else {
                "--timestamp=none"
            }
            .into(),
        );
        let entitlement = if target.as_os_str().is_empty() {
            signing.entitlements.as_ref().map(|p| settings.path(p))
        } else {
            entitlements.get(&target).cloned()
        };
        if let Some(path) = entitlement {
            inputs.watch(&path)?;
            arguments.extend(["--entitlements".into(), path.into_os_string()]);
        }
        arguments.push(app.join(&target).into_os_string());
        tools.run("codesign", arguments, inputs)?;
    }
    tools.run(
        "codesign",
        [
            "--verify".as_ref(),
            "--deep".as_ref(),
            "--strict".as_ref(),
            app.as_os_str(),
        ],
        inputs,
    )
}
