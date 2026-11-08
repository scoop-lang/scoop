mod dependencies;
mod signing;

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, ensure};

use crate::{cache::Inputs, config::Loaded, deploy, project::Input, tools::Tools};

pub fn build(
    input: &Input,
    publish: &Path,
    scratch: &Path,
    inputs: &mut Inputs,
    tools: &mut Tools<'_>,
) -> Result<PathBuf> {
    let settings = &input.settings;
    let mut plan = deploy::payload(settings, &input.target, &input.executable, inputs)?;
    let code = dependencies::collect(input, &mut plan, inputs)?;
    let icon = icon(settings, scratch, inputs, tools)?;
    if let Some(icon) = &icon {
        plan.generated("Contents/Resources/AppIcon.icns", fs::read(icon)?)?;
    }
    plan.generated(
        "Contents/Info.plist",
        info_plist(settings, code.minimum, icon.is_some(), inputs)?,
    )?;
    let artifact = PathBuf::from(format!("{}.app", settings.display_name));
    let app = publish.join(&artifact);
    fs::create_dir(&app)?;
    plan.write(&app)?;
    #[cfg(unix)]
    let permissions: Vec<_> = {
        use std::os::unix::fs::PermissionsExt;
        code.files
            .iter()
            .map(|file| {
                let path = app.join(&file.destination);
                let permissions = fs::metadata(&path)?.permissions();
                fs::set_permissions(
                    &path,
                    fs::Permissions::from_mode(permissions.mode() | 0o200),
                )?;
                Ok((path, permissions))
            })
            .collect::<Result<_>>()?
    };
    dependencies::relocate(&code, &app, inputs, tools)?;
    signing::sign(input, &app, &code, inputs, tools)?;
    #[cfg(unix)]
    for (path, permissions) in permissions {
        fs::set_permissions(path, permissions)?;
    }
    Ok(artifact)
}

fn version(text: &str, field: &str, maximum_parts: usize) -> Result<Vec<u32>> {
    let parts: Vec<_> = text.split('.').collect();
    ensure!(
        !parts.is_empty()
            && parts.len() <= maximum_parts
            && parts
                .iter()
                .all(|p| !p.is_empty() && p.bytes().all(|c| c.is_ascii_digit())),
        "bundle.macos.{field}: expected one to {maximum_parts} numeric version components"
    );
    parts
        .into_iter()
        .map(|p| {
            p.parse()
                .with_context(|| format!("bundle.macos.{field}: version component is too large"))
        })
        .collect()
}

fn info_plist(
    settings: &Loaded,
    minimum: u32,
    has_icon: bool,
    inputs: &mut Inputs,
) -> Result<Vec<u8>> {
    let macos = &settings.config.macos;
    let stable = settings.version.pre.is_empty() && settings.version.build.is_empty();
    let default = settings.version.to_string();
    ensure!(
        stable || macos.short_version.is_some() && macos.build_version.is_some(),
        "pre-release/build metadata requires bundle.macos.short-version and build-version"
    );
    let short = macos.short_version.as_deref().unwrap_or(&default);
    ensure!(
        version(short, "short-version", 3)?.len() == 3,
        "bundle.macos.short-version needs three numeric components"
    );
    let build = macos.build_version.as_deref().unwrap_or(&default);
    let build_parts = version(build, "build-version", 3)?;
    ensure!(
        build_parts[0] <= 9999 && build_parts.iter().skip(1).all(|v| *v <= 99),
        "bundle.macos.build-version exceeds Apple's numeric field limits"
    );
    let minimum = if let Some(request) = &macos.minimum_system_version {
        let mut parts = version(request, "minimum-system-version", 3)?;
        parts.resize(3, 0);
        ensure!(
            parts[0] <= 65535 && parts[1] <= 255 && parts[2] <= 255,
            "invalid minimum-system-version"
        );
        let value = (parts[0] << 16) | (parts[1] << 8) | parts[2];
        ensure!(
            value >= minimum,
            "bundle.macos.minimum-system-version is lower than a packaged binary's deployment target {}.{}.{}",
            minimum >> 16,
            (minimum >> 8) & 255,
            minimum & 255
        );
        value
    } else {
        minimum
    };
    let mut generated = plist::Dictionary::new();
    for (key, value) in [
        ("CFBundleIdentifier", settings.config.identifier.clone()),
        ("CFBundleExecutable", settings.name.clone()),
        ("CFBundleName", settings.name.clone()),
        ("CFBundleDisplayName", settings.display_name.clone()),
        ("CFBundlePackageType", "APPL".into()),
        ("CFBundleInfoDictionaryVersion", "6.0".into()),
        ("CFBundleShortVersionString", short.into()),
        ("CFBundleVersion", build.into()),
        (
            "LSMinimumSystemVersion",
            format!(
                "{}.{}.{}",
                minimum >> 16,
                (minimum >> 8) & 255,
                minimum & 255
            ),
        ),
    ] {
        generated.insert(key.into(), plist::Value::String(value));
    }
    if has_icon {
        generated.insert(
            "CFBundleIconFile".into(),
            plist::Value::String("AppIcon".into()),
        );
    }
    if let Some(path) = &macos.info_plist {
        let path = settings.path(path);
        inputs.watch(&path)?;
        let extra = plist::Value::from_file(&path)
            .with_context(|| format!("read {}", path.display()))?
            .into_dictionary()
            .context("bundle.macos.info-plist must contain a dictionary")?;
        for (key, value) in extra {
            if let Some(current) = generated.get(&key) {
                ensure!(
                    current == &value,
                    "{}: generated Info.plist key {key} conflicts with the configured value",
                    path.display()
                );
            } else {
                generated.insert(key, value);
            }
        }
    }
    let mut bytes = Vec::new();
    plist::Value::Dictionary(generated).to_writer_xml(&mut bytes)?;
    Ok(bytes)
}

fn icon(
    settings: &Loaded,
    scratch: &Path,
    inputs: &mut Inputs,
    tools: &mut Tools<'_>,
) -> Result<Option<PathBuf>> {
    for path in &settings.config.icons {
        if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("icns"))
        {
            let path = settings.path(path);
            inputs.watch(&path)?;
            return Ok(Some(path));
        }
    }
    let Some(path) = settings
        .config
        .icons
        .iter()
        .find(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("png")))
    else {
        ensure!(
            settings.config.icons.is_empty(),
            "macOS icons require an ICNS or PNG input"
        );
        return Ok(None);
    };
    let path = settings.path(path);
    inputs.watch(&path)?;
    let image = image::open(&path).with_context(|| format!("load icon {}", path.display()))?;
    ensure!(image.width() == image.height(), "macOS icon must be square");
    let iconset = scratch.join("AppIcon.iconset");
    fs::create_dir(&iconset)?;
    for size in [16, 32, 128, 256, 512] {
        for scale in [1, 2] {
            let name = format!(
                "icon_{size}x{size}{}.png",
                if scale == 2 { "@2x" } else { "" }
            );
            image
                .resize_exact(
                    size * scale,
                    size * scale,
                    image::imageops::FilterType::Lanczos3,
                )
                .save(iconset.join(name))?;
        }
    }
    let output = scratch.join("AppIcon.icns");
    tools.run(
        "iconutil",
        [
            "-c".as_ref(),
            "icns".as_ref(),
            "-o".as_ref(),
            output.as_os_str(),
            iconset.as_os_str(),
        ],
        inputs,
    )?;
    Ok(Some(output))
}
