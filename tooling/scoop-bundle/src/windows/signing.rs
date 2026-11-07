use std::{ffi::OsString, fs, path::Path, process::Command};

use anyhow::{Context, Result, ensure};

use crate::{
    cache::Inputs,
    config::{CertificateStore, Loaded, WindowsSigning},
    tools::Tools,
};

fn settings(settings: &Loaded) -> Result<(&WindowsSigning, String)> {
    let signing = settings
        .config
        .windows
        .signing
        .as_ref()
        .context("missing Windows signing configuration")?;
    let thumbprint = signing
        .certificate_thumbprint
        .chars()
        .filter(|c| !c.is_ascii_whitespace())
        .collect::<String>();
    ensure!(
        thumbprint.len() == 40 && thumbprint.bytes().all(|b| b.is_ascii_hexdigit()),
        "certificate-thumbprint must be the certificate's 40-digit SHA-1 thumbprint"
    );
    ensure!(
        (signing.timestamp_url.starts_with("https://")
            || signing.timestamp_url.starts_with("http://"))
            && !signing.timestamp_url.contains('"')
            && !signing.timestamp_url.chars().any(char::is_whitespace)
            && !signing.timestamp_url.chars().any(char::is_control),
        "timestamp-url must be an HTTP(S) URL"
    );
    Ok((signing, thumbprint))
}

pub fn sign(
    config: &Loaded,
    path: &Path,
    inputs: &mut Inputs,
    tools: &mut Tools<'_>,
) -> Result<()> {
    let (signing, thumbprint) = settings(config)?;
    let mut arguments = [
        "sign",
        "/fd",
        "SHA256",
        "/td",
        "SHA256",
        "/s",
        "My",
        "/sha1",
        &thumbprint,
        "/tr",
        &signing.timestamp_url,
    ]
    .into_iter()
    .map(OsString::from)
    .collect::<Vec<_>>();
    if signing.certificate_store == CertificateStore::LocalMachine {
        arguments.push("/sm".into());
    }
    arguments.push(path.as_os_str().to_owned());
    #[cfg(windows)]
    #[allow(clippy::permissions_set_readonly_false)]
    // Only clears the Windows read-only attribute.
    let permissions = {
        let permissions = fs::metadata(path)?.permissions();
        let mut writable = permissions.clone();
        writable.set_readonly(false);
        fs::set_permissions(path, writable)?;
        permissions
    };
    tools.run("signtool", arguments, inputs)?;
    tools.run(
        "signtool",
        [
            std::ffi::OsStr::new("verify"),
            "/pa".as_ref(),
            "/all".as_ref(),
            path.as_os_str(),
        ],
        inputs,
    )?;
    #[cfg(windows)]
    fs::set_permissions(path, permissions)?;
    Ok(())
}

pub fn configure_uninstaller(
    config: &Loaded,
    scratch: &Path,
    command: &mut Command,
    inputs: &mut Inputs,
    tools: &mut Tools<'_>,
) -> Result<()> {
    let (signing, thumbprint) = settings(config)?;
    let script = scratch.join("sign-uninstaller.cmd");
    fs::write(
        &script,
        format!(
            "@echo off\r\nsetlocal DisableDelayedExpansion\r\n\"%SCOOP_BUNDLE_SIGNTOOL%\" sign /fd SHA256 /td SHA256 /s My /sha1 {thumbprint} /tr \"%SCOOP_BUNDLE_TIMESTAMP%\" {} \"%~1\"\r\nif errorlevel 1 exit /b 1\r\n\"%SCOOP_BUNDLE_SIGNTOOL%\" verify /pa /all \"%~1\"\r\nexit /b %errorlevel%\r\n",
            if signing.certificate_store == CertificateStore::LocalMachine {
                "/sm"
            } else {
                ""
            }
        ),
    )?;
    command.env("SCOOP_BUNDLE_SIGN_SCRIPT", super::path_text(&script)?);
    command.env(
        "SCOOP_BUNDLE_SIGNTOOL",
        super::path_text(&tools.find("signtool", inputs)?)?,
    );
    command.env("SCOOP_BUNDLE_TIMESTAMP", &signing.timestamp_url);
    Ok(())
}
