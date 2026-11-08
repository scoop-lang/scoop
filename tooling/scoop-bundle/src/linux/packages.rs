use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail, ensure};

use crate::{cache::Inputs, config::Loaded, tools::Tools};

fn line<'a>(value: &'a str, field: &str) -> Result<&'a str> {
    ensure!(
        !value.trim().is_empty() && !value.chars().any(char::is_control),
        "{field} must be a nonempty single line"
    );
    Ok(value)
}

fn version(settings: &Loaded, value: Option<&str>, field: &str) -> Result<String> {
    if let Some(value) = value {
        return Ok(line(value, field)?.into());
    }
    ensure!(
        settings.version.pre.is_empty() && settings.version.build.is_empty(),
        "pre-release/build metadata requires {field}"
    );
    Ok(settings.version.to_string())
}

fn deb_version(settings: &Loaded) -> Result<String> {
    let deb = &settings.config.linux.deb;
    let value = version(settings, deb.version.as_deref(), "bundle.linux.deb.version")?;
    ensure!(
        value.starts_with(|c: char| c.is_ascii_digit())
            && value
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b".+:~-".contains(&c)),
        "invalid bundle.linux.deb.version"
    );
    let revision = line(
        deb.revision.as_deref().unwrap_or("1"),
        "bundle.linux.deb.revision",
    )?;
    ensure!(
        revision
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"+.~".contains(&c)),
        "invalid Debian revision"
    );
    Ok(format!("{value}-{revision}"))
}

pub fn deb_control(settings: &Loaded) -> Result<String> {
    let deb = &settings.config.linux.deb;
    let maintainer = line(
        deb.maintainer
            .as_deref()
            .context("bundle.linux.deb.maintainer is required")?,
        "bundle.linux.deb.maintainer",
    )?;
    let section = line(
        deb.section.as_deref().unwrap_or("utils"),
        "bundle.linux.deb.section",
    )?;
    let description = settings
        .config
        .description
        .as_deref()
        .filter(|v| !v.trim().is_empty())
        .context("Debian packages require bundle.description")?;
    let mut lines = description.lines();
    let mut result = format!(
        "Package: {}\nVersion: {}\nArchitecture: amd64\nMaintainer: {maintainer}\nSection: {section}\nPriority: optional\nDescription: {}\n",
        settings.name,
        deb_version(settings)?,
        lines.next().context("missing description")?
    );
    for value in lines {
        result.push_str(&format!(
            " {}\n",
            if value.is_empty() { "." } else { value }
        ));
    }
    if !deb.depends.is_empty() {
        for value in &deb.depends {
            line(value, "bundle.linux.deb.depends")?;
        }
        result.push_str(&format!("Depends: {}\n", deb.depends.join(", ")));
    }
    if let Some(url) = &settings.config.homepage {
        result.push_str(&format!("Homepage: {}\n", line(url, "bundle.homepage")?));
    }
    Ok(result)
}

pub fn deb(
    settings: &Loaded,
    root: &Path,
    publish: &Path,
    inputs: &mut Inputs,
    tools: &mut Tools<'_>,
) -> Result<PathBuf> {
    let artifact = PathBuf::from(format!(
        "{}_{}_amd64.deb",
        settings.name,
        deb_version(settings)?
    ));
    tools.run(
        "dpkg-deb",
        [
            "--build".as_ref(),
            "--root-owner-group".as_ref(),
            root.as_os_str(),
            publish.join(&artifact).as_os_str(),
        ],
        inputs,
    )?;
    Ok(artifact)
}

fn dependency(text: &str) -> Result<rpm::Dependency> {
    line(text, "bundle.linux.rpm.requires")?;
    let fields: Vec<_> = text.split_whitespace().collect();
    Ok(match fields.as_slice() {
        [name] => rpm::Dependency::any(*name),
        [name, "=", version] => rpm::Dependency::eq(*name, *version),
        [name, ">=", version] => rpm::Dependency::greater_eq(*name, *version),
        [name, "<=", version] => rpm::Dependency::less_eq(*name, *version),
        [name, ">", version] => rpm::Dependency::greater(*name, *version),
        [name, "<", version] => rpm::Dependency::less(*name, *version),
        _ => bail!("RPM requirement must be NAME or NAME OP VERSION: {text:?}"),
    })
}

pub fn rpm(settings: &Loaded, root: &Path, publish: &Path) -> Result<PathBuf> {
    let metadata = &settings.config.linux.rpm;
    let version = version(
        settings,
        metadata.version.as_deref(),
        "bundle.linux.rpm.version",
    )?;
    let release = line(
        metadata.release.as_deref().unwrap_or("1"),
        "bundle.linux.rpm.release",
    )?;
    for (name, value) in [("version", version.as_str()), ("release", release)] {
        ensure!(
            value
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"._+~^".contains(&c)),
            "invalid RPM {name}"
        );
    }
    let description = settings
        .config
        .description
        .as_deref()
        .filter(|v| !v.trim().is_empty())
        .context("RPM packages require bundle.description")?;
    let license = settings
        .config
        .license
        .as_deref()
        .context("RPM packages require bundle.license")?;
    let mut builder = rpm::PackageBuilder::new(
        &settings.name,
        &version,
        license,
        "x86_64",
        description.lines().next().context("missing description")?,
    );
    builder
        .using_config(rpm::BuildConfig::v4().compression(rpm::CompressionType::Gzip))
        .description(description)
        .release(release);
    if let Some(url) = &settings.config.homepage {
        builder.url(line(url, "bundle.homepage")?);
    }
    for value in &metadata.requires {
        builder.requires(dependency(value)?);
    }
    for entry in walkdir::WalkDir::new(root)
        .min_depth(1)
        .sort_by_file_name()
        .follow_links(false)
    {
        let entry = entry?;
        let relative = entry.path().strip_prefix(root)?;
        let path = format!(
            "/{}",
            relative.to_str().context("RPM file names must be UTF-8")?
        );
        #[cfg(unix)]
        let mode = {
            use std::os::unix::fs::PermissionsExt;
            fs::symlink_metadata(entry.path())?.permissions().mode() as u16 & 0o7777
        };
        #[cfg(windows)]
        let mode = if entry.file_type().is_dir() {
            0o755
        } else {
            0o644
        };
        if entry.file_type().is_symlink() {
            let target = fs::read_link(entry.path())?;
            builder.with_symlink(
                rpm::FileOptions::symlink(
                    path,
                    target
                        .to_str()
                        .context("RPM symlink targets must be UTF-8")?,
                )
                .user("root")
                .group("root"),
            )?;
        } else if entry.file_type().is_dir() {
            if !matches!(
                relative.to_str(),
                Some(
                    "usr"
                        | "usr/bin"
                        | "usr/lib"
                        | "usr/share"
                        | "usr/share/applications"
                        | "usr/share/icons"
                        | "usr/share/icons/hicolor"
                        | "usr/share/icons/hicolor/256x256"
                        | "usr/share/icons/hicolor/256x256/apps"
                )
            ) {
                builder.with_dir_entry(
                    rpm::FileOptions::dir(path)
                        .permissions(mode)
                        .user("root")
                        .group("root"),
                )?;
            }
        } else {
            builder.with_file(
                entry.path(),
                rpm::FileOptions::new(path)
                    .permissions(mode)
                    .user("root")
                    .group("root"),
            )?;
        }
    }
    let artifact = PathBuf::from(format!("{}-{version}-{release}.x86_64.rpm", settings.name));
    builder.build()?.write_file(publish.join(&artifact))?;
    Ok(artifact)
}
