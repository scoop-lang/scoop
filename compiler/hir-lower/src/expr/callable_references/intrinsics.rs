//! Intrinsics without indirect-call semantics retain a distinct diagnostic.

use super::*;
use hir::ImportedCallableSource;
use std::fmt;

#[derive(Debug)]
pub(super) enum ReferenceFailureReason {
    Candidate(Box<scoop_source::DiagnosticNote>),
    Intrinsic {
        target: hir::SemanticCallable,
        note: Box<scoop_source::DiagnosticNote>,
    },
}

impl ReferenceFailureReason {
    pub(super) fn atomic() -> Self {
        Self::Candidate(Box::new(
            scoop_source::DiagnosticNote::without_location(
                scoop_source::DiagnosticCode::E1223,
                crate::expr::atomics::ATOMIC_REFERENCE_ERROR,
            )
            .with_note(scoop_source::DiagnosticNote::without_location(
                scoop_source::DiagnosticCode::H0702,
                "use a lambda with fixed orders",
            )),
        ))
    }

    pub(super) fn candidate(
        code: impl Into<scoop_source::DiagnosticIdentifier>,
        message: impl Into<String>,
    ) -> Self {
        Self::Candidate(Box::new(scoop_source::DiagnosticNote::without_location(
            code, message,
        )))
    }

    pub(super) fn upstream(diagnostics: &[scoop_ast::Diagnostic]) -> Self {
        if let Some(diagnostic) = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.kind() == scoop_ast::DiagnosticKind::Internal)
        {
            return Self::Candidate(Box::new(diagnostic.clone().into_note()));
        }
        let mut note = scoop_source::DiagnosticNote::without_location(
            scoop_source::DiagnosticCode::N0004,
            "declaration could not be completed",
        );
        note.notes = diagnostics
            .iter()
            .cloned()
            .map(|diagnostic| diagnostic.into_note())
            .collect();
        Self::Candidate(Box::new(note))
    }

    pub(super) fn internal_diagnostic(&self) -> Option<scoop_ast::Diagnostic> {
        let note = self.note();
        if note.code.category() != Some(scoop_source::DiagnosticCategory::Internal) {
            return None;
        }
        let mut diagnostic = scoop_ast::Diagnostic::without_location(
            note.code,
            scoop_source::DiagnosticSeverity::Error,
            note.message,
        );
        diagnostic.location = note.location;
        diagnostic.notes = note.notes;
        Some(diagnostic)
    }

    pub(super) fn note(&self) -> scoop_source::DiagnosticNote {
        match self {
            Self::Candidate(note) | Self::Intrinsic { note, .. } => (**note).clone(),
        }
    }
}

impl fmt::Display for ReferenceFailureReason {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Candidate(note) | Self::Intrinsic { note, .. } => {
                formatter.write_str(&note.message)
            }
        }
    }
}

fn restriction(kind: hir::IntrinsicFunctionKind) -> Option<(&'static str, &'static str)> {
    match kind {
        hir::IntrinsicFunctionKind::IncludeStr
        | hir::IntrinsicFunctionKind::IncludeBytes
        | hir::IntrinsicFunctionKind::IncludeDir => Some((
            "this intrinsic is evaluated at compile time and has no runtime callable entry",
            "import and call it directly with a constant path",
        )),
        hir::IntrinsicFunctionKind::CompileEnv => Some((
            "this intrinsic is evaluated at compile time and has no runtime callable entry",
            "import and call it directly with a constant name",
        )),
        hir::IntrinsicFunctionKind::CurrentSourceLocation => Some((
            "this intrinsic requires the source location of a direct call; indirect calls do not provide it",
            "call it directly where the location is needed, or pass a SourceLocation value explicitly",
        )),
        _ => None,
    }
}

fn failure(
    target: hir::SemanticCallable,
    name: &str,
    (explanation, help): (&'static str, &'static str),
) -> ReferenceFailureReason {
    ReferenceFailureReason::Intrinsic {
        target,
        note: Box::new(
            scoop_source::DiagnosticNote::without_location(
                scoop_source::DiagnosticCode::E0713,
                format!("compiler intrinsic `{name}` does not support callable references"),
            )
            .with_note(scoop_source::DiagnosticNote::without_location(
                scoop_source::DiagnosticCode::N0705,
                explanation,
            ))
            .with_note(scoop_source::DiagnosticNote::without_location(
                scoop_source::DiagnosticCode::H0701,
                help,
            )),
        ),
    }
}

impl Lowerer {
    pub(super) fn local_intrinsic_reference_failure(
        &self,
        function: hir::FunctionId,
    ) -> Option<ReferenceFailureReason> {
        let hir::FunctionKind::Intrinsic(intrinsic) = self.functions[function].kind else {
            return None;
        };
        Some(failure(
            hir::SemanticCallable::Current(function),
            &self.functions[function].name,
            restriction(intrinsic.kind)?,
        ))
    }

    pub(super) fn imported_intrinsic_reference_failure(
        &self,
        source: &impl ImportedCallableSource,
    ) -> Option<ReferenceFailureReason> {
        let interface = source.interface();
        let hir::CallableImplementationV1::Intrinsic(kind) = interface.effects().implementation()
        else {
            return None;
        };
        let reason = restriction(kind)?;
        let target =
            super::super::named_calls::imported_dependency::semantic_function_target(source)
                .expect("a restricted function intrinsic retains its function declaration");
        let declaration = self
            .dependencies
            .as_ref()
            .expect("an imported reference has a dependency catalog")
            .callable_declaration(interface.declaration())
            .expect("an imported candidate retains its declaration");
        Some(failure(target, declaration.name(), reason))
    }

    pub(super) fn report_intrinsic_reference_failure(
        &mut self,
        span: Span,
        reason: &ReferenceFailureReason,
    ) -> bool {
        let ReferenceFailureReason::Intrinsic { target, note } = reason else {
            return false;
        };
        self.observe_rejected_callable_reference(span, *target);
        let mut diagnostic =
            scoop_ast::Diagnostic::at(note.code.clone(), span, note.message.clone());
        diagnostic.notes = note.notes.clone();
        diagnostic.set_file(self.current_text_file());
        self.diagnostics.push(diagnostic);
        true
    }
}
