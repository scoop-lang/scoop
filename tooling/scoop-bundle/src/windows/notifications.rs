use anyhow::{Result, ensure};
use uuid::Uuid;

use crate::config::Loaded;

pub(super) fn clsid(identifier: &str) -> String {
    format!(
        "{{{}}}",
        Uuid::new_v5(
            &Uuid::NAMESPACE_URL,
            format!("scoop-notification:{identifier}").as_bytes()
        )
    )
}

pub(super) fn validate(settings: &Loaded, format: &str) -> Result<()> {
    if settings.config.windows.notifications {
        let shortcut = match format {
            "nsis" => settings.config.windows.nsis.start_menu_shortcut,
            "msi" => settings.config.windows.msi.start_menu_shortcut,
            _ => false,
        };
        ensure!(
            shortcut,
            "bundle.windows.notifications requires the {format} start-menu-shortcut"
        );
    }
    Ok(())
}
