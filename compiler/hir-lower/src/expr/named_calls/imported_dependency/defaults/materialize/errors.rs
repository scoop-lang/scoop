use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ImportedDefaultMaterializationError {
    MissingReceiver,
    UnknownLocal(LocalValueSelector),
    ExpectedMaterializedLocal(LocalValueSelector),
    InvalidControlFlow(&'static str),
    MissingCallable(scoop_identity::CallableTemplateOrigin),
    DependencySelection(String),
    DefinitionOrigin(ImportedDefinitionOriginError),
    Plan(String),
    Requirement(crate::call_resolution::constraints::MemberRequirementFailure),
}

impl From<ImportedDefinitionOriginError> for ImportedDefaultMaterializationError {
    fn from(value: ImportedDefinitionOriginError) -> Self {
        Self::DefinitionOrigin(value)
    }
}

impl fmt::Display for ImportedDefaultMaterializationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingReceiver => {
                formatter.write_str("dependency default is missing its receiver value")
            }
            Self::UnknownLocal(local) => {
                write!(
                    formatter,
                    "dependency default reads unmapped local {local:?}"
                )
            }
            Self::ExpectedMaterializedLocal(local) => write!(
                formatter,
                "dependency default local {local:?} is not backed by a materialized local"
            ),
            Self::InvalidControlFlow(operation) => {
                write!(
                    formatter,
                    "invalid dependency default control flow: {operation}"
                )
            }
            Self::MissingCallable(callee) => {
                write!(
                    formatter,
                    "dependency default call {callee:?} has no declaration"
                )
            }
            Self::DependencySelection(error) => {
                write!(
                    formatter,
                    "dependency default callable selection failed: {error}"
                )
            }
            Self::DefinitionOrigin(error) => error.fmt(formatter),
            Self::Plan(error) => formatter.write_str(error),
            Self::Requirement(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for ImportedDefaultMaterializationError {}

impl ImportedDefaultMaterializationError {
    pub(crate) fn diagnostic(&self, file: usize, span: Span) -> scoop_ast::Diagnostic {
        let code = match self {
            Self::Requirement(error) => error.code.clone(),
            _ => scoop_source::DiagnosticCode::ICE0501.into(),
        };
        let mut diagnostic = scoop_ast::Diagnostic::at_file(code, file, span, self.to_string());
        if let Self::Requirement(error) = self {
            diagnostic.notes = error.notes.clone();
        }
        diagnostic
    }
}
