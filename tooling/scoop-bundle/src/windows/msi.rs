use std::{fmt::Write, fs, path::Path, process::Command};

use anyhow::{Context, Result, ensure};
use uuid::Uuid;

use super::{ICON, Installer, path_text, per_user, registry_key, relative_text};
use crate::{
    cache::Inputs,
    config::Loaded,
    tools::{self, Tools},
};

pub fn upgrade_code(settings: &Loaded) -> Result<Uuid> {
    let value = settings
        .config
        .windows
        .msi
        .upgrade_code
        .as_deref()
        .context("MSI requires bundle.windows.msi.upgrade-code")?;
    let uuid = Uuid::parse_str(value).context("bundle.windows.msi.upgrade-code must be a UUID")?;
    ensure!(!uuid.is_nil(), "MSI UpgradeCode must not be nil");
    Ok(uuid)
}

fn xml(value: &str) -> String {
    value
        .replace("$(", "$$(")
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\r', "&#13;")
        .replace('\n', "&#10;")
        .replace('\t', "&#9;")
}

pub(super) fn identity(upgrade: Uuid, user: bool, kind: &str, path: &Path) -> Result<Uuid> {
    Ok(Uuid::new_v5(
        &upgrade,
        format!(
            "{}:{kind}:{}",
            if user { "user" } else { "machine" },
            relative_text(path)?.to_lowercase()
        )
        .as_bytes(),
    ))
}

pub fn build(installer: &Installer<'_>, inputs: &mut Inputs, tools: &mut Tools<'_>) -> Result<()> {
    let Installer {
        settings,
        version,
        payload,
        has_icon,
        output,
        scratch,
    } = installer;
    let upgrade = upgrade_code(settings)?;
    let user = per_user(settings);
    let hive = if user { "HKCU" } else { "HKLM" };
    let other = if user { "HKLM" } else { "HKCU" };
    let key = xml(&registry_key(settings));
    let publisher = settings
        .config
        .publisher
        .as_deref()
        .context("MSI requires bundle.publisher")?;
    ensure!(
        !publisher.chars().any(char::is_control),
        "bundle.publisher must be a single line"
    );
    let mut source = format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<Wix xmlns="http://wixtoolset.org/schemas/v4/wxs" xmlns:ui="http://wixtoolset.org/schemas/v4/wxs/ui">
<Package Name="{name}" Manufacturer="{publisher}" Version="{version}" UpgradeCode="{upgrade}" Scope="{scope}" Language="1033" Codepage="65001" InstallerVersion="500">
  <SummaryInformation Codepage="1252" Description="{identifier} {version}" Manufacturer="scoop-bundle" Comments="Installs {identifier}." />
  <MediaTemplate EmbedCab="yes" />
  <MajorUpgrade AllowSameVersionUpgrades="yes" Schedule="afterInstallInitialize" DowngradeErrorMessage="A newer version of this application is already installed." />
  <Property Id="BUNDLE_EXISTING_INSTALLER" Secure="yes"><RegistrySearch Id="ExistingInstaller" Root="{hive}" Key="{key}" Name="Installer" Type="raw" Bitness="always64" /></Property>
  <Property Id="BUNDLE_OTHER_SCOPE" Secure="yes"><RegistrySearch Id="OtherScope" Root="{other}" Key="{key}" Name="Installer" Type="raw" Bitness="always64" /></Property>
  <Property Id="INSTALLFOLDER" Secure="yes"><RegistrySearch Id="ExistingDirectory" Root="{hive}" Key="{key}" Name="InstallDir" Type="raw" Bitness="always64" /></Property>
  <Launch Condition="Installed OR NOT BUNDLE_OTHER_SCOPE" Message="Uninstall the existing installation before changing its installation scope." />
  <Launch Condition="Installed OR NOT BUNDLE_EXISTING_INSTALLER OR BUNDLE_EXISTING_INSTALLER = &quot;msi&quot;" Message="Uninstall the existing installation before changing its installer format." />
  <ui:WixUI Id="WixUI_InstallDir" InstallDirectory="INSTALLFOLDER" />
  <UI>
    <Publish Dialog="WelcomeDlg" Control="Next" Event="NewDialog" Value="InstallDirDlg" Order="2" Condition="NOT Installed" />
    <Publish Dialog="InstallDirDlg" Control="Back" Event="NewDialog" Value="WelcomeDlg" Order="2" />
  </UI>
"#,
        name = xml(&settings.display_name),
        publisher = xml(publisher),
        identifier = settings.config.identifier,
        scope = if user { "perUser" } else { "perMachine" },
    );
    if *has_icon {
        writeln!(
            source,
            "  <Icon Id=\"AppIcon\" SourceFile=\"{}\" /><Property Id=\"ARPPRODUCTICON\" Value=\"AppIcon\" />",
            xml(&path_text(&payload.root.join(ICON))?)
        )?;
    }
    if user {
        writeln!(
            source,
            "  <StandardDirectory Id=\"LocalAppDataFolder\"><Directory Id=\"LocalProgramsFolder\" Name=\"Programs\"><Directory Id=\"INSTALLFOLDER\" Name=\"{}\" /></Directory></StandardDirectory>",
            settings.config.identifier
        )?;
    } else {
        writeln!(
            source,
            "  <StandardDirectory Id=\"ProgramFiles64Folder\"><Directory Id=\"INSTALLFOLDER\" Name=\"{}\" /></StandardDirectory>",
            settings.config.identifier
        )?;
    }
    let directory_id = |path: &Path| -> Result<String> {
        Ok(if path.as_os_str().is_empty() {
            "INSTALLFOLDER".into()
        } else {
            format!("d{}", identity(upgrade, user, "directory", path)?.simple())
        })
    };
    for directory in payload.directories.iter().skip(1) {
        writeln!(
            source,
            "  <DirectoryRef Id=\"{}\"><Directory Id=\"{}\" Name=\"{}\" /></DirectoryRef>",
            directory_id(directory.parent().context("directory has no parent")?)?,
            directory_id(directory)?,
            xml(directory
                .file_name()
                .context("directory has no name")?
                .to_str()
                .context("directory must be Unicode")?)
        )?;
    }
    let mut components = Vec::new();
    let mut component = |source: &mut String,
                         kind: &str,
                         path: &Path,
                         directory: &str,
                         contents: &str|
     -> Result<()> {
        let guid = identity(upgrade, user, kind, path)?;
        let id = format!("c{}", guid.simple());
        writeln!(
            source,
            "  <Component Id=\"{id}\" Guid=\"{guid}\" Directory=\"{directory}\" Bitness=\"always64\">{contents}"
        )?;
        if user {
            writeln!(
                source,
                "    <RegistryValue Root=\"HKCU\" Key=\"{key}\\Components\" Name=\"{id}\" Type=\"integer\" Value=\"1\" KeyPath=\"yes\" />"
            )?;
        }
        source.push_str("  </Component>\n");
        components.push(id);
        Ok(())
    };
    for file in &payload.files {
        let id = format!("f{}", identity(upgrade, user, "file", file)?.simple());
        let contents = format!(
            "<File Id=\"{id}\" Name=\"{}\" Source=\"{}\" KeyPath=\"{}\" />",
            xml(file
                .file_name()
                .context("file has no name")?
                .to_str()
                .context("filename must be Unicode")?),
            xml(&path_text(&payload.root.join(file))?),
            if user { "no" } else { "yes" }
        );
        component(
            &mut source,
            "file",
            file,
            &directory_id(file.parent().context("file has no directory")?)?,
            &contents,
        )?;
    }
    for directory in &payload.directories {
        let id = directory_id(directory)?;
        component(
            &mut source,
            "folder",
            directory,
            &id,
            &format!("<CreateFolder /><RemoveFolder Id=\"r{id}\" On=\"uninstall\" />"),
        )?;
    }
    let shortcuts = &settings.config.windows.msi;
    if shortcuts.start_menu_shortcut || shortcuts.desktop_shortcut {
        let main = xml(&format!("[INSTALLFOLDER]{}.exe", settings.name));
        let icon = if *has_icon { " Icon=\"AppIcon\"" } else { "" };
        for (enabled, location, directory) in [
            (
                shortcuts.start_menu_shortcut,
                "ProgramMenuFolder",
                "ApplicationProgramsFolder",
            ),
            (shortcuts.desktop_shortcut, "DesktopFolder", "DesktopFolder"),
        ] {
            if !enabled {
                continue;
            }
            if location == "ProgramMenuFolder" {
                writeln!(
                    source,
                    "  <StandardDirectory Id=\"ProgramMenuFolder\"><Directory Id=\"ApplicationProgramsFolder\" Name=\"{}\" /></StandardDirectory>",
                    settings.config.identifier
                )?;
            } else {
                source.push_str("  <StandardDirectory Id=\"DesktopFolder\" />\n");
            }
            let contents = format!(
                "<Shortcut Id=\"s{directory}\" Name=\"{}\" Target=\"{main}\" WorkingDirectory=\"INSTALLFOLDER\"{icon} />{}{}",
                xml(&settings.display_name),
                if location == "ProgramMenuFolder" {
                    "<RemoveFolder Id=\"RemoveProgramsFolder\" On=\"uninstall\" />"
                } else {
                    ""
                },
                if user {
                    String::new()
                } else {
                    format!(
                        "<RegistryValue Root=\"HKLM\" Key=\"{key}\\Shortcuts\" Name=\"{directory}\" Type=\"integer\" Value=\"1\" KeyPath=\"yes\" />"
                    )
                }
            );
            component(
                &mut source,
                "shortcut",
                Path::new(directory),
                directory,
                &contents,
            )?;
        }
    }
    let owner = identity(upgrade, user, "owner", Path::new(""))?;
    writeln!(
        source,
        "  <Component Id=\"InstallationOwner\" Guid=\"{owner}\" Directory=\"INSTALLFOLDER\" Bitness=\"always64\"><RegistryKey Root=\"{hive}\" Key=\"{key}\"><RegistryValue Name=\"Installer\" Type=\"string\" Value=\"msi\" KeyPath=\"yes\" /><RegistryValue Name=\"InstallDir\" Type=\"string\" Value=\"[INSTALLFOLDER]\" /><RegistryValue Name=\"ProductCode\" Type=\"string\" Value=\"[ProductCode]\" /><RegistryValue Name=\"Version\" Type=\"string\" Value=\"[ProductVersion]\" /></RegistryKey></Component>"
    )?;
    source.push_str(
        "  <Feature Id=\"Application\" Level=\"1\"><ComponentRef Id=\"InstallationOwner\" />\n",
    );
    for id in components {
        writeln!(source, "    <ComponentRef Id=\"{id}\" />")?;
    }
    source.push_str("  </Feature>\n</Package>\n</Wix>\n");
    let path = scratch.join("installer.wxs");
    fs::write(&path, source)?;
    // WiX 5 converts the CAB output directory to the system ANSI code page.
    // Keep it in TEMP instead of inheriting a Unicode application output path.
    let intermediate = tempfile::Builder::new()
        .prefix("scoop-bundle-wix-")
        .tempdir()?;
    let mut command = Command::new(tools.find("wix", inputs)?);
    command
        .args([
            "build", "-nologo", "-arch", "x64", "-pdbtype", "none", "-ext",
        ])
        .arg(path_text(&tools.find("wix-ui-extension", inputs)?)?);
    command
        .arg("-intermediateFolder")
        .arg(path_text(intermediate.path())?)
        .arg("-o")
        .arg(path_text(output)?)
        .arg(path_text(&path)?);
    tools::run(&mut command, "wix build").context(
        "WiX 5 CAB output requires TEMP to be representable in the Windows ANSI code page",
    )?;
    Ok(())
}
