//! Check one root through complete HIR, building only its source dependencies.

use scoop_identity::{ConeCoordinate, ConeIdentity};
use scoop_protocol::{CompilationPurpose, StructuredDiagnosticV1, TargetSelectionRequestV1};

use crate::{BuildFailure, BuildFailurePhase, BuildResult, ResolvedBuildGraph, SelectedSource};

mod compile;
mod plan;

#[derive(Debug)]
pub struct CheckRequest {
    pub target: TargetSelectionRequestV1,
    pub purpose: CompilationPurpose,
}

#[derive(Debug)]
pub struct CheckOutcome {
    pub root: ConeCoordinate,
    pub target: String,
    pub purpose: CompilationPurpose,
    pub warnings: Vec<StructuredDiagnosticV1>,
    pub sources: Vec<SelectedSource>,
    pub compiled_dependencies: Vec<ConeIdentity>,
}

pub fn check(graph: ResolvedBuildGraph, request: CheckRequest) -> BuildResult<CheckOutcome> {
    let plan = plan::Plan::new(graph, &request)?;
    let (warnings, compiled_dependencies) = compile::run(plan.graph, &plan.sources)?;
    Ok(CheckOutcome {
        root: plan.root,
        target: request.target.canonical_triple().into(),
        purpose: request.purpose,
        warnings,
        sources: plan.sources,
        compiled_dependencies,
    })
}

fn error(message: impl std::fmt::Display) -> Box<BuildFailure> {
    BuildFailure::tool("SCOOP_CHECK", BuildFailurePhase::ChildDiagnostic, message)
}

fn io_error(path: &std::path::Path, message: impl std::fmt::Display) -> Box<BuildFailure> {
    error(message).at_host(path)
}
