use crate::{SourceText, Span};
use scoop_identity::SourceIdentity;

mod code;
mod identifier;
mod render;
pub use code::{DiagnosticCategory, DiagnosticCode, DiagnosticRole};
pub use identifier::DiagnosticIdentifier;

/// Severity of one compiler diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
}

impl DiagnosticSeverity {
    fn level(self) -> annotate_snippets::Level<'static> {
        match self {
            Self::Error => annotate_snippets::Level::ERROR,
            Self::Warning => annotate_snippets::Level::WARNING,
        }
    }
}

/// A source index is meaningful only inside its producing compilation context.
/// Imported and generated diagnostics already carry their canonical source.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum LocalDiagnosticSource {
    Index(usize),
    Bound(Box<SourceIdentity>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticLocation<S> {
    Source { source: S, span: Option<Span> },
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticKind {
    Source,
    Internal,
}

/// Local diagnostics never carry an independent index and canonical identity.
/// Published diagnostics use `SourceIdentity` and cannot contain local indices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic<S = LocalDiagnosticSource> {
    pub severity: DiagnosticSeverity,
    pub code: DiagnosticIdentifier,
    pub location: DiagnosticLocation<S>,
    pub message: String,
    pub is_argument_root: bool,
    pub notes: Vec<DiagnosticNote<S>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticNote<S = LocalDiagnosticSource> {
    pub code: DiagnosticIdentifier,
    pub location: DiagnosticLocation<S>,
    pub message: String,
    pub import_path: Vec<String>,
    pub notes: Vec<DiagnosticNote<S>>,
}

pub type SourceDiagnostic = Diagnostic<SourceIdentity>;
pub type SourceDiagnosticNote = DiagnosticNote<SourceIdentity>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiagnosticBindingError {
    pub index: usize,
    pub source_count: usize,
}

impl std::fmt::Display for DiagnosticBindingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "diagnostic source index {} is outside its producing context ({} sources)",
            self.index, self.source_count
        )
    }
}
impl std::error::Error for DiagnosticBindingError {}

pub fn bind_diagnostics(
    diagnostics: impl IntoIterator<Item = Diagnostic>,
    sources: &[SourceIdentity],
) -> Vec<SourceDiagnostic> {
    diagnostics
        .into_iter()
        .map(|diagnostic| {
            diagnostic
                .bind(sources)
                .unwrap_or_else(|error| SourceDiagnostic::internal(DiagnosticCode::ICE0401, error))
        })
        .collect()
}

impl LocalDiagnosticSource {
    fn bind(self, sources: &[SourceIdentity]) -> Result<SourceIdentity, DiagnosticBindingError> {
        match self {
            Self::Bound(source) => Ok(*source),
            Self::Index(index) => sources.get(index).cloned().ok_or(DiagnosticBindingError {
                index,
                source_count: sources.len(),
            }),
        }
    }
}

impl Diagnostic {
    pub fn at(
        code: impl Into<DiagnosticIdentifier>,
        span: Span,
        message: impl Into<String>,
    ) -> Self {
        Self::at_file(code, 0, span, message)
    }
    pub fn at_file(
        code: impl Into<DiagnosticIdentifier>,
        file: usize,
        span: Span,
        message: impl Into<String>,
    ) -> Self {
        let mut result = Self::without_span(code, DiagnosticSeverity::Error, file, message);
        result.location = DiagnosticLocation::Source {
            source: LocalDiagnosticSource::Index(file),
            span: Some(span),
        };
        result
    }
    pub fn warning_at(
        code: impl Into<DiagnosticIdentifier>,
        span: Span,
        message: impl Into<String>,
    ) -> Self {
        Self::warning_at_file(code, 0, span, message)
    }
    pub fn warning_at_file(
        code: impl Into<DiagnosticIdentifier>,
        file: usize,
        span: Span,
        message: impl Into<String>,
    ) -> Self {
        let mut result = Self::at_file(code, file, span, message);
        result.severity = DiagnosticSeverity::Warning;
        result
    }
    pub fn without_span(
        code: impl Into<DiagnosticIdentifier>,
        severity: DiagnosticSeverity,
        file: usize,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            code: code.into(),
            location: DiagnosticLocation::Source {
                source: LocalDiagnosticSource::Index(file),
                span: None,
            },
            message: message.into(),
            is_argument_root: false,
            notes: Vec::new(),
        }
    }
    pub fn local_file(&self) -> Option<usize> {
        match self.source() {
            Some(LocalDiagnosticSource::Index(file)) => Some(*file),
            _ => None,
        }
    }
    pub fn set_file(&mut self, file: usize) {
        self.location = DiagnosticLocation::Source {
            source: LocalDiagnosticSource::Index(file),
            span: self.span(),
        };
    }
    pub fn set_source(&mut self, source: SourceIdentity) {
        self.location = DiagnosticLocation::Source {
            source: LocalDiagnosticSource::Bound(Box::new(source)),
            span: self.span(),
        };
    }
    pub fn reattribute_single_source(&mut self, file: usize) {
        self.set_file(file);
        for note in &mut self.notes {
            *note = note
                .clone()
                .map_sources(&mut |_| LocalDiagnosticSource::Index(file));
        }
    }
    pub fn bind(
        self,
        sources: &[SourceIdentity],
    ) -> Result<SourceDiagnostic, DiagnosticBindingError> {
        let location = match self.location {
            DiagnosticLocation::None => DiagnosticLocation::None,
            DiagnosticLocation::Source { source, span } => DiagnosticLocation::Source {
                source: source.bind(sources)?,
                span,
            },
        };
        let notes = self
            .notes
            .into_iter()
            .map(|note| note.bind(sources))
            .collect::<Result<_, _>>()?;
        Ok(Diagnostic {
            severity: self.severity,
            code: self.code,
            location,
            message: self.message,
            is_argument_root: self.is_argument_root,
            notes,
        })
    }
    /// Preserve a source before an inner lowering context is dropped. A broken
    /// local index becomes an explicit internal failure, never another file.
    pub fn resolve_sources(&mut self, sources: &[SourceIdentity]) {
        *self = match self.clone().bind(sources) {
            Ok(bound) => bound.map_sources(|source| LocalDiagnosticSource::Bound(Box::new(source))),
            Err(error) => Self::internal(DiagnosticCode::ICE0401, error),
        };
    }
}

impl<S> Diagnostic<S> {
    pub fn without_location(
        code: impl Into<DiagnosticIdentifier>,
        severity: DiagnosticSeverity,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            code: code.into(),
            location: DiagnosticLocation::None,
            message: message.into(),
            is_argument_root: false,
            notes: Vec::new(),
        }
    }

    pub fn internal(code: impl Into<DiagnosticIdentifier>, error: impl std::fmt::Display) -> Self {
        Self {
            severity: DiagnosticSeverity::Error,
            code: code.into(),
            location: DiagnosticLocation::None,
            message: format!("internal compiler error: {error}"),
            is_argument_root: false,
            notes: Vec::new(),
        }
    }
    pub fn kind(&self) -> DiagnosticKind {
        if self.code.category() == Some(DiagnosticCategory::Internal) {
            DiagnosticKind::Internal
        } else {
            DiagnosticKind::Source
        }
    }
    pub fn span(&self) -> Option<Span> {
        match &self.location {
            DiagnosticLocation::Source { span, .. } => *span,
            DiagnosticLocation::None => None,
        }
    }
    pub fn source(&self) -> Option<&S> {
        match &self.location {
            DiagnosticLocation::Source { source, .. } => Some(source),
            DiagnosticLocation::None => None,
        }
    }
    pub fn with_note(mut self, note: DiagnosticNote<S>) -> Self {
        self.notes.push(note);
        self
    }
    fn map_sources<T>(self, mut map: impl FnMut(S) -> T) -> Diagnostic<T> {
        Diagnostic {
            severity: self.severity,
            code: self.code,
            location: match self.location {
                DiagnosticLocation::None => DiagnosticLocation::None,
                DiagnosticLocation::Source { source, span } => DiagnosticLocation::Source {
                    source: map(source),
                    span,
                },
            },
            message: self.message,
            is_argument_root: self.is_argument_root,
            notes: self
                .notes
                .into_iter()
                .map(|note| note.map_sources(&mut map))
                .collect(),
        }
    }
    pub fn into_note(self) -> DiagnosticNote<S> {
        DiagnosticNote {
            code: self.code,
            location: self.location,
            message: self.message,
            import_path: Vec::new(),
            notes: self.notes,
        }
    }
    pub fn render(&self, file_name: &str, source: &str) -> String {
        self.render_with_code(file_name, source, Some(self.code.as_str()), false)
    }
    pub fn render_with_source(&self, file_name: &str, source: &SourceText) -> String {
        self.render(file_name, source.as_str())
    }
    pub fn render_with_code(
        &self,
        file_name: &str,
        source: &str,
        code: Option<&str>,
        color: bool,
    ) -> String {
        render::render(
            self.severity.level(),
            code,
            &self.message,
            file_name,
            source,
            self.span(),
            color,
        )
    }
}

impl DiagnosticNote {
    pub fn at(
        code: impl Into<DiagnosticIdentifier>,
        file: usize,
        span: Span,
        message: impl Into<String>,
    ) -> Self {
        Self::located(code, LocalDiagnosticSource::Index(file), span, message)
    }
    pub fn at_source(
        code: impl Into<DiagnosticIdentifier>,
        source: SourceIdentity,
        span: Span,
        message: impl Into<String>,
    ) -> Self {
        Self::located(
            code,
            LocalDiagnosticSource::Bound(Box::new(source)),
            span,
            message,
        )
    }
    pub fn local_file(&self) -> Option<usize> {
        match self.source() {
            Some(LocalDiagnosticSource::Index(file)) => Some(*file),
            _ => None,
        }
    }
    fn bind(
        self,
        sources: &[SourceIdentity],
    ) -> Result<SourceDiagnosticNote, DiagnosticBindingError> {
        let location = match self.location {
            DiagnosticLocation::None => DiagnosticLocation::None,
            DiagnosticLocation::Source { source, span } => DiagnosticLocation::Source {
                source: source.bind(sources)?,
                span,
            },
        };
        Ok(DiagnosticNote {
            code: self.code,
            location,
            message: self.message,
            import_path: self.import_path,
            notes: self
                .notes
                .into_iter()
                .map(|note| note.bind(sources))
                .collect::<Result<_, _>>()?,
        })
    }
}

impl<S> DiagnosticNote<S> {
    pub fn located(
        code: impl Into<DiagnosticIdentifier>,
        source: S,
        span: Span,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            location: DiagnosticLocation::Source {
                source,
                span: Some(span),
            },
            message: message.into(),
            import_path: Vec::new(),
            notes: Vec::new(),
        }
    }
    pub fn without_location(
        code: impl Into<DiagnosticIdentifier>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            location: DiagnosticLocation::None,
            message: message.into(),
            import_path: Vec::new(),
            notes: Vec::new(),
        }
    }
    pub fn with_note(mut self, note: Self) -> Self {
        self.notes.push(note);
        self
    }
    pub fn source(&self) -> Option<&S> {
        match &self.location {
            DiagnosticLocation::None => None,
            DiagnosticLocation::Source { source, .. } => Some(source),
        }
    }
    pub fn span(&self) -> Option<Span> {
        match self.location {
            DiagnosticLocation::None => None,
            DiagnosticLocation::Source { span, .. } => span,
        }
    }
    pub fn set_source(&mut self, source: S) {
        self.location = DiagnosticLocation::Source {
            source,
            span: self.span(),
        };
    }
    fn map_sources<T>(self, map: &mut impl FnMut(S) -> T) -> DiagnosticNote<T> {
        DiagnosticNote {
            code: self.code,
            location: match self.location {
                DiagnosticLocation::None => DiagnosticLocation::None,
                DiagnosticLocation::Source { source, span } => DiagnosticLocation::Source {
                    source: map(source),
                    span,
                },
            },
            message: self.message,
            import_path: self.import_path,
            notes: self
                .notes
                .into_iter()
                .map(|note| note.map_sources(map))
                .collect(),
        }
    }
    pub fn render(&self, file_name: &str, source: &str) -> String {
        self.render_with_color(file_name, source, false)
    }
    pub fn render_with_source(&self, file_name: &str, source: &SourceText) -> String {
        self.render(file_name, source.as_str())
    }
    pub fn render_with_color(&self, file_name: &str, source: &str, color: bool) -> String {
        let level = match self.code.role() {
            DiagnosticRole::Help => annotate_snippets::Level::HELP,
            DiagnosticRole::Note => annotate_snippets::Level::NOTE,
            DiagnosticRole::Cause => annotate_snippets::Level::ERROR.with_name("cause"),
        };
        render::render(
            level,
            Some(self.code.as_str()),
            &self.message,
            file_name,
            source,
            self.span(),
            color,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_render_uses_typed_severity_with_and_without_a_span() {
        let error = Diagnostic::at(crate::DiagnosticCode::E0501, Span::new(6, 10), "bad value");
        let rendered = error.render("main.scoop", "first\nvalue");
        assert!(
            rendered.starts_with("error[E0501]: bad value\n"),
            "{rendered}"
        );
        assert!(rendered.contains("--> main.scoop:2:1"), "{rendered}");
        assert!(rendered.contains("2 | value\n  | ^^^^"), "{rendered}");

        let warning = Diagnostic::warning_at(
            crate::DiagnosticCode::W0901,
            Span::new(6, 10),
            "suspicious value",
        );
        let rendered = warning.render("main.scoop", "first\nvalue");
        assert!(
            rendered.starts_with("warning[W0901]: suspicious value\n"),
            "{rendered}"
        );
        assert!(rendered.contains("--> main.scoop:2:1"), "{rendered}");
        assert!(rendered.contains("2 | value\n  | ^^^^"), "{rendered}");

        let no_span = Diagnostic::without_span(
            crate::DiagnosticCode::W1810,
            DiagnosticSeverity::Warning,
            0,
            "link warning",
        );
        assert_eq!(
            no_span.render("main.scoop", ""),
            "warning[W1810]: link warning\n --> main.scoop"
        );
    }

    #[test]
    fn diagnostic_notes_are_structured_and_render_their_own_location() {
        let note = DiagnosticNote::at(
            crate::DiagnosticCode::N0001,
            3,
            Span::new(13, 17),
            "declared here",
        );
        let diagnostic = Diagnostic::at(
            crate::DiagnosticCode::E0403,
            Span::new(0, 3),
            "cannot access",
        )
        .with_note(note.clone());

        assert_eq!(diagnostic.notes, vec![note.clone()]);
        let rendered = note.render("library.scoop", "first\nsecond\nthird");
        assert!(
            rendered.starts_with("note[N0001]: declared here\n"),
            "{rendered}"
        );
        assert!(rendered.contains("--> library.scoop:3:1"), "{rendered}");
        assert!(rendered.contains("3 | third\n  | ^^^^"), "{rendered}");
    }

    #[test]
    fn reattributes_primary_and_single_source_notes_together() {
        let mut diagnostic =
            Diagnostic::at(crate::DiagnosticCode::E0301, Span::new(0, 3), "duplicate").with_note(
                DiagnosticNote::at(
                    crate::DiagnosticCode::N0001,
                    0,
                    Span::new(8, 11),
                    "first declared here",
                ),
            );

        diagnostic.reattribute_single_source(4);

        assert_eq!(diagnostic.local_file(), Some(4));
        assert_eq!(diagnostic.notes[0].local_file(), Some(4));
    }
}

#[cfg(test)]
mod binding_tests {
    use super::*;
    use scoop_identity::{ConeIdentity, NormalizedSourcePath};

    fn source(path: &str) -> SourceIdentity {
        SourceIdentity::new(
            ConeIdentity::SINGLE_FILE,
            NormalizedSourcePath::new(path).unwrap(),
        )
    }

    #[test]
    fn binding_preserves_distinct_primary_and_related_source_contexts() {
        let primary = source("main.scoop");
        let imported = source("dependency.scoop");
        let other = source("other.scoop");
        let diagnostic = Diagnostic::at(DiagnosticCode::E0401, Span::new(0, 1), "use failed")
            .with_note(DiagnosticNote::at_source(
                crate::DiagnosticCode::N0001,
                imported.clone(),
                Span::new(4, 8),
                "declared here",
            ))
            .with_note(DiagnosticNote::at(
                crate::DiagnosticCode::N0001,
                1,
                Span::new(2, 3),
                "local reason",
            ))
            .bind(&[primary.clone(), other.clone()])
            .unwrap();
        assert_eq!(diagnostic.source(), Some(&primary));
        assert_eq!(diagnostic.notes[0].source(), Some(&imported));
        assert_eq!(diagnostic.notes[1].source(), Some(&other));
    }

    #[test]
    fn invalid_primary_or_note_indices_fail_at_the_producing_boundary() {
        let sources = [source("main.scoop")];
        assert_eq!(
            Diagnostic::at_file(DiagnosticCode::E0401, 1, Span::new(0, 1), "bad index")
                .bind(&sources)
                .unwrap_err(),
            DiagnosticBindingError {
                index: 1,
                source_count: 1
            }
        );
        let diagnostic = Diagnostic::at(crate::DiagnosticCode::E0401, Span::new(0, 1), "bad note")
            .with_note(DiagnosticNote::at(
                crate::DiagnosticCode::N0001,
                2,
                Span::new(2, 3),
                "bad index",
            ));
        assert_eq!(
            diagnostic.bind(&sources).unwrap_err(),
            DiagnosticBindingError {
                index: 2,
                source_count: 1
            }
        );
    }

    #[test]
    fn a_bound_inner_context_cannot_be_reinterpreted_by_an_outer_file_table() {
        let original = source("original.scoop");
        let unrelated = source("unrelated.scoop");
        let mut diagnostic =
            Diagnostic::at(crate::DiagnosticCode::E0501, Span::new(0, 1), "failure");
        diagnostic.resolve_sources(std::slice::from_ref(&original));
        let published = diagnostic.bind(&[unrelated]).unwrap();
        assert_eq!(published.source(), Some(&original));
        let internal = SourceDiagnostic::internal(
            crate::DiagnosticCode::ICE0401,
            DiagnosticBindingError {
                index: 2,
                source_count: 1,
            },
        );
        assert_eq!(internal.kind(), DiagnosticKind::Internal);
        assert_eq!(internal.location, DiagnosticLocation::None);
    }
}
