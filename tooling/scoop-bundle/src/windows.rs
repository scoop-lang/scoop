mod dependencies;
mod msi;
mod notifications;
mod nsis;
mod signing;

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
    config::{InstallMode, Loaded},
    deploy,
    project::Input,
    tools::Tools,
};

const ICON: &str = ".scoop-bundle.ico";

pub fn build(
    input: &Input,
    format: &str,
    publish: &Path,
    scratch: &Path,
    inputs: &mut Inputs,
    tools: &mut Tools<'_>,
) -> Result<PathBuf> {
    let settings = &input.settings;
    notifications::validate(settings, format)?;
    let version = package_version(settings, format)?;
    if format == "msi" {
        msi::upgrade_code(settings)?;
        ensure!(
            settings
                .config
                .publisher
                .as_deref()
                .is_some_and(|v| !v.trim().is_empty()),
            "MSI requires bundle.publisher"
        );
    }
    let mut plan = deploy::payload(settings, &input.target, &input.executable, inputs)?;
    let code = dependencies::collect(input, &mut plan, inputs)?;
    let has_icon = if let Some(icon) = icon(settings, inputs)? {
        plan.generated(ICON, icon)?;
        true
    } else {
        false
    };
    if format == "nsis" {
        plan.require_unused(Path::new("uninstall.exe"))?;
    }
    let root = scratch.join("payload");
    fs::create_dir(&root)?;
    plan.write(&root)?;
    if settings.config.windows.signing.is_some() {
        for path in code {
            signing::sign(settings, &root.join(path), inputs, tools)?;
        }
    }
    let payload = Payload::read(&root)?;
    let artifact = PathBuf::from(format!(
        "{}-{}-x86_64.{}",
        settings.name,
        settings.version,
        if format == "nsis" { "exe" } else { "msi" }
    ));
    let output = publish.join(&artifact);
    let installer = Installer {
        settings,
        version: &version,
        payload: &payload,
        has_icon,
        output: &output,
        scratch,
    };
    match format {
        "nsis" => nsis::build(&installer, inputs, tools)?,
        "msi" => msi::build(&installer, inputs, tools)?,
        _ => anyhow::bail!("invalid Windows package format {format}"),
    }
    ensure!(
        output.is_file(),
        "installer tool did not generate {}",
        output.display()
    );
    if settings.config.windows.signing.is_some() {
        signing::sign(settings, &output, inputs, tools)?;
    }
    Ok(artifact)
}

struct Installer<'a> {
    settings: &'a Loaded,
    version: &'a str,
    payload: &'a Payload,
    has_icon: bool,
    output: &'a Path,
    scratch: &'a Path,
}

pub(super) struct Payload {
    root: PathBuf,
    files: Vec<PathBuf>,
    directories: Vec<PathBuf>,
}

impl Payload {
    fn read(root: &Path) -> Result<Self> {
        let mut result = Self {
            root: root.into(),
            files: Vec::new(),
            directories: vec![PathBuf::new()],
        };
        for entry in walkdir::WalkDir::new(root).min_depth(1).sort_by_file_name() {
            let entry = entry?;
            let path = entry.path().strip_prefix(root)?.to_path_buf();
            if entry.file_type().is_dir() {
                result.directories.push(path);
            } else {
                ensure!(
                    entry.file_type().is_file(),
                    "Windows installers require regular files: {}",
                    entry.path().display()
                );
                result.files.push(path);
            }
        }
        Ok(result)
    }
}

fn icon(settings: &Loaded, inputs: &mut Inputs) -> Result<Option<Vec<u8>>> {
    let Some(path) = settings
        .config
        .icons
        .iter()
        .find(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("ico")))
        .or_else(|| {
            settings
                .config
                .icons
                .iter()
                .find(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("png")))
        })
    else {
        ensure!(
            settings.config.icons.is_empty(),
            "Windows icons require ICO or PNG"
        );
        return Ok(None);
    };
    let path = settings.path(path);
    inputs.watch(&path)?;
    let image =
        image::open(&path).with_context(|| format!("read Windows icon {}", path.display()))?;
    ensure!(
        image.width() == image.height(),
        "Windows icon must be square"
    );
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("ico"))
    {
        return Ok(Some(fs::read(path)?));
    }
    let mut output = Cursor::new(Vec::new());
    let frames = [16, 32, 48, 64, 128, 256]
        .into_iter()
        .map(|size| {
            let frame = image
                .resize_exact(size, size, image::imageops::FilterType::Lanczos3)
                .to_rgba8();
            image::codecs::ico::IcoFrame::as_png(
                frame.as_raw(),
                size,
                size,
                image::ExtendedColorType::Rgba8,
            )
        })
        .collect::<image::ImageResult<Vec<_>>>()?;
    image::codecs::ico::IcoEncoder::new(&mut output).encode_images(&frames)?;
    Ok(Some(output.into_inner()))
}

fn package_version(settings: &Loaded, format: &str) -> Result<String> {
    let (explicit, limits, field) = if format == "msi" {
        (
            settings.config.windows.msi.product_version.as_deref(),
            &[255_u64, 255, 65535][..],
            "bundle.windows.msi.product-version",
        )
    } else {
        (
            settings.config.windows.nsis.file_version.as_deref(),
            &[65535_u64; 4][..],
            "bundle.windows.nsis.file-version",
        )
    };
    let value = if let Some(value) = explicit {
        value.to_owned()
    } else {
        ensure!(
            settings.version.pre.is_empty() && settings.version.build.is_empty(),
            "pre-release/build metadata requires {field}"
        );
        format!(
            "{}{}",
            settings.version,
            if format == "msi" { "" } else { ".0" }
        )
    };
    let parts = value.split('.').collect::<Vec<_>>();
    ensure!(
        parts.len() == limits.len()
            && parts.iter().zip(limits).all(|(s, max)| !s.is_empty()
                && s.bytes().all(|c| c.is_ascii_digit())
                && s.parse::<u64>().is_ok_and(|n| n <= *max)),
        "{field} requires {} numeric components within {:?}",
        limits.len(),
        limits
    );
    Ok(value)
}

fn per_user(settings: &Loaded) -> bool {
    settings.config.windows.install_mode == InstallMode::PerUser
}

fn registry_key(settings: &Loaded) -> String {
    format!("Software\\ScoopBundle\\{}", settings.config.identifier)
}

fn path_text(path: &Path) -> Result<String> {
    let value = path.to_str().with_context(|| {
        format!(
            "installer source cannot represent this path as Unicode: {}",
            path.display()
        )
    })?;
    Ok(if let Some(rest) = value.strip_prefix("\\\\?\\UNC\\") {
        format!("\\\\{rest}")
    } else {
        value.strip_prefix("\\\\?\\").unwrap_or(value).to_owned()
    })
}

fn relative_text(path: &Path) -> Result<String> {
    Ok(path_text(path)?.replace('/', "\\"))
}
