use std::{collections::BTreeSet, path::PathBuf};

use scoop_protocol::{CompilationPurpose, package::PackageMemberV1};

use super::{
    LoadError, LocalMemberInput, ProjectConfiguration, ProjectDiagnostic, WorkspaceInputs,
    artifacts::ArtifactInputs, files::FileInputs, graph::ProjectBuilder, normalize_path,
    package::PackageInputs,
};
use crate::AnalysisHost;

/// Keeps unchanged disk inputs; open documents are provided by the caller.
#[derive(Default)]
pub struct WorkspaceLoader {
    pub(super) files: FileInputs,
    pub(super) artifacts: ArtifactInputs,
}

impl WorkspaceLoader {
    pub fn load(
        &mut self,
        host: &mut AnalysisHost,
        configurations: &[ProjectConfiguration],
        overlays: &BTreeSet<PathBuf>,
        active_configuration: Option<String>,
    ) -> WorkspaceInputs {
        let mut inputs = WorkspaceInputs {
            active_configuration,
            ..WorkspaceInputs::default()
        };
        let mut missing_lockfiles = BTreeSet::new();
        for configuration in configurations {
            if let Err(error) = self.configuration(
                host,
                overlays,
                configuration,
                &mut missing_lockfiles,
                &mut inputs,
            ) {
                self.report(host, overlays, configuration, error, &mut inputs);
            }
        }
        inputs
    }

    fn configuration(
        &mut self,
        host: &mut AnalysisHost,
        overlays: &BTreeSet<PathBuf>,
        configuration: &ProjectConfiguration,
        missing_lockfiles: &mut BTreeSet<PathBuf>,
        inputs: &mut WorkspaceInputs,
    ) -> Result<(), LoadError> {
        let package = PackageInputs::new(configuration, host, overlays)?;
        let members = package.members(&configuration.root)?;
        let root = if configuration
            .root
            .file_name()
            .is_some_and(|name| name == "Cone.toml")
        {
            configuration.root.parent().expect("manifest has a parent")
        } else {
            &configuration.root
        };
        let workspace_root = members
            .root
            .to_path_buf()
            .map_err(|error| LoadError::new(scoop_source::DiagnosticCode::E2502, root, error))?;
        let all = normalize_path(root)
            .map_err(|error| LoadError::new(scoop_source::DiagnosticCode::E2201, root, error))?
            == workspace_root;
        let lockfile = configuration
            .lockfile
            .clone()
            .unwrap_or_else(|| workspace_root.join("Cone.lock"));
        let initialize_lockfile = missing_lockfiles.contains(&lockfile)
            || !lockfile.try_exists().map_err(|error| {
                LoadError::new(scoop_source::DiagnosticCode::E2201, &lockfile, error)
            })?;
        if initialize_lockfile {
            // All members and configurations in this load need their own lock records.
            missing_lockfiles.insert(lockfile.clone());
        }
        let mut selected = members
            .members
            .iter()
            .filter(|member| all || member.selected)
            .collect::<Vec<_>>();
        let locked_purposes = if configuration.purpose.is_none() && !initialize_lockfile {
            match package.locked_purposes(&lockfile) {
                Ok(purposes) => Some(purposes),
                Err(error) => {
                    self.report(host, overlays, configuration, error, inputs);
                    for member in selected {
                        self.keep_local_member(host, overlays, configuration, member, true, inputs);
                    }
                    return Ok(());
                }
            }
        } else {
            None
        };
        if let Some(purposes) = &locked_purposes {
            // Load actual root graphs before checking for dependency-only members.
            selected.sort_by_key(|member| !purposes.contains_key(&member.path));
        }
        let first_project = inputs.projects.len();
        let default_purpose = [configuration.purpose.unwrap_or_default()];
        for member in selected {
            let directory = member.directory.to_path_buf().map_err(|error| {
                LoadError::new(scoop_source::DiagnosticCode::E2502, root, error)
            })?;
            let mut member_configuration = configuration.clone();
            member_configuration.root = directory;
            let locked = locked_purposes
                .as_ref()
                .and_then(|purposes| purposes.get(&member.path));
            let without_root = locked_purposes.is_some() && locked.is_none();
            if without_root
                && inputs.projects[first_project..].iter().any(|project| {
                    project.complete
                        && project
                            .cones
                            .values()
                            .filter_map(|cone| cone.source())
                            .any(|cone| {
                                cone.manifest.real_root() == member_configuration.root
                                    && cone.manifest.coordinate() == &member.coordinate
                            })
                })
            {
                continue;
            }

            let mut failed_purpose = None;
            for &purpose in locked.map(Vec::as_slice).unwrap_or(&default_purpose) {
                let result = package
                    .load_member(
                        &member_configuration.root,
                        member,
                        purpose,
                        initialize_lockfile,
                    )
                    .and_then(|resolution| {
                        ProjectBuilder::load(
                            self,
                            host,
                            overlays,
                            member_configuration.clone(),
                            resolution,
                        )
                    });
                match result {
                    Ok((project, errors)) => {
                        inputs.projects.push(project);
                        for error in errors {
                            self.report(host, overlays, &member_configuration, error, inputs);
                        }
                    }
                    Err(error) => {
                        failed_purpose = Some(purpose);
                        self.report(host, overlays, &member_configuration, error, inputs);
                    }
                }
            }
            if let Some(purpose) = failed_purpose {
                self.keep_local_member(
                    host,
                    overlays,
                    &member_configuration,
                    member,
                    without_root || purpose == CompilationPurpose::Test,
                    inputs,
                );
            }
        }
        Ok(())
    }

    fn keep_local_member(
        &mut self,
        host: &mut AnalysisHost,
        overlays: &BTreeSet<PathBuf>,
        configuration: &ProjectConfiguration,
        member: &PackageMemberV1,
        include_tests: bool,
        inputs: &mut WorkspaceInputs,
    ) {
        match self
            .files
            .member_sources(host, overlays, member, configuration, include_tests)
        {
            Ok(sources) => inputs.unresolved_members.push(LocalMemberInput {
                configuration: configuration.name.clone(),
                sources,
            }),
            Err(error) => self.report(host, overlays, configuration, error, inputs),
        }
    }

    fn report(
        &mut self,
        host: &mut AnalysisHost,
        overlays: &BTreeSet<PathBuf>,
        configuration: &ProjectConfiguration,
        error: LoadError,
        inputs: &mut WorkspaceInputs,
    ) {
        let error = diagnostic(configuration, error);
        // The text is needed for diagnostic positions, never for a second parse.
        let _ = self.files.read(host, overlays, &error.path);
        inputs.diagnostics.push(error);
    }
}

fn diagnostic(
    configuration: &ProjectConfiguration,
    mut error: super::LoadError,
) -> ProjectDiagnostic {
    if error
        .path
        .file_name()
        .is_none_or(|name| name != "Cone.toml")
    {
        error.message = format!("{}: {}", error.path.display(), error.message);
        error.path = if configuration
            .root
            .file_name()
            .is_some_and(|name| name == "Cone.toml")
        {
            configuration.root.clone()
        } else {
            configuration.root.join("Cone.toml")
        };
        error.span = None;
    }
    ProjectDiagnostic {
        notes: error.notes,
        code: error.code,
        configuration: configuration.name.clone(),
        path: error.path,
        span: error.span,
        message: error.message,
    }
}
