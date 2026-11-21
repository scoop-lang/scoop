use lsp_server::{ErrorCode, ResponseError};
use lsp_types::{
    CodeAction, CodeActionContext, CodeActionKind, DocumentChanges, OneOf,
    OptionalVersionedTextDocumentIdentifier, Range, TextDocumentEdit, TextEdit, Uri, WorkspaceEdit,
};
use scoop_analysis::{
    AnalysisSnapshot, Cancellation, ProjectAnalysisError, SemanticSession, SourceId, SourceInput,
    SyntaxFix,
};
use scoop_source::{SourceEdit, Span};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{convert, initialize::Settings};

mod imports;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct ResolveData {
    pub uri: Uri,
    source: u64,
    revision: u64,
    repair: Repair,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum Repair {
    Syntax {
        missing: usize,
    },
    Import {
        path: Vec<String>,
        start: u32,
        end: u32,
        analysis_revision: u64,
    },
    Extract {
        target: ExtractionTarget,
        start: u32,
        end: u32,
        analysis_revision: u64,
    },
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
enum ExtractionTarget {
    Variable,
    Function,
}

pub(crate) struct Context<'a> {
    pub uri: &'a Uri,
    pub source_id: SourceId,
    pub source: &'a SourceInput,
    pub version: Option<i32>,
    pub snapshot: &'a AnalysisSnapshot,
    pub settings: Settings,
    pub cancellation: &'a Cancellation,
}

#[derive(Clone)]
pub(crate) struct ActionQuery {
    pub range: Range,
    pub context: CodeActionContext,
}

pub(crate) fn actions(
    context: Context<'_>,
    query: &ActionQuery,
    semantics: &mut SemanticSession,
) -> Result<Value, ResponseError> {
    if !context.settings.code_actions {
        return Err(unsupported());
    }
    let quickfix = requested(&query.context, &CodeActionKind::QUICKFIX);
    let extract = requested(&query.context, &CodeActionKind::REFACTOR_EXTRACT);
    if !quickfix && !extract {
        return super::json(Vec::<CodeAction>::new());
    }
    let range = Span::new(
        convert::offset(
            &context.source.text,
            query.range.start,
            context.settings.encoding,
        )?,
        convert::offset(
            &context.source.text,
            query.range.end,
            context.settings.encoding,
        )?,
    );
    if range.start > range.end {
        return Err(convert::error(
            ErrorCode::InvalidParams,
            "reversed code action range",
        ));
    }
    let mut actions = Vec::new();
    if quickfix {
        let fixes = context
            .source
            .syntax_fixes(range)
            .map_err(|error| convert::error(ErrorCode::RequestFailed, error))?;
        for fix in fixes {
            context
                .cancellation
                .check()
                .map_err(|_| super::semantic::cancelled())?;
            actions.push(syntax_action(&context, fix)?);
        }
        for repair in imports::repairs(&context, semantics, range)? {
            actions.push(imports::action(&context, repair)?);
        }
    }
    if extract && range.start != range.end {
        for target in [ExtractionTarget::Variable, ExtractionTarget::Function] {
            actions.extend(extract_selection(&context, semantics, range, target)?);
        }
    }
    super::json(actions)
}

pub(crate) fn resolve(
    context: Context<'_>,
    data: ResolveData,
    semantics: &mut SemanticSession,
) -> Result<Value, ResponseError> {
    if !context.settings.code_action_resolve {
        return Err(unsupported());
    }
    if context.source.version.source.0 != data.source
        || context.source.version.revision.0 != data.revision
    {
        return Err(convert::error(
            ErrorCode::ContentModified,
            "code action source changed",
        ));
    }
    match data.repair {
        Repair::Syntax { missing } => {
            let fix = context
                .source
                .syntax_fix(missing)
                .map_err(|error| convert::error(ErrorCode::RequestFailed, error))?
                .ok_or_else(|| {
                    convert::error(ErrorCode::InvalidParams, "syntax repair is unavailable")
                })?;
            super::json(syntax_action(&context, fix)?)
        }
        Repair::Import {
            path,
            start,
            end,
            analysis_revision,
        } => {
            if context.snapshot.revision.0 != analysis_revision {
                return Err(convert::error(
                    ErrorCode::ContentModified,
                    "import context changed",
                ));
            }
            let repair = imports::repairs(&context, semantics, Span::new(start, end))?
                .into_iter()
                .find(|repair| repair.path == path && repair.diagnostic == Span::new(start, end))
                .ok_or_else(|| {
                    convert::error(ErrorCode::InvalidParams, "import repair is unavailable")
                })?;
            super::json(imports::action(&context, repair)?)
        }
        Repair::Extract {
            target,
            start,
            end,
            analysis_revision,
        } => {
            if context.snapshot.revision.0 != analysis_revision {
                return Err(convert::error(
                    ErrorCode::ContentModified,
                    "refactoring context changed",
                ));
            }
            let action = extract_selection(&context, semantics, Span::new(start, end), target)?
                .ok_or_else(|| {
                    convert::error(ErrorCode::InvalidParams, "extraction is unavailable")
                })?;
            super::json(action)
        }
    }
}

fn requested(context: &CodeActionContext, kind: &CodeActionKind) -> bool {
    context.only.as_ref().is_none_or(|kinds| {
        kinds.iter().any(|prefix| {
            prefix == &CodeActionKind::EMPTY
                || prefix == kind
                || kind
                    .as_str()
                    .strip_prefix(prefix.as_str())
                    .is_some_and(|suffix| suffix.starts_with('.'))
        })
    })
}

fn extract_selection(
    context: &Context<'_>,
    semantics: &mut SemanticSession,
    range: Span,
    target: ExtractionTarget,
) -> Result<Option<CodeAction>, ResponseError> {
    let cancellation = context.cancellation.clone();
    let check = move || cancellation.check().is_err();
    let result = match target {
        ExtractionTarget::Variable => {
            semantics.extract_local(context.snapshot, context.source_id, range, check)
        }
        ExtractionTarget::Function => {
            semantics.extract_function(context.snapshot, context.source_id, range, check)
        }
    };
    let edit = match result {
        Ok(edit) => edit,
        Err(
            ProjectAnalysisError::Cancelled
            | ProjectAnalysisError::Declarations(scoop_analysis::DeclarationAnalysisError::Cancelled),
        ) => return Err(super::semantic::cancelled()),
        Err(_) => None,
    };
    edit.map(|edit| {
        let title = match target {
            ExtractionTarget::Variable => "Extract Local Variable",
            ExtractionTarget::Function => "Extract Function",
        };
        action(
            context,
            title.to_owned(),
            edit,
            Repair::Extract {
                target,
                start: range.start,
                end: range.end,
                analysis_revision: context.snapshot.revision.0,
            },
        )
    })
    .transpose()
}

fn syntax_action(context: &Context<'_>, fix: SyntaxFix) -> Result<CodeAction, ResponseError> {
    action(
        context,
        format!("Insert `{}`", fix.edit.replacement),
        fix.edit,
        Repair::Syntax {
            missing: fix.missing,
        },
    )
}

fn action(
    context: &Context<'_>,
    title: String,
    edit: SourceEdit,
    repair: Repair,
) -> Result<CodeAction, ResponseError> {
    let kind = match repair {
        Repair::Extract { .. } => CodeActionKind::REFACTOR_EXTRACT,
        _ => CodeActionKind::QUICKFIX,
    };
    let data = context.settings.code_action_resolve.then(|| ResolveData {
        uri: context.uri.clone(),
        source: edit.source.source.0,
        revision: edit.source.revision.0,
        repair,
    });
    Ok(CodeAction {
        title,
        kind: Some(kind),
        edit: Some(WorkspaceEdit {
            document_changes: Some(DocumentChanges::Edits(vec![TextDocumentEdit {
                text_document: OptionalVersionedTextDocumentIdentifier {
                    uri: context.uri.clone(),
                    version: context.version,
                },
                edits: vec![OneOf::Left(TextEdit {
                    range: convert::range(
                        &context.source.text,
                        edit.span,
                        context.settings.encoding,
                    )?,
                    new_text: edit.replacement,
                })],
            }])),
            ..Default::default()
        }),
        data: data.map(super::json).transpose()?,
        ..Default::default()
    })
}

fn unsupported() -> ResponseError {
    convert::error(
        ErrorCode::MethodNotFound,
        "client does not support versioned code actions",
    )
}
