mod dependencies;
mod packages;

#[cfg(test)]
mod tests;

use std::{
    fs,
    io::Cursor,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, ensure};

use crate::{
    cache::Inputs,
    config::Loaded,
    deploy,
    project::Input,
    tools::{self, Tools},
};

pub fn build(
    input: &Input,
    format: &str,
    publish: &Path,
    scratch: &Path,
    inputs: &mut Inputs,
    tools: &mut Tools<'_>,
) -> Result<PathBuf> {
    let settings = &input.settings;
    ensure!(
        settings
            .name
            .starts_with(|c: char| c.is_ascii_alphanumeric())
            && settings
                .name
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"-+._".contains(&c)),
        "Linux bundle.name must contain only ASCII letters, digits, -, +, . or _"
    );
    if format == "deb" {
        ensure!(
            settings.name.len() >= 2
                && settings
                    .name
                    .bytes()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"-+.".contains(&c)),
            "Debian package names require at least two lowercase letters, digits, -, + or ."
        );
    }
    let mut plan = deploy::payload(settings, &input.target, &input.executable, inputs)?;
    let appimage = format == "appimage";
    let code = dependencies::collect(input, &mut plan, appimage, inputs, tools)?;
    let icon = icon(settings, inputs)?;
    ensure!(
        !appimage || icon.is_some(),
        "AppImage requires a PNG or ICO in bundle.icons"
    );
    let id = &settings.config.identifier;
    if let Some(icon) = &icon {
        plan.generated(
            format!("usr/share/icons/hicolor/256x256/apps/{id}.png"),
            icon.clone(),
        )?;
    }
    let desktop = desktop(settings, appimage, icon.is_some())?;
    plan.generated(
        format!("usr/share/applications/{id}.desktop"),
        desktop.clone(),
    )?;
    if appimage {
        plan.generated(format!("{id}.desktop"), desktop)?;
        plan.generated(format!("{id}.png"), icon.context("missing AppImage icon")?)?;
        plan.symlink(".DirIcon", format!("{id}.png"))?;
        let launch = format!(
            "#!/bin/sh\nHERE=$(CDPATH= cd -- \"$(dirname -- \"$0\")\" && pwd) || exit 1\nexec \"$HERE/usr/lib/{id}/bin/{}\" \"$@\"\n",
            settings.name
        );
        plan.generated("AppRun", launch.into_bytes())?;
    } else {
        plan.symlink(
            format!("usr/bin/{}", settings.name),
            format!("../lib/{id}/bin/{}", settings.name),
        )?;
    }
    if format == "deb" {
        plan.generated(
            "DEBIAN/control",
            packages::deb_control(settings)?.into_bytes(),
        )?;
    }
    let root = scratch.join(if appimage { "AppDir" } else { "payload" });
    fs::create_dir(&root)?;
    plan.write(&root)?;
    dependencies::relocate(input, &code, &root, inputs, tools)?;
    match format {
        "deb" => packages::deb(settings, &root, publish, inputs, tools),
        "rpm" => packages::rpm(settings, &root, publish),
        "appimage" => {
            tools::executable(&root.join("AppRun"))?;
            let artifact = PathBuf::from(format!(
                "{}-{}-{}.AppImage",
                settings.name, settings.version, input.target.arch
            ));
            let tool = tools.find("appimagetool", inputs)?;
            let mut command = std::process::Command::new(tool);
            command.env("ARCH", &input.target.arch);
            command
                .arg("--runtime-file")
                .arg(tools.find("appimage-runtime", inputs)?);
            command.arg(&root).arg(publish.join(&artifact));
            tools::run(&mut command, "appimagetool")?;
            tools::executable(&publish.join(&artifact))?;
            Ok(artifact)
        }
        _ => anyhow::bail!("invalid Linux package format {format}"),
    }
}

fn icon(settings: &Loaded, inputs: &mut Inputs) -> Result<Option<Vec<u8>>> {
    let Some(path) = settings.config.icons.iter().find(|p| {
        p.extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("png") || ext.eq_ignore_ascii_case("ico"))
    }) else {
        ensure!(
            settings.config.icons.is_empty(),
            "Linux icons require PNG or ICO"
        );
        return Ok(None);
    };
    let path = settings.path(path);
    inputs.watch(&path)?;
    let image = image::open(&path).with_context(|| format!("load icon {}", path.display()))?;
    ensure!(image.width() == image.height(), "Linux icon must be square");
    let mut bytes = Cursor::new(Vec::new());
    image
        .resize_exact(256, 256, image::imageops::FilterType::Lanczos3)
        .write_to(&mut bytes, image::ImageFormat::Png)?;
    Ok(Some(bytes.into_inner()))
}

fn desktop(settings: &Loaded, appimage: bool, icon: bool) -> Result<String> {
    fn text(value: &str) -> String {
        value
            .replace('\\', "\\\\")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
    }
    fn argument(value: &str) -> String {
        let mut escaped = String::new();
        for ch in value.chars() {
            if ch == '%' {
                escaped.push('%');
            }
            if matches!(ch, '"' | '`' | '$' | '\\') {
                escaped.push('\\');
            }
            escaped.push(ch);
        }
        text(&format!("\"{escaped}\""))
    }
    let linux = &settings.config.linux;
    for category in &linux.categories {
        ensure!(
            !category.is_empty()
                && category
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'-'),
            "invalid desktop category {category:?}"
        );
    }
    let command = if appimage {
        settings.name.clone()
    } else {
        format!("/usr/bin/{}", settings.name)
    };
    let mut result = format!(
        "[Desktop Entry]\nType=Application\nVersion=1.0\nName={}\nExec={}\nTerminal={}\nCategories={};\n",
        text(&settings.display_name),
        argument(&command),
        linux.terminal,
        if linux.categories.is_empty() {
            "Utility".into()
        } else {
            linux.categories.join(";")
        }
    );
    if icon {
        result.push_str(&format!("Icon={}\n", settings.config.identifier));
    }
    if let Some(description) = &settings.config.description {
        result.push_str(&format!("Comment={}\n", text(description)));
    }
    result.push_str(&format!("StartupWMClass={}\n", settings.config.identifier));
    if !linux.actions.is_empty() {
        let mut ids = std::collections::BTreeSet::new();
        for action in &linux.actions {
            ensure!(
                !action.id.is_empty()
                    && action
                        .id
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || c == b'-')
                    && ids.insert(&action.id),
                "desktop action id must be unique and contain only ASCII letters, digits or -: {:?}",
                action.id
            );
            ensure!(
                !action.title.is_empty()
                    && !action.title.contains('\0')
                    && action.arguments.iter().all(|value| !value.contains('\0')),
                "desktop action {:?} requires a nonempty title and NUL-free text",
                action.id
            );
        }
        result.push_str("Actions=");
        for action in &linux.actions {
            result.push_str(&action.id);
            result.push(';');
        }
        result.push('\n');
        for action in &linux.actions {
            result.push_str(&format!(
                "\n[Desktop Action {}]\nName={}\nExec={}",
                action.id,
                text(&action.title),
                argument(&command)
            ));
            for value in &action.arguments {
                result.push(' ');
                result.push_str(&argument(value));
            }
            result.push('\n');
            if icon {
                result.push_str(&format!("Icon={}\n", settings.config.identifier));
            }
        }
    }
    Ok(result)
}
