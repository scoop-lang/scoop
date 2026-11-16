use scoop_identity::SourceIdentity;
use scoop_source::SourceText;
use std::fmt;

mod documentation;
mod notes;
mod protocol;
pub use documentation::{PublishedDocumentation, SingleConeDocumentationSuccess};
pub use protocol::DiagnosticMappingError;

use scoop_ast::{CurrentConeParsedSources, Diagnostic, SourceDiagnostic};
use scoop_slib::PublishedCrossConeArtifact;

use super::StageDumpKind;

#[derive(Debug)]
pub struct EmittedStageDump {
    kind: StageDumpKind,
    text: String,
}

impl EmittedStageDump {
    pub(super) fn new(kind: StageDumpKind, text: String) -> Self {
        Self { kind, text }
    }

    pub const fn kind(&self) -> StageDumpKind {
        self.kind
    }

    pub fn text(&self) -> &str {
        &self.text
    }
}

#[derive(Debug)]
struct CurrentConeDiagnosticSource {
    identity: SourceIdentity,
    display_locator: String,
    source_text: Option<SourceText>,
    in_memory: bool,
}

/// Published diagnostics use stable source identities; display text is optional.
#[derive(Debug)]
pub struct CurrentConeDiagnosticSet {
    diagnostics: Vec<SourceDiagnostic>,
    sources: Vec<CurrentConeDiagnosticSource>,
}

impl CurrentConeDiagnosticSet {
    pub(super) fn try_new(
        diagnostics: Vec<Diagnostic>,
        sources: &CurrentConeParsedSources,
    ) -> Result<Self, CurrentConeDiagnosticSetError> {
        Self::try_new_with_generated(diagnostics, sources, &[])
    }

    pub(super) fn try_new_with_generated(
        diagnostics: Vec<Diagnostic>,
        sources: &CurrentConeParsedSources,
        generated: &[scoop_hir::GeneratedMethodSource],
    ) -> Result<Self, CurrentConeDiagnosticSetError> {
        let diagnostics = Self::bind_local(diagnostics, sources)?;
        Ok(Self::from_bound(diagnostics, sources, generated))
    }

    pub(super) fn bind_local(
        diagnostics: Vec<Diagnostic>,
        sources: &CurrentConeParsedSources,
    ) -> Result<Vec<SourceDiagnostic>, CurrentConeDiagnosticSetError> {
        let identities = sources
            .iter()
            .map(|source| source.text().identity().clone())
            .collect::<Vec<_>>();
        diagnostics
            .into_iter()
            .map(|diagnostic| {
                diagnostic
                    .bind(&identities)
                    .map_err(CurrentConeDiagnosticSetError)
            })
            .collect()
    }

    pub(super) fn from_bound(
        diagnostics: Vec<SourceDiagnostic>,
        sources: &CurrentConeParsedSources,
        generated: &[scoop_hir::GeneratedMethodSource],
    ) -> Self {
        let mut sources = sources
            .iter()
            .map(|source| CurrentConeDiagnosticSource {
                identity: source.text().identity().clone(),
                display_locator: source.diagnostic().display_locator().display().to_string(),
                source_text: Some(source.text().source_text().clone()),
                in_memory: false,
            })
            .collect::<Vec<_>>();
        sources.extend(generated.iter().map(|source| CurrentConeDiagnosticSource {
            identity: source.identity.clone(),
            display_locator: source.name.clone(),
            source_text: Some(source.text.clone()),
            in_memory: true,
        }));
        for diagnostic in &diagnostics {
            if let Some(identity) = diagnostic.source() {
                source_index(&mut sources, identity);
            }
            register_note_sources(&mut sources, &diagnostic.notes);
        }
        Self {
            diagnostics,
            sources,
        }
    }

    pub fn diagnostics(&self) -> &[SourceDiagnostic] {
        &self.diagnostics
    }

    fn source(&self, identity: &SourceIdentity) -> &CurrentConeDiagnosticSource {
        self.sources
            .iter()
            .find(|source| &source.identity == identity)
            .expect("diagnostic display source was registered")
    }

    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }

    pub fn render_human(&self) -> String {
        let mut output = Vec::new();
        for diagnostic in &self.diagnostics {
            let kind = match diagnostic.severity {
                scoop_ast::DiagnosticSeverity::Error => "error",
                scoop_ast::DiagnosticSeverity::Warning => "warning",
            };
            output.push(
                match diagnostic.source().map(|identity| self.source(identity)) {
                    Some(source) => match &source.source_text {
                        Some(text) => diagnostic.render_with_source(&source.display_locator, text),
                        None => render_without_text(
                            source,
                            diagnostic.span(),
                            &format!("{kind}[{}]", diagnostic.code),
                            &diagnostic.message,
                        ),
                    },
                    None => format!("{kind}[{}]: {}", diagnostic.code, diagnostic.message),
                },
            );
            self.render_notes(&diagnostic.notes, &mut output);
        }
        output.join("\n")
    }

    fn render_notes(&self, notes: &[scoop_source::SourceDiagnosticNote], output: &mut Vec<String>) {
        for note in notes {
            let kind = format!("{}[{}]", note.code.role().as_str(), note.code);
            output.push(match note.source().map(|identity| self.source(identity)) {
                Some(source) => match &source.source_text {
                    Some(text) => note.render_with_source(&source.display_locator, text),
                    None => render_without_text(source, note.span(), &kind, &note.message),
                },
                None => format!("{kind}: {}", note.message),
            });
            self.render_notes(&note.notes, output);
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CurrentConeDiagnosticSetError(pub scoop_source::DiagnosticBindingError);
impl fmt::Display for CurrentConeDiagnosticSetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for CurrentConeDiagnosticSetError {}

#[derive(Debug)]
pub struct SingleConeProductionSuccess {
    artifact: PublishedCrossConeArtifact,
    warnings: CurrentConeDiagnosticSet,
    emitted_dumps: Vec<EmittedStageDump>,
    used_generators: Vec<scoop_protocol::GeneratorToolInputV1>,
    compiler_inputs: Vec<scoop_protocol::CompilerInputV1>,
    pub(in crate::request) documentation: Option<PublishedDocumentation>,
}

impl SingleConeProductionSuccess {
    pub(super) fn new_cross_cone(
        artifact: PublishedCrossConeArtifact,
        warnings: CurrentConeDiagnosticSet,
        emitted_dumps: Vec<EmittedStageDump>,
        used_generators: Vec<scoop_protocol::GeneratorToolInputV1>,
        compiler_inputs: Vec<scoop_protocol::CompilerInputV1>,
    ) -> Self {
        Self {
            artifact,
            warnings,
            emitted_dumps,
            used_generators,
            compiler_inputs,
            documentation: None,
        }
    }

    pub const fn artifact(&self) -> &PublishedCrossConeArtifact {
        &self.artifact
    }

    pub fn compiler_inputs(&self) -> &[scoop_protocol::CompilerInputV1] {
        &self.compiler_inputs
    }

    pub fn used_generators(&self) -> &[scoop_protocol::GeneratorToolInputV1] {
        &self.used_generators
    }

    pub const fn warnings(&self) -> &CurrentConeDiagnosticSet {
        &self.warnings
    }

    pub fn emitted_dumps(&self) -> &[EmittedStageDump] {
        &self.emitted_dumps
    }

    pub fn documentation(&self) -> Option<&PublishedDocumentation> {
        self.documentation.as_ref()
    }
}

#[derive(Debug)]
pub enum SingleConeCompilation {
    Published(Box<SingleConeProductionSuccess>),
    Observed(StageObservation),
    Documented(SingleConeDocumentationSuccess),
    Checked(SingleConeCheckSuccess),
}

#[derive(Debug)]
pub struct SingleConeCheckSuccess {
    pub(in crate::request) warnings: CurrentConeDiagnosticSet,
    pub(in crate::request) used_generators: Vec<scoop_protocol::GeneratorToolInputV1>,
    pub(in crate::request) compiler_inputs: Vec<scoop_protocol::CompilerInputV1>,
}

impl SingleConeCheckSuccess {
    pub fn compiler_inputs(&self) -> &[scoop_protocol::CompilerInputV1] {
        &self.compiler_inputs
    }

    pub fn used_generators(&self) -> &[scoop_protocol::GeneratorToolInputV1] {
        &self.used_generators
    }
}

#[derive(Debug)]
pub struct StageObservation {
    pub(super) warnings: CurrentConeDiagnosticSet,
    pub(super) dumps: Vec<EmittedStageDump>,
}

impl SingleConeCompilation {
    pub fn warnings(&self) -> &CurrentConeDiagnosticSet {
        match self {
            Self::Published(result) => result.warnings(),
            Self::Observed(result) => &result.warnings,
            Self::Documented(result) => &result.warnings,
            Self::Checked(result) => &result.warnings,
        }
    }

    pub fn emitted_dumps(&self) -> &[EmittedStageDump] {
        match self {
            Self::Published(result) => result.emitted_dumps(),
            Self::Observed(result) => &result.dumps,
            Self::Documented(result) => &result.dumps,
            Self::Checked(_) => &[],
        }
    }
}

fn source_index(
    sources: &mut Vec<CurrentConeDiagnosticSource>,
    identity: &SourceIdentity,
) -> usize {
    if let Some(index) = sources
        .iter()
        .position(|source| &source.identity == identity)
    {
        return index;
    }
    let index = sources.len();
    sources.push(CurrentConeDiagnosticSource {
        identity: identity.clone(),
        display_locator: format!("{}/{}", identity.cone(), identity.logical_path()),
        source_text: None,
        in_memory: false,
    });
    index
}

fn render_without_text(
    source: &CurrentConeDiagnosticSource,
    span: Option<scoop_ast::Span>,
    severity: &str,
    message: &str,
) -> String {
    let location = match span {
        Some(span) => format!(
            "{}:bytes {}..{}",
            source.display_locator, span.start, span.end
        ),
        None => source.display_locator.clone(),
    };
    format!("{location}: {severity}: {message}")
}

fn register_note_sources(
    sources: &mut Vec<CurrentConeDiagnosticSource>,
    notes: &[scoop_source::SourceDiagnosticNote],
) {
    for note in notes {
        if let Some(identity) = note.source() {
            source_index(sources, identity);
        }
        register_note_sources(sources, &note.notes);
    }
}
