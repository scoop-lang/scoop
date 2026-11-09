use super::*;
use crate::{ProductionSingleConeCompilerRunner, ResolvedBuildGraph, SingleConeCompilerRunner};
use scoop_protocol::{RequestCorrelationId, ScoopcProductV1, ScoopcResponseEnvelopeV1};

pub(super) fn run(
    graph: ResolvedBuildGraph,
    sources: &[SelectedSource],
) -> BuildResult<(Vec<StructuredDiagnosticV1>, Vec<ConeIdentity>)> {
    let root = graph.root_identity();
    let mut prepared = graph
        .prepare_frontend()
        .map_err(BuildFailure::preparation)?;
    for id in prepared.dependency_first().to_vec() {
        if id != root && prepared.source_input(id).is_some() {
            prepared
                .prepare_machine_source(id)
                .map_err(BuildFailure::preparation)?;
        }
    }
    let runner = ProductionSingleConeCompilerRunner;
    let (completed, observations) = prepared
        .execute_dependencies(&runner)
        .map_err(|cause| BuildFailure::execution(cause, sources))?;
    let mut warnings = completed
        .iter()
        .flat_map(|node| node.warnings().iter().cloned())
        .collect::<Vec<_>>();
    let result = (|| {
        let inputs = completed.iter().collect::<Vec<_>>();
        let invocation = prepared
            .child_check_plan(
                RequestCorrelationId::from_array((prepared.node_count() as u128 + 1).to_be_bytes()),
                &inputs,
            )
            .map_err(error)?;
        match runner
            .invoke(prepared.compiler(), invocation.request(), invocation.io())
            .map_err(error)?
        {
            ScoopcResponseEnvelopeV1::Success { result, .. }
                if matches!(result.product(), ScoopcProductV1::Check) =>
            {
                Ok(result)
            }
            ScoopcResponseEnvelopeV1::Failure { diagnostics, .. } => {
                let mut failure = error("root check failed");
                failure.diagnostics = diagnostics;
                Err(failure)
            }
            _ => Err(error("compiler returned an unexpected check result")),
        }
    })()
    .map_err(|failure| failure.with_context(&warnings, sources))?;
    warnings.extend_from_slice(result.warnings());
    Ok((warnings, observations.child_invocations().to_vec()))
}
