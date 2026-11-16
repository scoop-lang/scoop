use super::*;
use crate::call_resolution::diagnostics::{callable_layer_name, callable_source_signature};

impl ReferenceDeclaration {
    fn signature(&self, state: &Lowerer, name: &str) -> String {
        match self {
            Self::Constructor(declaration) => declaration.signature(state, name),
            Self::Local(declaration) => callable_source_signature(state, name, &declaration.view),
            Self::Imported(declaration) => declaration.signature(state, name),
        }
    }

    fn layer(&self, state: &Lowerer) -> &'static str {
        match self {
            Self::Constructor(_) => "constructor candidate",
            Self::Local(declaration) => {
                callable_layer_name(state, std::slice::from_ref(&declaration.view))
            }
            Self::Imported(declaration) => declaration.layer(),
        }
    }
}

fn layer_name(layers: impl IntoIterator<Item = &'static str>) -> &'static str {
    let mut layers = layers.into_iter();
    let first = layers
        .next()
        .expect("a failed or ambiguous reference has candidates");
    if layers.all(|layer| layer == first) {
        first
    } else {
        "declaration candidate"
    }
}

impl Lowerer {
    pub(super) fn reference_failures_diagnostic(
        &mut self,
        context: ReferenceResolutionContext<'_>,
        failures: &[ReferenceFailure],
    ) {
        if let [failure] = failures
            && self.report_intrinsic_reference_failure(context.span, &failure.reason)
        {
            return;
        }
        if failures.is_empty() {
            self.error(
                scoop_source::DiagnosticCode::E0713,
                context.span,
                format!(
                    "no declaration candidate has a permitted form for {}",
                    context.display
                ),
            );
            return;
        }
        let layer = layer_name(failures.iter().map(|failure| failure.layer));
        let mut diagnostic = scoop_ast::Diagnostic::at(
            scoop_source::DiagnosticCode::E0711,
            context.span,
            format!(
                "no applicable candidate for {} in {layer} layer",
                context.display
            ),
        );
        diagnostic.set_file(self.current_text_file());
        diagnostic.notes = failures
            .iter()
            .map(|failure| {
                scoop_source::DiagnosticNote::without_location(
                    scoop_source::DiagnosticCode::N0002,
                    failure.signature.clone(),
                )
                .with_note(failure.reason.note())
            })
            .collect();
        self.diagnostics.push(diagnostic);
    }

    pub(super) fn reference_ambiguity_diagnostic(
        &mut self,
        context: ReferenceResolutionContext<'_>,
        applicable: &[&ApplicableReference],
    ) {
        let layer = layer_name(
            applicable
                .iter()
                .map(|candidate| candidate.declaration.layer(&candidate.state)),
        );
        let reason = if context.expected.is_some() {
            "matches the expected function type"
        } else {
            "forms a complete non-generic function type"
        };
        let mut diagnostic = scoop_ast::Diagnostic::at(
            scoop_source::DiagnosticCode::E0712,
            context.span,
            format!("{} is ambiguous in {layer} layer", context.display),
        );
        diagnostic.set_file(self.current_text_file());
        diagnostic.notes = applicable
            .iter()
            .map(|candidate| {
                scoop_source::DiagnosticNote::without_location(
                    scoop_source::DiagnosticCode::N0002,
                    candidate
                        .declaration
                        .signature(&candidate.state, context.name),
                )
                .with_note(scoop_source::DiagnosticNote::without_location(
                    scoop_source::DiagnosticCode::N0701,
                    reason,
                ))
                .with_note(scoop_source::DiagnosticNote::without_location(
                    scoop_source::DiagnosticCode::N0703,
                    "candidate remains tied after overload ranking",
                ))
            })
            .collect();
        self.diagnostics.push(diagnostic);
    }
}
