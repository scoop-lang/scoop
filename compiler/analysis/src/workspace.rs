//! Local project inputs selected by the package manager and artifact readers.

use std::{collections::BTreeMap, path::PathBuf, sync::Arc};

use scoop_identity::{ConeIdentity, SourceIdentity, SourceRole};
use scoop_lir::ValidatedLirTargetSelection;
use scoop_manifest::LoadedConeManifest;
use scoop_protocol::CompilationPurpose;
use scoop_source::{SourceId, Span};

mod artifacts;
mod attached_sources;
mod files;
mod graph;
mod loader;
mod package;
mod sources;

pub use files::normalize_path;
pub use loader::WorkspaceLoader;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectConfiguration {
    pub name: String,
    pub root: PathBuf,
    pub target: ValidatedLirTargetSelection,
    /// None selects the purposes recorded for each member in the lockfile.
    pub purpose: Option<CompilationPurpose>,
    pub sysroot: PathBuf,
    pub artifact_search_roots: Vec<PathBuf>,
    pub package_manager: PathBuf,
    pub package_cache: Option<PathBuf>,
    pub lockfile: Option<PathBuf>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectDiagnostic {
    pub notes: Vec<scoop_protocol::DiagnosticNoteV1>,
    pub code: String,
    pub configuration: String,
    pub path: PathBuf,
    pub span: Option<Span>,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectSource {
    pub source: SourceId,
    pub identity: SourceIdentity,
    pub path: PathBuf,
    pub role: SourceRole,
}

#[derive(Clone, Debug)]
pub struct SourceConeInput {
    pub manifest: Arc<LoadedConeManifest>,
    pub sources: Vec<ProjectSource>,
    pub test_dependencies: Vec<ConeIdentity>,
}

impl SourceConeInput {
    pub(crate) fn compiler_inputs(&self) -> scoop_toolchain::CompileTimeInputs {
        scoop_toolchain::CompileTimeInputs::default().with_sources(
            self.manifest.input_root().to_owned(),
            self.sources
                .iter()
                .map(|source| (source.identity.clone(), source.path.clone())),
        )
    }
}

impl PartialEq for SourceConeInput {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.manifest, &other.manifest)
            && self.sources == other.sources
            && self.test_dependencies == other.test_dependencies
    }
}
impl Eq for SourceConeInput {}

#[derive(Clone, Debug)]
pub struct ArtifactConeInput {
    pub path: PathBuf,
    pub file: scoop_slib::ArtifactFile,
    pub summary: Arc<scoop_slib::ArtifactManifestSummaryV1>,
    stamp: files::FileStamp,
}

impl PartialEq for ArtifactConeInput {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path && self.stamp == other.stamp
    }
}
impl Eq for ArtifactConeInput {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConeInput {
    Source(SourceConeInput),
    Artifact(ArtifactConeInput),
}

impl ConeInput {
    pub fn coordinate(&self) -> &scoop_identity::ConeCoordinate {
        match self {
            Self::Source(source) => source.manifest.coordinate(),
            Self::Artifact(artifact) => artifact.summary.cone().coordinate(),
        }
    }

    pub fn source(&self) -> Option<&SourceConeInput> {
        match self {
            Self::Source(source) => Some(source),
            Self::Artifact(_) => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectInput {
    pub configuration: ProjectConfiguration,
    pub purpose: CompilationPurpose,
    pub root: ConeIdentity,
    pub cones: BTreeMap<ConeIdentity, ConeInput>,
    pub artifact_sources: BTreeMap<SourceIdentity, ProjectSource>,
    pub dependencies: BTreeMap<ConeIdentity, Vec<ConeIdentity>>,
    pub dependency_order: Vec<ConeIdentity>,
    pub complete: bool,
}

impl ProjectInput {
    pub(crate) fn source_file_by_id(&self, source: SourceId) -> Option<&ProjectSource> {
        self.source_cone(source)
            .and_then(|(_, cone)| cone.sources.iter().find(|file| file.source == source))
            .or_else(|| {
                self.artifact_sources
                    .values()
                    .find(|file| file.source == source)
            })
    }

    /// Navigation attachments do not become source dependency inputs.
    pub fn source_file(&self, identity: &SourceIdentity) -> Option<&ProjectSource> {
        self.cones
            .get(&identity.cone())
            .and_then(ConeInput::source)
            .and_then(|cone| cone.sources.iter().find(|file| &file.identity == identity))
            .or_else(|| self.artifact_sources.get(identity))
    }

    pub fn source_cone(&self, source: SourceId) -> Option<(ConeIdentity, &SourceConeInput)> {
        self.cones.iter().find_map(|(identity, cone)| {
            let input = cone.source()?;
            input
                .sources
                .iter()
                .any(|file| file.source == source)
                .then_some((*identity, input))
        })
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WorkspaceInputs {
    pub projects: Vec<ProjectInput>,
    /// Local syntax stays available when a member has no usable locked graph.
    pub unresolved_members: Vec<LocalMemberInput>,
    pub diagnostics: Vec<ProjectDiagnostic>,
    pub active_configuration: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalMemberInput {
    pub configuration: String,
    pub sources: Vec<ProjectSource>,
}

impl WorkspaceInputs {
    /// The selected configuration owns semantic results; text can be shared.
    pub fn project_for_source(&self, source: SourceId) -> Option<&ProjectInput> {
        let candidates = self.projects.iter().filter_map(|project| {
            let (cone, _) = project.source_cone(source)?;
            Some((project, cone == project.root))
        });
        if let Some(active) = &self.active_configuration {
            let selected = preferred_project(
                candidates
                    .clone()
                    .filter(|(project, _)| project.configuration.name == *active),
            );
            if let Some((project, true)) = selected {
                return Some(project);
            }
            if self.unresolved_members.iter().any(|member| {
                member.configuration == *active
                    && member.sources.iter().any(|file| file.source == source)
            }) {
                return None;
            }
            if !self
                .projects
                .iter()
                .any(|project| project.configuration.name == *active)
                && !self
                    .unresolved_members
                    .iter()
                    .any(|member| member.configuration == *active)
            {
                return None;
            }
            if let Some((project, _)) = selected {
                return Some(project);
            }
        }
        preferred_project(candidates).map(|(project, _)| project)
    }

    pub(crate) fn unresolved_for_source(&self, source: SourceId) -> Option<&LocalMemberInput> {
        if self.project_for_source(source).is_some() {
            return None;
        }
        let mut candidates = self
            .unresolved_members
            .iter()
            .filter(|member| member.sources.iter().any(|file| file.source == source));
        if let Some(active) = &self.active_configuration {
            if let Some(member) = candidates
                .clone()
                .find(|member| member.configuration == *active)
            {
                return Some(member);
            }
            if !self
                .projects
                .iter()
                .any(|project| project.configuration.name == *active)
                && !self
                    .unresolved_members
                    .iter()
                    .any(|member| member.configuration == *active)
            {
                return None;
            }
        }
        candidates.next()
    }
}

fn preferred_project<'a>(
    mut candidates: impl Iterator<Item = (&'a ProjectInput, bool)>,
) -> Option<(&'a ProjectInput, bool)> {
    let first = candidates.next()?;
    let mut best = first;
    for candidate in std::iter::once(first).chain(
        candidates
            .take_while(|(project, _)| project.configuration.name == first.0.configuration.name),
    ) {
        if candidate.1 && candidate.0.complete {
            return Some(candidate);
        }
        if (candidate.1, candidate.0.complete) > (best.1, best.0.complete) {
            best = candidate;
        }
    }
    Some(best)
}

#[derive(Clone, Debug)]
struct LoadError {
    notes: Vec<scoop_protocol::DiagnosticNoteV1>,
    code: String,
    path: PathBuf,
    span: Option<Span>,
    message: String,
}

impl LoadError {
    fn new(
        code: scoop_source::DiagnosticCode,
        path: impl Into<PathBuf>,
        message: impl std::fmt::Display,
    ) -> Self {
        Self {
            code: code.to_string(),
            notes: Vec::new(),
            path: path.into(),
            span: None,
            message: message.to_string(),
        }
    }

    fn with_span(mut self, range: std::ops::Range<usize>) -> Self {
        self.span = u32::try_from(range.start)
            .ok()
            .zip(u32::try_from(range.end).ok())
            .map(|(start, end)| Span { start, end });
        self
    }
}
