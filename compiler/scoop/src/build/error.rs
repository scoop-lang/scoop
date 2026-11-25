use std::fmt;
use std::ops::Range;
use std::path::{Path, PathBuf};

use scoop_manifest::{ManifestRootError, ManifestRootErrorKind};
use scoop_protocol::StructuredDiagnosticV1;
use scoop_slib::{ArtifactManifestSummaryError, SlibDiagnostic};

use crate::{
    BuildFailurePhase, BuildGraphDiscoveryError, BuildGraphExecutionFailure, ClassifyBuildFailure,
    DependencyLocatorError, LoadBuildRootError, PrepareBuildGraphError, SelectedSource,
};

#[derive(Debug)]
pub struct BuildFailure {
    pub message: String,
    pub notes: Vec<scoop_protocol::DiagnosticNoteV1>,
    pub code: String,
    pub phase: BuildFailurePhase,
    pub location: BuildFailureLocation,
    pub diagnostics: Vec<StructuredDiagnosticV1>,
    pub warnings: Vec<StructuredDiagnosticV1>,
    pub sources: Vec<SelectedSource>,
}

#[derive(Debug)]
pub enum BuildFailureLocation {
    None,
    Host {
        path: PathBuf,
        span: Option<Range<usize>>,
    },
    Artifact {
        path: PathBuf,
        member: String,
    },
}

impl BuildFailure {
    pub fn toolchain(
        phase: BuildFailurePhase,
        error: scoop_toolchain::ToolchainError,
    ) -> Box<Self> {
        let mut failure = Self::tool(error.code(), phase, &error.0);
        failure.notes = error.2.iter().map(tool_note).collect();
        failure
    }
    pub fn link(error: scoop_linker::LinkError) -> Box<Self> {
        let mut failure = Self::tool(error.code(), BuildFailurePhase::FinalLink, &error.0);
        failure.notes = error.2.iter().map(tool_note).collect();
        failure
    }
    pub fn from_diagnostics(
        message: impl fmt::Display,
        phase: BuildFailurePhase,
        diagnostics: Vec<StructuredDiagnosticV1>,
    ) -> Box<Self> {
        let code = diagnostics
            .first()
            .expect("a failed compilation supplies diagnostics")
            .code()
            .to_owned();
        Box::new(Self {
            notes: Vec::new(),
            message: message.to_string(),
            code,
            phase,
            location: BuildFailureLocation::None,
            diagnostics,
            warnings: Vec::new(),
            sources: Vec::new(),
        })
    }

    pub fn tool(
        code: scoop_source::DiagnosticCode,
        phase: BuildFailurePhase,
        message: impl fmt::Display,
    ) -> Box<Self> {
        Box::new(Self {
            notes: Vec::new(),
            message: message.to_string(),
            code: code.to_string(),
            phase,
            location: BuildFailureLocation::None,
            diagnostics: Vec::new(),
            warnings: Vec::new(),
            sources: Vec::new(),
        })
    }

    pub fn at_artifact(mut self: Box<Self>, path: &Path, member: impl Into<String>) -> Box<Self> {
        self.location = BuildFailureLocation::Artifact {
            path: path.to_owned(),
            member: member.into(),
        };
        self
    }

    pub fn with_help(
        mut self: Box<Self>,
        code: scoop_source::DiagnosticCode,
        message: impl Into<String>,
    ) -> Box<Self> {
        self.notes
            .push(tool_note(&scoop_source::DiagnosticNote::without_location(
                code, message,
            )));
        self
    }

    pub fn at_host(mut self: Box<Self>, path: &Path) -> Box<Self> {
        self.location = BuildFailureLocation::Host {
            path: path.to_owned(),
            span: None,
        };
        self
    }

    pub fn classified(error: impl std::error::Error + ClassifyBuildFailure + 'static) -> Box<Self> {
        let classification = error.classification();
        // Deferred tool selection retains the same source-preparation diagnostic.
        if classification.phase() == BuildFailurePhase::SourceSnapshot
            && let Some(source) = scoop_toolchain::ToolchainError::in_chain(&error)
        {
            return Self::toolchain(classification.phase(), source.clone());
        }
        let notes = scoop_toolchain::ToolchainError::in_chain(&error)
            .map(|error| error.2.iter().map(tool_note).collect())
            .unwrap_or_default();
        let mut failure = Self::tool(
            classification
                .code()
                .expect("child diagnostics are handled by execution"),
            classification.phase(),
            error,
        );
        failure.notes = notes;
        failure
    }

    pub(crate) fn root(error: LoadBuildRootError) -> Box<Self> {
        let location = match &error {
            LoadBuildRootError::SingleFile(source) => BuildFailureLocation::Host {
                path: source.path().to_owned(),
                span: source.span(),
            },
        };
        let mut failure = Self::classified(error);
        failure.location = location;
        failure
    }

    pub(crate) fn discovery(error: BuildGraphDiscoveryError) -> Box<Self> {
        let location = match &error {
            BuildGraphDiscoveryError::Locator(error) => locator_location(error),
            _ => BuildFailureLocation::None,
        };
        let mut failure = Self::classified(error);
        failure.location = location;
        failure
    }

    pub(crate) fn preparation(error: PrepareBuildGraphError) -> Box<Self> {
        let path: Option<&Path> = match &error {
            PrepareBuildGraphError::SourceDiscovery(error) => Some(error.path()),
            PrepareBuildGraphError::SingleFile(error) => Some(error.path()),
            PrepareBuildGraphError::Manifest(error) => Some(error.path()),
            _ => None,
        };
        let path = path.map(Path::to_owned);
        let failure = Self::classified(error);
        match path {
            Some(path) => failure.at_host(&path),
            None => failure,
        }
    }

    pub(crate) fn execution(
        error: BuildGraphExecutionFailure,
        sources: &[SelectedSource],
    ) -> Box<Self> {
        let diagnostics = error.child_diagnostics().unwrap_or_default().to_vec();
        let warnings = error.warnings().to_vec();
        let mut failure = if let Some(diagnostic) = diagnostics.first() {
            Box::new(Self {
                notes: Vec::new(),
                message: error.to_string(),
                code: diagnostic.code().to_owned(),
                phase: error.classification().phase(),
                location: BuildFailureLocation::None,
                diagnostics: Vec::new(),
                warnings: Vec::new(),
                sources: Vec::new(),
            })
        } else {
            Self::classified(error)
        };
        failure.diagnostics = diagnostics;
        failure.warnings = warnings;
        failure.sources = sources.to_vec();
        failure
    }

    pub fn with_context(
        mut self: Box<Self>,
        warnings: &[StructuredDiagnosticV1],
        sources: &[SelectedSource],
    ) -> Box<Self> {
        self.warnings = warnings.to_vec();
        self.sources = sources.to_vec();
        self
    }
}

fn manifest_location(error: &ManifestRootError) -> BuildFailureLocation {
    BuildFailureLocation::Host {
        path: error.path().to_owned(),
        span: match error.kind() {
            ManifestRootErrorKind::Parse(error) => error.span().map(|span| span.range()),
            _ => None,
        },
    }
}

fn locator_location(error: &DependencyLocatorError) -> BuildFailureLocation {
    use DependencyLocatorError as Error;
    match error {
        Error::Manifest(error) => manifest_location(error),
        Error::Summary { path, source } => BuildFailureLocation::Artifact {
            path: path.clone(),
            member: match source {
                ArtifactManifestSummaryError::Envelope(error) => error.diagnostic().semantic_path(),
                ArtifactManifestSummaryError::Io(_) => "container:$".to_owned(),
                ArtifactManifestSummaryError::TestDirectory(_) => {
                    "manifest:test-directory".to_owned()
                }
            },
        },
        Error::InvalidArtifactSourceForm { path, .. } => BuildFailureLocation::Artifact {
            path: path.clone(),
            member: "manifest:cone.source_form".to_owned(),
        },
        Error::ReservedArtifact { path, .. } => BuildFailureLocation::Artifact {
            path: path.clone(),
            member: "manifest:cone.identity".to_owned(),
        },
        Error::Io { path, .. }
        | Error::SysrootArtifactNotFound { path, .. }
        | Error::CoordinateMismatch { path, .. }
        | Error::NonLibraryDependency { path, .. }
        | Error::SelfSourceLocator { root: path, .. } => BuildFailureLocation::Host {
            path: path.clone(),
            span: None,
        },
        Error::UndeclaredDependency(_)
        | Error::MissingLocatorProjection(_)
        | Error::ArtifactNotFound { .. }
        | Error::AmbiguousArtifact { .. } => BuildFailureLocation::None,
    }
}

impl fmt::Display for BuildFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}
impl std::error::Error for BuildFailure {}

pub type BuildResult<T> = Result<T, Box<BuildFailure>>;

fn tool_note(note: &scoop_source::DiagnosticNote) -> scoop_protocol::DiagnosticNoteV1 {
    scoop_protocol::DiagnosticNoteV1::new(
        &note.code,
        note.message.clone(),
        scoop_protocol::DiagnosticOriginV1::None,
    )
    .expect("tool note has valid text")
    .with_notes(note.notes.iter().map(tool_note).collect())
}
