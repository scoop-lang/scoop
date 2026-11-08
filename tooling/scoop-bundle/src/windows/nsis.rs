use std::{fmt::Write, fs, process::Command};

use anyhow::Result;

use super::{
    ICON, Installer, notifications, path_text, per_user, registry_key, relative_text, signing,
};
use crate::{
    cache::Inputs,
    tools::{self, Tools},
};

fn quoted(value: &str) -> String {
    literal(&value.replace('$', "$$"))
}

fn literal(value: &str) -> String {
    format!(
        "\"{}\"",
        value
            .replace('$', "${U+24}")
            .replace('"', "$\\\"")
            .replace('\r', "$\\r")
            .replace('\n', "$\\n")
            .replace('\t', "$\\t")
    )
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
    let user = per_user(settings);
    let hive = if user { "HKCU" } else { "HKLM" };
    let other = if user { "HKLM" } else { "HKCU" };
    let shell = if user { "current" } else { "all" };
    let key = quoted(&registry_key(settings));
    let uninstall_key = quoted(&format!(
        "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{}",
        settings.config.identifier
    ));
    let name = quoted(&settings.display_name);
    let id = &settings.config.identifier;
    let main = format!("{}.exe", settings.name);
    let display_version = settings.version.to_string();
    let mut script = format!(
        r#"Unicode true
ManifestSupportedOS all
!include "MUI2.nsh"
!include "LogicLib.nsh"
!include "x64.nsh"
!include "WordFunc.nsh"
Name {name}
OutFile {out}
RequestExecutionLevel {level}
InstallDir "{base}\{id}"
InstallDirRegKey {hive} {key} "InstallDir"
SetCompressor /SOLID lzma
VIProductVersion "{version}"
VIAddVersionKey "ProductName" {version_name}
VIAddVersionKey "ProductVersion" {display_version}
VIAddVersionKey "FileVersion" "{version}"
VIAddVersionKey "FileDescription" {version_name}
VIAddVersionKey "LegalCopyright" {publisher}
Var PreviousDirectory
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"
Function .onInit
  SetRegView 64
  SetShellVarContext {shell}
  ${{IfNot}} ${{RunningX64}}
    MessageBox MB_OK|MB_ICONSTOP "This application requires 64-bit Windows." /SD IDOK
    SetErrorLevel 1633
    Abort
  ${{EndIf}}
  ReadRegStr $R0 {other} {key} "Installer"
  ${{If}} $R0 != ""
    MessageBox MB_OK|MB_ICONSTOP "Uninstall the existing installation before changing its installation scope." /SD IDOK
    SetErrorLevel 1638
    Abort
  ${{EndIf}}
  ReadRegStr $R0 {hive} {key} "Installer"
  ${{If}} $R0 != ""
  ${{AndIf}} $R0 != "nsis"
    MessageBox MB_OK|MB_ICONSTOP "Uninstall the existing installation before changing its installer format." /SD IDOK
    SetErrorLevel 1638
    Abort
  ${{EndIf}}
  ReadRegStr $PreviousDirectory {hive} {key} "InstallDir"
  ReadRegStr $R0 {hive} {key} "FileVersion"
  ${{If}} $R0 != ""
    ${{VersionCompare}} $R0 "{version}" $R1
    ${{If}} $R1 == 1
      MessageBox MB_OK|MB_ICONSTOP "A newer version of this application is already installed." /SD IDOK
      SetErrorLevel 1638
      Abort
    ${{EndIf}}
  ${{EndIf}}
FunctionEnd
Section "Install"
  SetRegView 64
  SetShellVarContext {shell}
  ${{If}} $PreviousDirectory != ""
    ClearErrors
    ExecWait '"$PreviousDirectory\uninstall.exe" /S _?=$PreviousDirectory' $R0
    ${{If}} ${{Errors}}
    ${{OrIf}} $R0 != 0
      MessageBox MB_OK|MB_ICONSTOP "The previous installation could not be removed." /SD IDOK
      SetErrorLevel 1
      Abort
    ${{EndIf}}
    Delete "$PreviousDirectory\uninstall.exe"
    RMDir "$PreviousDirectory"
  ${{EndIf}}
  SetOverwrite on
"#,
        out = literal(&path_text(output)?),
        version_name = literal(&settings.display_name),
        level = if user { "user" } else { "admin" },
        base = if user {
            "$LOCALAPPDATA\\Programs"
        } else {
            "$PROGRAMFILES64"
        },
        display_version = quoted(&display_version),
        publisher = literal(settings.config.publisher.as_deref().unwrap_or("")),
    );
    for directory in &payload.directories {
        writeln!(
            script,
            "  CreateDirectory \"$INSTDIR\\{}\"",
            escaped_path(directory)?
        )?;
    }
    for file in &payload.files {
        writeln!(
            script,
            "  SetOutPath \"$INSTDIR\\{}\"",
            escaped_path(file.parent().expect("relative file directory"))?
        )?;
        writeln!(
            script,
            "  ClearErrors\n  File {} {}",
            quoted(&format!(
                "/oname={}",
                file.file_name()
                    .expect("relative filename")
                    .to_str()
                    .expect("validated Unicode filename")
            )),
            literal(&path_text(&payload.root.join(file))?)
        )?;
        script.push_str("  ${If} ${Errors}\n    SetErrorLevel 1\n    Abort\n  ${EndIf}\n");
    }
    writeln!(
        script,
        r#"  SetOutPath "$INSTDIR"
  WriteUninstaller "$INSTDIR\uninstall.exe"
  WriteRegStr {hive} {key} "Installer" "nsis"
  WriteRegStr {hive} {key} "InstallDir" "$INSTDIR"
  WriteRegStr {hive} {key} "FileVersion" "{version}"
  WriteRegStr {hive} {uninstall_key} "DisplayName" {name}
  WriteRegStr {hive} {uninstall_key} "DisplayVersion" {display_version}
  WriteRegStr {hive} {uninstall_key} "Publisher" {publisher}
  WriteRegStr {hive} {uninstall_key} "InstallLocation" "$INSTDIR"
  WriteRegStr {hive} {uninstall_key} "UninstallString" '$\"$INSTDIR\uninstall.exe$\"'
  WriteRegStr {hive} {uninstall_key} "QuietUninstallString" '$\"$INSTDIR\uninstall.exe$\" /S'
  WriteRegStr {hive} {uninstall_key} "DisplayIcon" "$INSTDIR\{icon}"
  WriteRegDWORD {hive} {uninstall_key} "NoModify" 1
  WriteRegDWORD {hive} {uninstall_key} "NoRepair" 1
"#,
        display_version = quoted(&display_version),
        publisher = quoted(settings.config.publisher.as_deref().unwrap_or("")),
        icon = escaped_path(std::path::Path::new(if *has_icon { ICON } else { &main }))?,
    )?;
    let start_menu = settings.config.windows.nsis.start_menu_shortcut;
    let desktop = settings.config.windows.nsis.desktop_shortcut;
    let shortcut = quoted(&settings.display_name);
    let shortcut = &shortcut[1..shortcut.len() - 1];
    let main_escaped = quoted(&main);
    let main_escaped = &main_escaped[1..main_escaped.len() - 1];
    let icon_arguments = if *has_icon {
        format!(" \"\" \"$INSTDIR\\{ICON}\" 0")
    } else {
        String::new()
    };
    if start_menu {
        writeln!(
            script,
            "  CreateDirectory \"$SMPROGRAMS\\{id}\"\n  CreateShortcut \"$SMPROGRAMS\\{id}\\{shortcut}.lnk\" \"$INSTDIR\\{main_escaped}\"{icon_arguments}"
        )?;
        set_shortcut_identity(&mut script, &format!("$SMPROGRAMS\\{id}\\{shortcut}.lnk"))?;
    }
    if desktop {
        writeln!(
            script,
            "  CreateShortcut \"$DESKTOP\\{shortcut}.lnk\" \"$INSTDIR\\{main_escaped}\"{icon_arguments}"
        )?;
        set_shortcut_identity(&mut script, &format!("$DESKTOP\\{shortcut}.lnk"))?;
    }
    if settings.config.windows.notifications {
        let clsid = notifications::clsid(id);
        writeln!(
            script,
            r#"  WriteRegStr {hive} "Software\Classes\AppUserModelId\{id}" "DisplayName" {name}
  WriteRegStr {hive} "Software\Classes\AppUserModelId\{id}" "CustomActivator" "{clsid}"
  WriteRegStr {hive} "Software\Classes\CLSID\{clsid}\LocalServer32" "" '$\"$INSTDIR\{main_escaped}$\" -Embedding'"#
        )?;
        if *has_icon {
            writeln!(
                script,
                "  WriteRegStr {hive} \"Software\\Classes\\AppUserModelId\\{id}\" \"IconUri\" \"$INSTDIR\\{ICON}\""
            )?;
        }
    }
    writeln!(
        script,
        "SectionEnd\nSection \"Uninstall\"\n  SetRegView 64\n  SetShellVarContext {shell}"
    )?;
    if settings.config.windows.notifications {
        writeln!(
            script,
            "  DeleteRegKey {hive} \"Software\\Classes\\AppUserModelId\\{id}\"\n  DeleteRegKey {hive} \"Software\\Classes\\CLSID\\{}\"",
            notifications::clsid(id)
        )?;
    }
    for (index, file) in payload.files.iter().enumerate() {
        writeln!(
            script,
            "  IfFileExists \"$INSTDIR\\{}\" 0 file_removed_{index}\n  ClearErrors\n  Delete \"$INSTDIR\\{}\"\n  ${{If}} ${{Errors}}\n    SetErrorLevel 1\n    Abort\n  ${{EndIf}}\nfile_removed_{index}:",
            escaped_path(file)?,
            escaped_path(file)?
        )?;
    }
    if start_menu {
        writeln!(
            script,
            "  Delete \"$SMPROGRAMS\\{id}\\{shortcut}.lnk\"\n  RMDir \"$SMPROGRAMS\\{id}\""
        )?;
    }
    if desktop {
        writeln!(script, "  Delete \"$DESKTOP\\{shortcut}.lnk\"")?;
    }
    script.push_str("  Delete \"$INSTDIR\\uninstall.exe\"\n");
    for directory in payload.directories.iter().rev() {
        writeln!(script, "  RMDir \"$INSTDIR\\{}\"", escaped_path(directory)?)?;
    }
    writeln!(
        script,
        "  DeleteRegKey {hive} {key}\n  DeleteRegKey {hive} {uninstall_key}\nSectionEnd"
    )?;
    if *has_icon {
        writeln!(
            script,
            "Icon {}\nUninstallIcon {}",
            literal(&path_text(&payload.root.join(ICON))?),
            literal(&path_text(&payload.root.join(ICON))?)
        )?;
    }
    if settings.config.windows.signing.is_some() {
        script.push_str("!uninstfinalize '\"\"%SCOOP_BUNDLE_SIGN_SCRIPT%\" \"%1\"\"' = 0\n");
    }
    if start_menu || desktop {
        script.push_str(&shortcut_function(
            id,
            settings.config.windows.notifications,
        ));
    }
    let path = scratch.join("installer.nsi");
    fs::write(&path, script)?;
    let mut command = Command::new(tools.find("makensis", inputs)?);
    command.args(["/V2", "/INPUTCHARSET", "UTF8"]).arg(&path);
    if settings.config.windows.signing.is_some() {
        signing::configure_uninstaller(settings, scratch, &mut command, inputs, tools)?;
    }
    tools::run(&mut command, "makensis")?;
    Ok(())
}

fn escaped_path(path: &std::path::Path) -> Result<String> {
    let quoted = quoted(&relative_text(path)?);
    Ok(quoted[1..quoted.len() - 1].into())
}

fn set_shortcut_identity(script: &mut String, path: &str) -> Result<()> {
    writeln!(
        script,
        "  Push \"{path}\"\n  Call SetShortcutIdentity\n  Pop $R0\n  ${{If}} $R0 < 0\n    DetailPrint \"Could not set the shortcut application identity: $R0\"\n    SetErrorLevel 1\n    Abort\n  ${{EndIf}}"
    )?;
    Ok(())
}

pub(super) fn shortcut_function(id: &str, notification: bool) -> String {
    let activator = if notification {
        format!(
            r#"  System::Call 'ole32::PropVariantClear(p r4)'
  System::Call 'propsys::PSGetPropertyKeyFromName(w "System.AppUserModel.ToastActivatorCLSID", p r3) i.r5'
  ${{If}} $5 < 0
    Goto identity_done
  ${{EndIf}}
  System::Call 'propsys::InitPropVariantFromCLSID(g "{}", p r4) i.r5'
  ${{If}} $5 < 0
    Goto identity_done
  ${{EndIf}}
  System::Call '$2->6(p r3, p r4) i.r5'
  ${{If}} $5 < 0
    Goto identity_done
  ${{EndIf}}
"#,
            notifications::clsid(id)
        )
    } else {
        String::new()
    };
    include_str!("shortcut.nsh")
        .replace("@SCOOP_APPID@", &quoted(id))
        .replace("@SCOOP_ACTIVATOR@", &activator)
}
