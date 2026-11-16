//! Driver task selection over already-bound ordinary annotation metadata.

use scoop_ast::{Diagnostic, DiagnosticNote, NonEmptyVec, Span};
use scoop_generator_protocol::{MethodId, QueryJson, Reference};
use scoop_hir as hir;
use scoop_identity::{BindableEntity, BindingNamespace, CanonicalIdentifier, ConeIdentity};
use std::collections::HashMap;

mod configuration;
mod external;
pub(crate) use external::ExternalGenerator;
mod plan;
mod request;

#[derive(Clone, Debug)]
pub struct GeneratorBinding {
    pub interface: hir::TypeId,
    pub origin: hir::SemanticSourceLocation,
}

#[derive(Clone, Debug)]
pub struct GeneratorMethod {
    pub method: MethodId,
    pub pending: hir::PendingMethod,
    pub origin: hir::SemanticSourceLocation,
}

#[derive(Clone, Debug)]
pub struct GeneratorTask {
    pub owner: hir::NominalOwner,
    pub bindings: NonEmptyVec<GeneratorBinding>,
    pub methods: NonEmptyVec<GeneratorMethod>,
}

#[derive(Clone, Debug)]
pub struct GeneratorBatch {
    pub generator: String,
    pub tasks: Vec<GeneratorTask>,
}

#[derive(Clone)]
struct GenerationAnnotation {
    owner: hir::NominalOwner,
    application: hir::SourceAnnotationApplication,
}

/// Selects each missing implementation once while preserving all chosen bindings.
/// The returned local ids belong to this borrowed frontend query view.
pub fn plan_generators(
    queries: &mut dyn hir::SemanticQueries,
) -> Result<Vec<GeneratorBatch>, Vec<Diagnostic>> {
    let annotations = configuration::annotations(queries).map_err(|error| vec![error])?;
    plan::plan(queries, annotations)
}

fn diagnostic(
    code: scoop_source::DiagnosticCode,
    origin: &hir::SemanticSourceLocation,
    message: impl Into<String>,
) -> Diagnostic {
    let mut diagnostic = Diagnostic::at(code, origin.span, message);
    diagnostic.set_source(origin.source.clone());
    diagnostic
}

fn related(origin: &hir::SemanticSourceLocation, message: impl Into<String>) -> DiagnosticNote {
    DiagnosticNote::at_source(
        scoop_source::DiagnosticCode::N0003,
        origin.source.clone(),
        origin.span,
        message,
    )
}

#[cfg(test)]
mod tests;
