use scoop_identity::{CanonicalNativeLibraryName, NativeLibraryKind};
use scoop_toolchain::NativeLibraryOutput;

use super::*;

impl NativeInputs {
    pub(super) fn bind_outputs(
        &mut self,
        outputs: &[NativeLibraryOutput],
    ) -> Result<(), LinkError> {
        for output in outputs {
            let kind = match output.kind() {
                NativeLibraryKind::StaticArchive => NativeFileKind::Archive,
                NativeLibraryKind::Dynamic => NativeFileKind::Dylib,
                NativeLibraryKind::Framework => NativeFileKind::Framework,
                NativeLibraryKind::TargetDefault => {
                    unreachable!("produced libraries have an explicit kind")
                }
            };
            self.bind(&output.name, &output.binary, kind)?;
            for path in &output.dependencies {
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .and_then(|s| s.strip_prefix("lib"))
                    .ok_or_else(|| {
                        error(
                            scoop_source::DiagnosticCode::E2305,
                            format!("invalid produced dylib name {}", path.display()),
                        )
                    })?;
                self.bind(name, path, NativeFileKind::Dylib)?;
            }
        }
        Ok(())
    }

    fn bind(&mut self, name: &str, path: &Path, kind: NativeFileKind) -> Result<(), LinkError> {
        if let Some(previous) = self
            .bindings
            .insert(name.to_owned(), (path.to_owned(), kind))
            && previous != (path.to_owned(), kind)
        {
            return Err(error(
                scoop_source::DiagnosticCode::E2306,
                format!(
                    "conflicting explicit native outputs for {name}: {} and {}",
                    previous.0.display(),
                    path.display()
                ),
            ));
        }
        Ok(())
    }
}

pub(super) fn read(
    key: &NativeLinkRequirementKey,
    path: &Path,
    kind: NativeFileKind,
    profile: &ValidatedFinalLinkProfile,
) -> Result<locate::LibraryResolution, LinkError> {
    let compatible = matches!(
        (key.kind(), kind),
        (NativeLibraryKind::TargetDefault, _)
            | (NativeLibraryKind::StaticArchive, NativeFileKind::Archive)
            | (NativeLibraryKind::Dynamic, NativeFileKind::Dylib)
            | (NativeLibraryKind::Framework, NativeFileKind::Framework)
    );
    if !compatible {
        return Err(error(
            scoop_source::DiagnosticCode::E2306,
            format!(
                "produced native library {} is incompatible with {:?}",
                key.library().as_str(),
                key.kind()
            ),
        ));
    }
    Ok(locate::LibraryResolution {
        files: vec![locate::read(path, kind, profile, false)?],
        scripts: Vec::new(),
        system_alias: false,
    })
}

pub(super) fn settings(
    output: &NativeLibraryOutput,
    profile: &ValidatedFinalLinkProfile,
) -> Result<Vec<NativeLinkRequirementKey>, LinkError> {
    output
        .settings
        .libraries
        .iter()
        .map(|name| (name, NativeLibraryKind::TargetDefault))
        .chain(
            output
                .settings
                .frameworks
                .iter()
                .map(|name| (name, NativeLibraryKind::Framework)),
        )
        .map(|(name, kind)| {
            Ok(NativeLinkRequirementKey::for_target(
                profile.target().wire_id(),
                CanonicalNativeLibraryName::new(name)
                    .map_err(|cause| error(scoop_source::DiagnosticCode::E2305, cause))?,
                kind,
                NativeLibraryGrouping::Independent,
            ))
        })
        .collect()
}
