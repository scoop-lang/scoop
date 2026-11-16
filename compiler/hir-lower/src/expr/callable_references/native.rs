//! Exact native callback references use the ordinary declaration layers.

use super::*;
use crate::call_resolution::constraints::CallableCategory;
use crate::imports::lookup::calls::{NamedCallBinding, NamedCallOrigin, NamedCallTarget};

mod declarations;
use declarations::{NativeReferenceDeclaration, NativeReferenceFailure};

struct ApplicableNativeReference {
    state: Box<Lowerer>,
    declaration: NativeReferenceDeclaration,
}

impl Lowerer {
    pub(super) fn lower_native_function_reference(
        &mut self,
        name: &ast::Ident,
        span: Span,
        expected: TypeId,
    ) -> Option<hir::Expr> {
        if self.scopes.lookup(&name.text).is_some()
            || !self.local_function_scopes.lookup(&name.text).is_empty()
        {
            self.error(
                scoop_source::DiagnosticCode::E1217,
                span,
                "a native `FunPtr` address cannot target a local function or function value"
                    .to_string(),
            );
            return None;
        }
        let mut first_failure = None;
        for layer in self.named_callable_reference_layers(&name.text) {
            if self.report_unavailable_lookup(name, &layer.unavailable) {
                return None;
            }
            let mut matching = Vec::new();
            let mut failures = Vec::new();
            let mut suppressed = !layer.suppressed_callables.is_empty();
            for candidate in &layer.candidates {
                if !matches!(
                    candidate.target,
                    NamedCallTarget::Function(_)
                        | NamedCallTarget::ImportedDependency(
                            hir::ImportedTarget::Function(_)
                                | hir::ImportedTarget::GenericFunction(_)
                        )
                ) {
                    continue;
                }

                if let NamedCallTarget::Function(function) = candidate.target
                    && self.declaration_surface.rejects_function(function)
                {
                    suppressed = true;
                    continue;
                }
                let mut state = self.clone();
                let declaration =
                    match NativeReferenceDeclaration::resolve(&mut state, candidate, &name.text) {
                        Ok(declaration) => declaration,
                        Err(failure) => {
                            if self.retain_internal_diagnostics(&state) {
                                return None;
                            }
                            failures.push(failure);
                            continue;
                        }
                    };
                match state.check_concrete_callable_signature(
                    CallableCategory::Native,
                    declaration.effects.is_suspend,
                    &declaration.parameters,
                    declaration.return_type,
                    expected,
                ) {
                    Ok(()) => matching.push(ApplicableNativeReference {
                        state: Box::new(state),
                        declaration,
                    }),
                    Err(crate::call_resolution::constraints::InferenceFailure::Internal(error)) => {
                        self.diagnostics.push(scoop_ast::Diagnostic::internal(
                            scoop_source::DiagnosticCode::ICE0601,
                            error,
                        ));
                        return None;
                    }
                    Err(crate::call_resolution::constraints::InferenceFailure::Constraint(
                        constraint,
                    )) => failures.push(declaration.failure(&state, &constraint)),
                }
            }
            if matching.len() == 1 {
                let selected = matching
                    .pop()
                    .expect("one native reference candidate exists");
                *self = *selected.state;
                if selected.declaration.effects.attributes.safety == hir::Safety::Unsafe {
                    self.require_unsafe_operation(span, "taking the address of an unsafe callback");
                }
                let semantic_target = selected.declaration.semantic_target();
                let target = match selected.declaration.commit(self) {
                    Ok(target) => target,
                    Err(message) => {
                        self.error(scoop_source::DiagnosticCode::ICE0501, span, message);
                        return None;
                    }
                };
                if let Some(semantic_target) = semantic_target {
                    self.observe_callable_reference(span, semantic_target);
                }
                self.observe_import_lookup(name, layer.kind);
                return Some(hir::Expr {
                    kind: ExprKind::FunctionAddress(target),
                    ty: expected,
                    span,
                    origin: self.expression_origin(span),
                });
            }
            if matching.len() > 1 {
                let layer = matching[0].declaration.layer;
                let mut diagnostic = ast::Diagnostic::at_file(
                    scoop_source::DiagnosticCode::E0712,
                    self.current_text_file(),
                    span,
                    format!(
                        "native function reference `::{}` is ambiguous in {layer} layer",
                        name.text
                    ),
                );
                diagnostic
                    .notes
                    .push(scoop_ast::DiagnosticNote::without_location(
                        scoop_source::DiagnosticCode::N0701,
                        format!(
                            "expected native function pointer type: {}",
                            self.type_name(expected)
                        ),
                    ));
                diagnostic.notes.extend(matching.iter().map(|candidate| {
                    scoop_ast::DiagnosticNote::without_location(
                        scoop_source::DiagnosticCode::N0002,
                        candidate.declaration.display.clone(),
                    )
                    .with_note(scoop_ast::DiagnosticNote::without_location(
                        scoop_source::DiagnosticCode::N0703,
                        "exactly matches the expected FunPtr signature",
                    ))
                }));
                self.diagnostics.push(diagnostic);
                return None;
            }
            if suppressed {
                if !failures.is_empty() {
                    self.native_reference_failures(name, &failures, span);
                }
                return None;
            }
            if !failures.is_empty() {
                first_failure.get_or_insert(failures);
            }
        }
        self.native_reference_failures(name, first_failure.as_deref().unwrap_or_default(), span);
        None
    }

    fn native_reference_failures(
        &mut self,
        name: &ast::Ident,
        failures: &[NativeReferenceFailure],
        span: Span,
    ) {
        if let Some(diagnostic) = failures
            .iter()
            .find_map(|failure| failure.reason.internal_diagnostic())
        {
            self.diagnostics.push(diagnostic);
            return;
        }
        if let [failure] = failures
            && self.report_intrinsic_reference_failure(span, &failure.reason)
        {
            return;
        }
        let Some(first) = failures.first() else {
            self.error(scoop_source::DiagnosticCode::E1217, span, format!("no eligible `@NoGC` top-level function `::{}` exactly matches the expected FunPtr signature", name.text));
            return;
        };
        let layer = first.layer;
        let mut diagnostic = scoop_ast::Diagnostic::at(
            scoop_source::DiagnosticCode::E0711,
            span,
            format!(
                "no applicable candidate for native function reference `::{}` in {layer} layer",
                name.text
            ),
        );
        diagnostic.set_file(self.current_text_file());
        diagnostic.notes = failures
            .iter()
            .map(|failure| {
                scoop_source::DiagnosticNote::without_location(
                    scoop_source::DiagnosticCode::N0002,
                    failure.display.clone(),
                )
                .with_note(failure.reason.note())
            })
            .collect();
        self.diagnostics.push(diagnostic);
    }
}
