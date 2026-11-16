use scoop_ast as ast;
use scoop_hir as hir;

use crate::call_resolution::arguments::CandidateArgumentMap;
use crate::call_resolution::candidates::{NominalConstructorSource, NominalConstructorView};
use crate::call_resolution::diagnostics::nominal_source_signature;
use crate::call_resolution::specificity::ApplicableDeclaration;
use crate::expr::ResolvedCallTypeArgument;
use crate::expr::{NominalArgumentInput, NominalArguments};
use crate::{Lowerer, TypeId};

mod aliases;
mod enclosing;
mod named;
mod safety;
mod test_selection;

pub(crate) use named::NamedNominalProbe;

pub(crate) struct ResolvedNominalConstructor {
    pub(crate) source: NominalConstructorSource,
    pub(crate) type_args: Vec<TypeId>,
    pub(crate) args: Vec<hir::Expr>,
}

pub(crate) struct NominalConstructorCall<'a> {
    pub(crate) enclosing: Option<&'a hir::Expr>,
    pub(crate) explicit_type_args: &'a [ResolvedCallTypeArgument],
    pub(crate) expected_type_args: Option<&'a [TypeId]>,
    pub(crate) arguments: &'a [ast::CallArgument],
    pub(crate) span: ast::Span,
}

struct ApplicableConstructor {
    enclosing: Option<hir::Expr>,
    state: Box<Lowerer>,
    source: NominalConstructorSource,
    view: NominalConstructorView,
    argument_map: CandidateArgumentMap,
    inferred: NominalArguments,
    parameter_types: Vec<TypeId>,
}

impl Lowerer {
    pub(crate) fn resolve_nominal_constructor_overload(
        &mut self,
        name: &str,
        candidates: &[NominalConstructorSource],
        call: NominalConstructorCall<'_>,
        sink: &mut Vec<hir::Statement>,
    ) -> Option<ResolvedNominalConstructor> {
        let span = call.span;
        let winner = self.select_nominal_constructor(name, candidates, call)?;
        self.commit_nominal_candidate(winner, span, sink)
    }

    fn select_nominal_constructor(
        &mut self,
        name: &str,
        candidates: &[NominalConstructorSource],
        call: NominalConstructorCall<'_>,
    ) -> Option<ApplicableConstructor> {
        let NominalConstructorCall {
            enclosing,
            explicit_type_args,
            expected_type_args,
            arguments,
            span,
        } = call;
        if candidates.is_empty() {
            self.error(
                scoop_source::DiagnosticCode::E0510,
                span,
                format!("type `{name}` has no source constructor"),
            );
            return None;
        }
        let mut applicable = Vec::new();
        let mut failures = Vec::new();
        let mut inaccessible = false;
        for &source in candidates {
            if !self.constructor_is_accessible(source) {
                inaccessible = true;
                continue;
            }
            let view = self.nominal_constructor_view(source, span);
            match self.probe_named_nominal(
                view.clone(),
                NominalConstructorCall {
                    enclosing,
                    explicit_type_args,
                    expected_type_args,
                    arguments,
                    span,
                },
            ) {
                Ok(probe) => applicable.push(probe.candidate),
                Err(failure) => {
                    if self.retain_internal_diagnostics(&failure) {
                        return None;
                    }
                    failures.push((view, failure));
                }
            }
        }

        let winner = match applicable.len() {
            0 => {
                if failures.is_empty() && inaccessible {
                    self.error(
                        scoop_source::DiagnosticCode::E0403,
                        span,
                        format!("constructor of type `{name}` is not accessible here"),
                    );
                    return None;
                }
                if failures.len() == 1 {
                    self.commit_layer_diagnostics(&failures.pop().expect("one failure").1);
                } else {
                    let mut diagnostic = ast::Diagnostic::at_file(
                        scoop_source::DiagnosticCode::E0711,
                        self.current_text_file(),
                        span,
                        format!(
                            "no applicable candidate for constructor `{name}` in nominal constructor candidate layer"
                        ),
                    );
                    for (view, state) in &failures {
                        let mut candidate = scoop_ast::DiagnosticNote::without_location(
                            scoop_source::DiagnosticCode::N0002,
                            nominal_source_signature(self, view),
                        );
                        candidate.notes = state.diagnostics[self.diagnostics.len()..]
                            .iter()
                            .cloned()
                            .map(|d| d.into_note())
                            .collect();
                        diagnostic.notes.push(candidate);
                    }
                    self.diagnostics.push(diagnostic);
                }
                return None;
            }
            1 => 0,
            _ => self.most_specific_nominal_constructor(name, &applicable, arguments, span)?,
        };
        Some(applicable.swap_remove(winner))
    }

    fn most_specific_nominal_constructor(
        &mut self,
        name: &str,
        applicable: &[ApplicableConstructor],
        arguments: &[ast::CallArgument],
        span: ast::Span,
    ) -> Option<usize> {
        let declarations = applicable
            .iter()
            .map(|candidate| ApplicableDeclaration {
                declaration: candidate.view.forwarding(&candidate.parameter_types),
                parameterized: !candidate.view.signature.owner_parameters.is_empty(),
                defaults: candidate.argument_map.explicit_default_count(),
                vararg: candidate
                    .view
                    .signature
                    .value_parameters
                    .iter()
                    .any(|parameter| parameter.is_vararg()),
            })
            .collect::<Vec<_>>();
        let pool = match self.most_specific_declarations(&declarations) {
            Ok(pool) => pool,
            Err(error) => {
                self.diagnostics.push(scoop_ast::Diagnostic::internal(
                    scoop_source::DiagnosticCode::ICE0601,
                    error,
                ));
                return None;
            }
        };
        let effects = applicable
            .iter()
            .map(|candidate| {
                use crate::call_resolution::specificity::{
                    LambdaEffectDeclaration, LambdaEffectReceiver,
                };
                let (safety, gc_effect) = match candidate.source {
                    NominalConstructorSource::Struct(id) => (
                        self.struct_constructors[id].safety,
                        self.struct_constructors[id].source_gc_effect(),
                    ),
                    NominalConstructorSource::Class(id) => {
                        (self.class_constructors[id].safety, hir::GcEffect::Managed)
                    }
                    _ => (hir::Safety::Safe, hir::GcEffect::Managed),
                };
                Some(LambdaEffectDeclaration::new(
                    &candidate.view.signature,
                    LambdaEffectReceiver::None,
                    hir::RequiredCallEffects {
                        is_suspend: false,
                        safety,
                        gc_effect,
                    },
                    &candidate.argument_map,
                    arguments,
                ))
            })
            .collect::<Vec<_>>();
        let pool = self.prefer_ordinary_lambdas(&pool, &effects);
        if let [winner] = pool.as_slice() {
            return Some(*winner);
        }
        let mut diagnostic = ast::Diagnostic::at_file(
            scoop_source::DiagnosticCode::E0712,
            self.current_text_file(),
            span,
            format!("ambiguous constructor call `{name}`"),
        );
        diagnostic.notes = pool
            .iter()
            .map(|&candidate| {
                scoop_ast::DiagnosticNote::without_location(
                    scoop_source::DiagnosticCode::N0002,
                    nominal_source_signature(self, &applicable[candidate].view),
                )
                .with_note(scoop_ast::DiagnosticNote::without_location(
                    scoop_source::DiagnosticCode::N0703,
                    "candidate remains tied after overload ranking",
                ))
            })
            .collect();
        self.diagnostics.push(diagnostic);
        None
    }
}
