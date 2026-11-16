use super::*;

use crate::call_resolution::constraints::{
    ConstraintFailure, ConstraintOrigin, InferenceVariableId,
};
use crate::call_resolution::diagnostics::{
    callable_layer_name, callable_source_signature, render_callable_constraint_failure,
};
use crate::expr::ResolvedCallTypeArgument;
use crate::overload::probe::{
    ApplicableCandidate, CandidateProbeFailure, CandidateProbeFailureKind, CandidateShapeFailure,
};

mod integers;
use integers::{primitive_integer_conversion_suggestion, render_literal_exact_commits};

pub(super) struct CandidateFailureContext<'call, 'arguments> {
    pub(super) arguments: &'call OverloadArguments<'arguments>,
    pub(super) extension_receiver: Option<&'call hir::Expr>,
    pub(super) explicit_type_args: &'call [ResolvedCallTypeArgument],
    pub(super) span: Span,
}

pub(super) struct AmbiguityContext<'call, 'arguments> {
    pub(super) applicable: &'call [ApplicableCandidate],
    pub(super) arguments: &'call OverloadArguments<'arguments>,
    pub(super) receiver_offset: usize,
    pub(super) span: Span,
}

impl Lowerer {
    pub(super) fn candidate_failures_diagnostic(
        &mut self,
        name: &str,
        prepared: &[Candidate],
        failures: &mut [CandidateProbeFailure],
        context: CandidateFailureContext<'_, '_>,
    ) -> bool {
        let CandidateFailureContext {
            arguments,
            extension_receiver,
            explicit_type_args,
            span,
        } = context;
        debug_assert_eq!(failures.len(), prepared.len());
        for (candidate, failure) in prepared.iter().zip(failures.iter()) {
            if let CandidateProbeFailureKind::Constraint(constraint) = &failure.kind
                && constraint.code().category() == Some(scoop_source::DiagnosticCategory::Internal)
            {
                self.error(
                    constraint.code(),
                    span,
                    render_candidate_failure(self, candidate, failure, arguments),
                );
                return true;
            }
        }

        if let [failure] = failures
            && let CandidateProbeFailureKind::Context { code, span, reason } = &failure.kind
        {
            // A single context-forbidden declaration needs the direct effect
            // diagnostic; overload sets still retain each candidate's reason.
            self.error(code, *span, reason.clone());
            return true;
        }
        if failures.iter().any(|failure| {
            matches!(failure.kind, CandidateProbeFailureKind::Expression { .. })
                && failure.state(self).diagnostics.len() == self.diagnostics.len()
        }) {
            // An argument depends on an already rejected declaration. Keep
            // that original error instead of inventing an overload failure.
            return false;
        }
        failures.sort_by_key(|failure| failure.candidate);
        if failures
            .iter()
            .all(|failure| matches!(failure.kind, CandidateProbeFailureKind::Expression { .. }))
            && failures
                .windows(2)
                .all(|pair| match (&pair[0].kind, &pair[1].kind) {
                    (
                        CandidateProbeFailureKind::Expression {
                            span: left_span,
                            reason: left_reason,
                            ..
                        },
                        CandidateProbeFailureKind::Expression {
                            span: right_span,
                            reason: right_reason,
                            ..
                        },
                    ) => left_span == right_span && left_reason == right_reason,
                    _ => false,
                })
        {
            let baseline = self.diagnostics.len();
            let diagnostics = failures[0].state(self).diagnostics[baseline..].to_vec();
            let same_diagnostics = !diagnostics.is_empty()
                && failures[1..]
                    .iter()
                    .all(|failure| failure.state(self).diagnostics[baseline..] == diagnostics);
            let independent_of_context = failures.iter().all(|failure| {
                matches!(
                    failure.kind,
                    CandidateProbeFailureKind::Expression {
                        context_dependent: false,
                        ..
                    }
                )
            });
            if same_diagnostics && independent_of_context {
                self.diagnostics.extend(diagnostics);
                return true;
            }
        }
        let diagnostic_span = common_failure_span(
            failures,
            arguments,
            extension_receiver,
            explicit_type_args,
            span,
        );
        let views = prepared
            .iter()
            .map(|candidate| candidate.view.clone())
            .collect::<Vec<_>>();
        let layer = callable_layer_name(self, &views);
        let mut diagnostic = ast::Diagnostic::at_file(
            scoop_source::DiagnosticCode::E0711,
            self.current_text_file(),
            diagnostic_span,
            format!("no applicable candidate for `{name}` in {layer} layer"),
        );
        for failure in failures {
            let candidate = &prepared[failure.candidate];
            let signature = callable_source_signature(self, name, &candidate.view);
            let mut note = candidate_note(self, candidate, signature);
            let state = failure.state(self);
            match &failure.kind {
                CandidateProbeFailureKind::Shape(shape) => {
                    let code = match shape {
                        CandidateShapeFailure::TypeArgumentArity { .. } => {
                            scoop_source::DiagnosticCode::E0601
                        }
                        CandidateShapeFailure::Argument(error) => error.code(),
                    };
                    let mut cause = ast::DiagnosticNote::at(
                        code,
                        self.current_text_file(),
                        diagnostic_span,
                        render_candidate_failure(self, candidate, failure, arguments),
                    );
                    if let CandidateShapeFailure::Argument(error) = shape {
                        cause.notes = error.diagnostic_note().notes;
                    }
                    note.notes.push(cause);
                }
                CandidateProbeFailureKind::Constraint(constraint) => {
                    let mut cause = ast::DiagnosticNote::at(
                        constraint.code(),
                        self.current_text_file(),
                        diagnostic_span,
                        render_candidate_failure(self, candidate, failure, arguments),
                    );
                    if let crate::call_resolution::constraints::ConstraintFailureKind::MemberRequirement(failure) = &constraint.kind {
                        cause.notes = failure.notes.clone();
                    }
                    note.notes.push(cause);
                }
                CandidateProbeFailureKind::Context { code, span, reason } => {
                    note.notes.push(ast::DiagnosticNote::at(
                        code,
                        self.current_text_file(),
                        *span,
                        reason.clone(),
                    ));
                }
                CandidateProbeFailureKind::Expression { reason, .. }
                | CandidateProbeFailureKind::Declaration { reason, .. } => {
                    let causes = &state.diagnostics[self.diagnostics.len()..];
                    if causes.is_empty() {
                        note.notes.push(ast::DiagnosticNote::without_location(
                            scoop_source::DiagnosticCode::N0004,
                            reason.clone(),
                        ));
                    } else {
                        note.notes.extend(causes.iter().cloned().map(|cause| {
                            let mut cause = cause.into_note();
                            if let CandidateProbeFailureKind::Expression {
                                source_index,
                                expected,
                                ..
                            } = &failure.kind
                            {
                                cause.message = expression_failure_message(
                                    state,
                                    candidate,
                                    *source_index,
                                    *expected,
                                    &cause.message,
                                );
                            }
                            cause
                        }));
                    }
                }
            }
            if let Some(suggestion) =
                primitive_integer_conversion_suggestion(state, candidate, failure, arguments)
            {
                note.notes.push(ast::DiagnosticNote::without_location(
                    scoop_source::DiagnosticCode::H0501,
                    suggestion.trim_start_matches("; "),
                ));
            }
            if let CandidateProbeFailureKind::Constraint(constraint) = &failure.kind
                && let Some(InferenceVariableId::Callable(variable)) =
                    constraint.kind.inference_variable()
                && matches!(
                    explicit_type_args.get(InferenceVariableId::Callable(variable).group_index()),
                    Some(ResolvedCallTypeArgument::Infer { .. })
                )
            {
                let fixed = explicit_type_args
                    .iter()
                    .zip(&candidate.view.signature.callable_parameters)
                    .filter_map(|(argument, parameter)| {
                        let ResolvedCallTypeArgument::Explicit { ty, .. } = argument else {
                            return None;
                        };
                        Some(format!("{} = {}", parameter.name, state.type_name(*ty)))
                    })
                    .collect::<Vec<_>>();
                if !fixed.is_empty() {
                    note.notes.push(ast::DiagnosticNote::without_location(
                        scoop_source::DiagnosticCode::N0702,
                        format!("fixed type arguments: {}", fixed.join(", ")),
                    ));
                }
            }
            diagnostic.notes.push(note);
        }
        self.diagnostics.push(diagnostic);
        true
    }

    pub(super) fn ambiguity_diagnostic(
        &mut self,
        name: &str,
        prepared: &[Candidate],
        tied: &[usize],
        context: AmbiguityContext<'_, '_>,
    ) {
        let AmbiguityContext {
            applicable,
            arguments,
            receiver_offset,
            span,
        } = context;
        let views = prepared
            .iter()
            .map(|candidate| candidate.view.clone())
            .collect::<Vec<_>>();
        let layer = callable_layer_name(self, &views);
        let mut diagnostic = ast::Diagnostic::at_file(
            scoop_source::DiagnosticCode::E0712,
            self.current_text_file(),
            span,
            format!("call to `{name}` is ambiguous in {layer} layer"),
        );
        for &index in tied {
            let candidate = &prepared[index];
            let transaction = applicable
                .iter()
                .find(|transaction| transaction.candidate == index)
                .expect("every tied candidate has an applicability transaction");
            let mut note = candidate_note(
                self,
                candidate,
                callable_source_signature(self, name, &candidate.view),
            );
            note.notes.push(ast::DiagnosticNote::without_location(
                scoop_source::DiagnosticCode::N0703,
                "tied after pairwise declaration forwarding",
            ));
            if let Some(commits) =
                render_literal_exact_commits(candidate, transaction, arguments, receiver_offset)
            {
                note.notes.push(ast::DiagnosticNote::without_location(
                    scoop_source::DiagnosticCode::N0704,
                    format!("requires integer literal exact commits: {commits}"),
                ));
            }
            diagnostic.notes.push(note);
        }
        self.diagnostics.push(diagnostic);
    }
}

fn candidate_note(state: &Lowerer, candidate: &Candidate, message: String) -> ast::DiagnosticNote {
    match state.function_definition_file(candidate.function) {
        Some(file) => ast::DiagnosticNote::at(
            scoop_source::DiagnosticCode::N0002,
            file,
            candidate.view.declaration_span,
            message,
        ),
        None => ast::DiagnosticNote::without_location(scoop_source::DiagnosticCode::N0002, message),
    }
}

fn render_candidate_failure(
    original: &Lowerer,
    candidate: &Candidate,
    failure: &CandidateProbeFailure,
    _arguments: &OverloadArguments<'_>,
) -> String {
    let state = failure.state(original);
    match &failure.kind {
        CandidateProbeFailureKind::Shape(CandidateShapeFailure::TypeArgumentArity {
            expected,
            supplied,
        }) => format!("expects {expected} explicit type argument(s), but {supplied} were supplied"),
        CandidateProbeFailureKind::Shape(CandidateShapeFailure::Argument(failure)) => {
            failure.describe()
        }
        CandidateProbeFailureKind::Expression {
            source_index,
            expected,
            reason,
            ..
        } => expression_failure_message(state, candidate, *source_index, *expected, reason),
        CandidateProbeFailureKind::Declaration { reason, .. }
        | CandidateProbeFailureKind::Context { reason, .. } => reason.clone(),
        CandidateProbeFailureKind::Constraint(constraint) => render_callable_constraint_failure(
            state,
            &candidate.view,
            candidate.argument_map.as_ref().ok(),
            &failure.arguments,
            constraint,
        ),
    }
}

fn expression_failure_message(
    state: &Lowerer,
    candidate: &Candidate,
    source_index: usize,
    expected: Option<hir::TypeId>,
    reason: &str,
) -> String {
    let parameter = candidate
        .argument_map
        .as_ref()
        .ok()
        .map(|mapping| {
            let (parameter, _) = mapping.source_binding(
                crate::call_resolution::arguments::SourceInputId::from_index(source_index),
            );
            parameter.index()
        })
        .and_then(|index| candidate.view.signature.value_parameters.get(index))
        .map(|parameter| format!("argument for `{}`", parameter.name))
        .unwrap_or_else(|| format!("argument {}", source_index + 1));
    match expected {
        Some(expected) => format!(
            "{parameter} (expected {}): {reason}",
            state.type_name(expected)
        ),
        None => format!("{parameter}: {reason}"),
    }
}

fn common_failure_span(
    failures: &[CandidateProbeFailure],
    arguments: &OverloadArguments<'_>,
    extension_receiver: Option<&hir::Expr>,
    explicit_type_args: &[ResolvedCallTypeArgument],
    fallback: Span,
) -> Span {
    let Some(first) = failures.first() else {
        return fallback;
    };
    let first = candidate_failure_span(
        first,
        arguments,
        extension_receiver,
        explicit_type_args,
        fallback,
    );
    if failures[1..].iter().all(|failure| {
        candidate_failure_span(
            failure,
            arguments,
            extension_receiver,
            explicit_type_args,
            fallback,
        ) == first
    }) {
        first
    } else {
        fallback
    }
}

fn candidate_failure_span(
    failure: &CandidateProbeFailure,
    arguments: &OverloadArguments<'_>,
    extension_receiver: Option<&hir::Expr>,
    explicit_type_args: &[ResolvedCallTypeArgument],
    fallback: Span,
) -> Span {
    match &failure.kind {
        CandidateProbeFailureKind::Shape(_) => fallback,
        CandidateProbeFailureKind::Declaration { span, .. }
        | CandidateProbeFailureKind::Context { span, .. } => *span,
        CandidateProbeFailureKind::Expression { span, .. } => *span,
        CandidateProbeFailureKind::Constraint(failure) => constraint_failure_span(
            arguments,
            extension_receiver,
            explicit_type_args,
            failure,
            fallback,
        ),
    }
}

fn constraint_failure_span(
    arguments: &OverloadArguments<'_>,
    extension_receiver: Option<&hir::Expr>,
    explicit_type_args: &[ResolvedCallTypeArgument],
    failure: &ConstraintFailure,
    fallback: Span,
) -> Span {
    if let Some(crate::call_resolution::constraints::InferenceVariableId::Callable(variable)) =
        failure.kind.inference_variable()
        && let Some(argument) = explicit_type_args.get(
            crate::call_resolution::constraints::InferenceVariableId::Callable(variable)
                .group_index(),
        )
    {
        return argument.span();
    }
    match failure.origin {
        ConstraintOrigin::Argument(input) => match arguments {
            OverloadArguments::Source(arguments) => arguments
                .get(input.index())
                .map_or(fallback, ast::CallArgument::span),
            OverloadArguments::Lowered(arguments) => arguments
                .get(input.index())
                .map_or(fallback, |arg| arg.span),
        },
        ConstraintOrigin::Receiver => extension_receiver.map_or(fallback, |receiver| receiver.span),
        _ => fallback,
    }
}
