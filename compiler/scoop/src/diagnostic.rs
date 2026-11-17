//! Stable phase and code classification for orchestration failures.

use std::fmt;

use scoop_protocol::StructuredDiagnosticV1;

use crate::{
    BuildGraphDiscoveryError, BuildGraphExecutionError, BuildGraphRequestError, CacheIoOperation,
    ChildTransportError, CompileCacheStoreError, DependencyLocatorError, LoadBuildRootError,
    OrdinarySourceExecutionError, PrebuiltCompletionError, PrepareBuildGraphError,
    ResolveBuildGraphError,
};

/// Canonical order in which build orchestration failures are reported.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum BuildFailurePhase {
    Request,
    Toolchain,
    Root,
    Locator,
    Summary,
    GraphIdentity,
    GraphVersionKind,
    GraphCycleOrder,
    SourceSnapshot,
    PrebuiltArtifact,
    Cache,
    ChildTransport,
    ChildDiagnostic,
    ChildOutput,
    CachePublish,
    RuntimeBuild,
    FinalLink,
    Publication,
    Run,
}

impl BuildFailurePhase {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Request => "Request",
            Self::Toolchain => "Toolchain",
            Self::Root => "Root",
            Self::Locator => "Locator",
            Self::Summary => "Summary",
            Self::GraphIdentity => "GraphIdentity",
            Self::GraphVersionKind => "GraphVersionKind",
            Self::GraphCycleOrder => "GraphCycleOrder",
            Self::SourceSnapshot => "SourceSnapshot",
            Self::PrebuiltArtifact => "PrebuiltArtifact",
            Self::Cache => "Cache",
            Self::ChildTransport => "ChildTransport",
            Self::ChildDiagnostic => "ChildDiagnostic",
            Self::ChildOutput => "ChildOutput",
            Self::CachePublish => "CachePublish",
            Self::RuntimeBuild => "RuntimeBuild",
            Self::FinalLink => "FinalLink",
            Self::Publication => "Publication",
            Self::Run => "Run",
        }
    }
}

impl fmt::Display for BuildFailurePhase {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

pub use scoop_source::DiagnosticCode;

/// Stable outer context for an orchestration failure.
///
/// `code` is absent only when the child supplied structured diagnostics, whose
/// original codes must be preserved.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BuildFailureClassification {
    phase: BuildFailurePhase,
    code: Option<DiagnosticCode>,
}

impl BuildFailureClassification {
    pub const fn new(phase: BuildFailurePhase, code: Option<DiagnosticCode>) -> Self {
        Self { phase, code }
    }

    pub const fn phase(self) -> BuildFailurePhase {
        self.phase
    }

    pub const fn code(self) -> Option<DiagnosticCode> {
        self.code
    }
}

/// Adds stable orchestrator context without flattening nested error details.
pub trait ClassifyBuildFailure {
    fn classification(&self) -> BuildFailureClassification;
}

const fn classified(phase: BuildFailurePhase, code: DiagnosticCode) -> BuildFailureClassification {
    BuildFailureClassification::new(phase, Some(code))
}

const fn phase_only(phase: BuildFailurePhase) -> BuildFailureClassification {
    BuildFailureClassification::new(phase, None)
}

impl ClassifyBuildFailure for BuildGraphRequestError {
    fn classification(&self) -> BuildFailureClassification {
        match self {
            Self::TestRequiresManifest => {
                classified(BuildFailurePhase::Request, DiagnosticCode::E2001)
            }
            Self::Toolchain(e) => classified(BuildFailurePhase::Toolchain, e.code()),
        }
    }
}

impl ClassifyBuildFailure for LoadBuildRootError {
    fn classification(&self) -> BuildFailureClassification {
        match self {
            Self::SingleFile(e) => classified(BuildFailurePhase::Root, e.code()),
        }
    }
}

impl ClassifyBuildFailure for BuildGraphDiscoveryError {
    fn classification(&self) -> BuildFailureClassification {
        match self {
            Self::PreparedGraph(_) => {
                classified(BuildFailurePhase::GraphIdentity, DiagnosticCode::ICE2201)
            }
            Self::Locator(source) => source.classification(),

            Self::Identity(_)
            | Self::IdentityCoordinateConflict { .. }
            | Self::ConflictingDependencyEdge { .. }
            | Self::InternalSourceClaim(_)
            | Self::InternalPrebuiltClaim(_) => {
                classified(BuildFailurePhase::GraphIdentity, DiagnosticCode::ICE2201)
            }
            Self::MissingLocatorProjection(_) | Self::MissingDependencySpan(_) => {
                classified(BuildFailurePhase::Locator, DiagnosticCode::ICE2201)
            }
            Self::ConflictingSourceLocator { .. } => {
                classified(BuildFailurePhase::Locator, DiagnosticCode::E2106)
            }
            Self::AmbiguousArtifact { .. } => {
                classified(BuildFailurePhase::Locator, DiagnosticCode::E2106)
            }
            Self::ConflictingNodeRepresentation { .. } => {
                classified(BuildFailurePhase::Locator, DiagnosticCode::E2106)
            }
            Self::ReservedNodeClaim(_) => {
                classified(BuildFailurePhase::GraphIdentity, DiagnosticCode::E2011)
            }
        }
    }
}

impl ClassifyBuildFailure for DependencyLocatorError {
    fn classification(&self) -> BuildFailureClassification {
        match self {
            Self::ArtifactNotFound { .. } | Self::SysrootArtifactNotFound { .. } => {
                classified(BuildFailurePhase::Locator, DiagnosticCode::E2101)
            }
            Self::InvalidArtifactSourceForm { .. } => {
                classified(BuildFailurePhase::Locator, DiagnosticCode::E2103)
            }
            Self::CoordinateMismatch { .. } => {
                classified(BuildFailurePhase::Locator, DiagnosticCode::E2102)
            }
            Self::AmbiguousArtifact { .. } => {
                classified(BuildFailurePhase::Locator, DiagnosticCode::E2106)
            }
            Self::ReservedArtifact { .. } => {
                classified(BuildFailurePhase::GraphIdentity, DiagnosticCode::E2011)
            }
            Self::NonLibraryDependency { .. } => {
                classified(BuildFailurePhase::GraphVersionKind, DiagnosticCode::E2103)
            }
            Self::Summary { source, .. } => classified(BuildFailurePhase::Summary, source.code()),
            Self::UndeclaredDependency(_) | Self::MissingLocatorProjection(_) => {
                classified(BuildFailurePhase::Locator, DiagnosticCode::ICE2201)
            }
            Self::Manifest(e) => classified(BuildFailurePhase::Locator, e.code()),
            Self::Io { .. } => classified(BuildFailurePhase::Locator, DiagnosticCode::E2201),
            Self::SelfSourceLocator { .. } => {
                classified(BuildFailurePhase::Locator, DiagnosticCode::E2105)
            }
        }
    }
}

impl ClassifyBuildFailure for ResolveBuildGraphError {
    fn classification(&self) -> BuildFailureClassification {
        match self {
            Self::Allocation { .. } => {
                classified(BuildFailurePhase::GraphCycleOrder, DiagnosticCode::E2205)
            }
            Self::InvalidReservedNode { .. } => {
                classified(BuildFailurePhase::GraphIdentity, DiagnosticCode::E2011)
            }
            Self::NonLibraryDependency { .. } => {
                classified(BuildFailurePhase::GraphVersionKind, DiagnosticCode::E2103)
            }
            Self::MultipleVersions(_) => {
                classified(BuildFailurePhase::GraphVersionKind, DiagnosticCode::E2104)
            }
            Self::MissingTrustedCore | Self::MissingDirectCore { .. } => {
                classified(BuildFailurePhase::GraphVersionKind, DiagnosticCode::E2101)
            }
            Self::UnreachableNodes(_) => {
                classified(BuildFailurePhase::GraphCycleOrder, DiagnosticCode::ICE2201)
            }
            Self::Cycles(_) => {
                classified(BuildFailurePhase::GraphCycleOrder, DiagnosticCode::E2105)
            }
            Self::Identity(_)
            | Self::MissingRoot(_)
            | Self::NodeIdentityMismatch { .. }
            | Self::InvalidRootRepresentation { .. }
            | Self::EdgeKeyMismatch(_)
            | Self::MissingEdgeEndpoint { .. }
            | Self::EdgeCoordinateMismatch { .. }
            | Self::DependencyRecordMismatch { .. }
            | Self::InvalidEdgeOrigin { .. } => {
                classified(BuildFailurePhase::GraphIdentity, DiagnosticCode::ICE2201)
            }
            Self::InternalCyclePath(_)
            | Self::InternalCycleEdge { .. }
            | Self::InternalTopologicalState(_)
            | Self::InternalTopologicalLength { .. } => {
                classified(BuildFailurePhase::GraphCycleOrder, DiagnosticCode::ICE2201)
            }
        }
    }
}

impl ClassifyBuildFailure for PrepareBuildGraphError {
    fn classification(&self) -> BuildFailureClassification {
        match self {
            Self::SourceDiscovery(e) => classified(BuildFailurePhase::SourceSnapshot, e.code()),
            Self::Native(e) => classified(BuildFailurePhase::SourceSnapshot, e.code()),
            Self::SingleFile(e) => classified(BuildFailurePhase::SourceSnapshot, e.code()),
            Self::Manifest(e) => classified(BuildFailurePhase::SourceSnapshot, e.code()),
        }
    }
}

impl ClassifyBuildFailure for BuildGraphExecutionError {
    fn classification(&self) -> BuildFailureClassification {
        match self {
            Self::Swift { source, .. } => classified(BuildFailurePhase::Toolchain, source.code()),
            Self::MissingPreparedNode(_) | Self::RequestIdOverflow => {
                classified(BuildFailurePhase::ChildTransport, DiagnosticCode::ICE2201)
            }
            Self::CacheKey(_, _) => classified(BuildFailurePhase::Cache, DiagnosticCode::ICE2201),
            Self::Prebuilt(_, source) => source.classification(),
            Self::Ordinary(_, source) => source.classification(),
        }
    }
}

impl BuildGraphExecutionError {
    /// Returns child-provided diagnostics without replacing their original codes.
    pub fn child_diagnostics(&self) -> Option<&[StructuredDiagnosticV1]> {
        match self {
            Self::Ordinary(_, source) => match source.as_ref() {
                OrdinarySourceExecutionError::ChildFailure(diagnostics) => Some(diagnostics),
                _ => None,
            },
            _ => None,
        }
    }
}

impl ClassifyBuildFailure for OrdinarySourceExecutionError {
    fn classification(&self) -> BuildFailureClassification {
        match self {
            Self::ProducedOutput { source, .. } => source.classification(),
            Self::Observation(error) if error.kind() != std::io::ErrorKind::InvalidData => {
                classified(BuildFailurePhase::ChildOutput, DiagnosticCode::E2201)
            }
            Self::Observation(_) | Self::UnexpectedResponse => {
                classified(BuildFailurePhase::ChildOutput, DiagnosticCode::E2502)
            }
            Self::CacheStore(source) => source.classification(),

            Self::ChildTransport(source) => source.classification(),
            Self::ChildFailure(_) => phase_only(BuildFailurePhase::ChildDiagnostic),
            Self::Completion(_) => {
                classified(BuildFailurePhase::ChildOutput, DiagnosticCode::ICE2201)
            }
            Self::NotOrdinarySource(_) | Self::RequestPlan(_) => {
                classified(BuildFailurePhase::ChildTransport, DiagnosticCode::ICE2201)
            }
            Self::CacheKey(_) | Self::Generators(_) => {
                classified(BuildFailurePhase::Cache, DiagnosticCode::ICE2201)
            }
        }
    }
}

impl ClassifyBuildFailure for PrebuiltCompletionError {
    fn classification(&self) -> BuildFailureClassification {
        match self {
            Self::StalePrebuiltDependency { .. } => {
                classified(BuildFailurePhase::PrebuiltArtifact, DiagnosticCode::E2108)
            }
            Self::CandidateDisagreement { .. } => {
                classified(BuildFailurePhase::PrebuiltArtifact, DiagnosticCode::E2107)
            }
            Self::NotPrebuilt(_)
            | Self::EmptyCandidateSet(_)
            | Self::DuplicateCompletedNode(_)
            | Self::CurrentNodeAlreadyCompleted(_)
            | Self::CandidatePlan { .. } => {
                classified(BuildFailurePhase::PrebuiltArtifact, DiagnosticCode::ICE2201)
            }
        }
    }
}

impl ClassifyBuildFailure for CompileCacheStoreError {
    fn classification(&self) -> BuildFailureClassification {
        match self {
            Self::Io {
                operation: CacheIoOperation::Lock,
                ..
            } => classified(BuildFailurePhase::Cache, DiagnosticCode::E2201),
            Self::WrongLock => classified(BuildFailurePhase::Cache, DiagnosticCode::ICE2201),
            Self::Io {
                operation: CacheIoOperation::Publish | CacheIoOperation::Write,
                ..
            } => classified(BuildFailurePhase::CachePublish, DiagnosticCode::E2201),
            Self::Encode(_) => classified(BuildFailurePhase::CachePublish, DiagnosticCode::ICE2501),
            Self::Decode(_) => classified(BuildFailurePhase::Cache, DiagnosticCode::ICE2501),
            Self::Io { .. } => classified(BuildFailurePhase::Cache, DiagnosticCode::E2201),
        }
    }
}

impl ClassifyBuildFailure for ChildTransportError {
    fn classification(&self) -> BuildFailureClassification {
        match self {
            Self::Response { .. } | Self::RequestIdMismatch { .. } => {
                classified(BuildFailurePhase::ChildTransport, DiagnosticCode::E2502)
            }
            Self::Signal | Self::ExitMismatch { .. } => {
                classified(BuildFailurePhase::ChildTransport, DiagnosticCode::E2203)
            }
            Self::Request(_) => {
                classified(BuildFailurePhase::ChildTransport, DiagnosticCode::ICE2501)
            }
            Self::MissingExecutableParent { .. } => {
                classified(BuildFailurePhase::ChildTransport, DiagnosticCode::E2302)
            }
            Self::TemporaryDirectory(_) | Self::WriteLog(_) => {
                classified(BuildFailurePhase::ChildTransport, DiagnosticCode::E2201)
            }
            Self::Spawn { .. } | Self::WorkerSpawn(_) => {
                classified(BuildFailurePhase::ChildTransport, DiagnosticCode::E2202)
            }
            Self::MissingPipe(_) | Self::ReaderPanicked(_) | Self::WorkerPanicked => {
                classified(BuildFailurePhase::ChildTransport, DiagnosticCode::ICE2201)
            }
            Self::WriteRequest(_) | Self::Wait(_) | Self::ReadStream { .. } => {
                classified(BuildFailurePhase::ChildTransport, DiagnosticCode::E2204)
            }
        }
    }
}

#[cfg(test)]
mod tests;
