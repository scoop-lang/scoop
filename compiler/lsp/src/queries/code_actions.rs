use lsp_server::{ErrorCode, ResponseError};
use lsp_types::{
    CodeAction, CodeActionContext, CodeActionKind, Command, DocumentChanges, OneOf,
    OptionalVersionedTextDocumentIdentifier, Range, TextDocumentEdit, TextEdit, Uri, WorkspaceEdit,
};
use scoop_analysis::{
    AnalysisSnapshot, Cancellation, DeclarationExtraction, ProjectAnalysisError, SafeDelete,
    SemanticSession, SourceId, SourceInput, SyntaxFix,
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
    Refactor {
        target: Refactoring,
        start: u32,
        end: u32,
        analysis_revision: u64,
    },
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
enum Refactoring {
    ExtractVariable,
    ExtractFunction,
    ExtractTypeAlias,
    ExtractConstant,
    ExtractProperty,
    ExtractParameter,
    InlineVariable,
    InlineFunction,
    SafeDelete,
    OrganizeImports,
}

impl Refactoring {
    const ALL: [Self; 10] = [
        Self::ExtractVariable,
        Self::ExtractFunction,
        Self::ExtractTypeAlias,
        Self::ExtractConstant,
        Self::ExtractProperty,
        Self::ExtractParameter,
        Self::InlineVariable,
        Self::InlineFunction,
        Self::SafeDelete,
        Self::OrganizeImports,
    ];

    fn kind(self) -> CodeActionKind {
        match self {
            Self::ExtractVariable
            | Self::ExtractFunction
            | Self::ExtractTypeAlias
            | Self::ExtractConstant
            | Self::ExtractProperty
            | Self::ExtractParameter => CodeActionKind::REFACTOR_EXTRACT,
            Self::InlineVariable | Self::InlineFunction => CodeActionKind::REFACTOR_INLINE,
            Self::SafeDelete => CodeActionKind::REFACTOR_REWRITE,
            Self::OrganizeImports => CodeActionKind::SOURCE_ORGANIZE_IMPORTS,
        }
    }

    fn title(self) -> &'static str {
        match self {
            Self::ExtractVariable => "Extract Local Variable",
            Self::ExtractFunction => "Extract Function",
            Self::ExtractTypeAlias => "Introduce Type Alias",
            Self::ExtractConstant => "Extract Constant",
            Self::ExtractProperty => "Extract Property",
            Self::ExtractParameter => "Extract Parameter",
            Self::InlineVariable => "Inline Local Variable",
            Self::InlineFunction => "Inline Function",
            Self::SafeDelete => "Safe Delete Function",
            Self::OrganizeImports => "Organize Imports",
        }
    }
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
    let refactors = Refactoring::ALL
        .into_iter()
        .filter(|target| {
            (!matches!(target, Refactoring::OrganizeImports) || query.context.only.is_some())
                && requested(&query.context, &target.kind())
                && (query.range.start != query.range.end
                    || target.kind() != CodeActionKind::REFACTOR_EXTRACT)
        })
        .collect::<Vec<_>>();
    if !quickfix && refactors.is_empty() {
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
    for target in refactors {
        actions.extend(refactor(&context, semantics, range, target)?);
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
        Repair::Refactor {
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
            let action =
                refactor(&context, semantics, Span::new(start, end), target)?.ok_or_else(|| {
                    convert::error(ErrorCode::InvalidParams, "refactoring is unavailable")
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

fn refactor(
    context: &Context<'_>,
    semantics: &mut SemanticSession,
    range: Span,
    target: Refactoring,
) -> Result<Option<CodeAction>, ResponseError> {
    let cancellation = context.cancellation.clone();
    let check = move || cancellation.check().is_err();
    let edits = match target {
        Refactoring::ExtractVariable => analysis_result(semantics.extract_local(
            context.snapshot,
            context.source_id,
            range,
            check,
        ))?
        .map(|edit| vec![edit]),
        Refactoring::ExtractFunction => analysis_result(semantics.extract_function(
            context.snapshot,
            context.source_id,
            range,
            check,
        ))?
        .map(|edit| vec![edit]),
        Refactoring::ExtractTypeAlias => analysis_result(semantics.extract_type_alias(
            context.snapshot,
            context.source_id,
            range,
            check,
        ))?
        .map(|edit| vec![edit]),
        Refactoring::ExtractConstant
        | Refactoring::ExtractProperty
        | Refactoring::ExtractParameter => {
            let kind = match target {
                Refactoring::ExtractConstant => DeclarationExtraction::Constant,
                Refactoring::ExtractProperty => DeclarationExtraction::Property,
                _ => DeclarationExtraction::Parameter,
            };
            analysis_result(semantics.extract_declaration(
                context.snapshot,
                context.source_id,
                range,
                kind,
                check,
            ))?
            .map(|edit| vec![edit])
        }
        Refactoring::InlineVariable => analysis_result(semantics.inline_local(
            context.snapshot,
            context.source_id,
            range,
            check,
        ))?,
        Refactoring::InlineFunction => analysis_result(semantics.inline_function(
            context.snapshot,
            context.source_id,
            range,
            check,
        ))?
        .map(|edit| vec![edit]),
        Refactoring::OrganizeImports => {
            analysis_result(semantics.organize_imports(context.snapshot, context.source_id, check))?
        }
        Refactoring::SafeDelete => match analysis_result(semantics.safe_delete_function(
            context.snapshot,
            context.source_id,
            range,
            check,
        ))? {
            Some(SafeDelete::Edit(edit)) => Some(vec![edit]),
            Some(SafeDelete::Blocked(usages)) => {
                return delete_usages(context, range, usages).map(Some);
            }
            None => None,
        },
    };
    edits
        .map(|edits| {
            action(
                context,
                target.title().to_owned(),
                edits,
                Repair::Refactor {
                    target,
                    start: range.start,
                    end: range.end,
                    analysis_revision: context.snapshot.revision.0,
                },
            )
        })
        .transpose()
}

fn analysis_result<T>(
    result: Result<Option<T>, ProjectAnalysisError>,
) -> Result<Option<T>, ResponseError> {
    match result {
        Ok(result) => Ok(result),
        Err(
            ProjectAnalysisError::Cancelled
            | ProjectAnalysisError::Declarations(scoop_analysis::DeclarationAnalysisError::Cancelled),
        ) => Err(super::semantic::cancelled()),
        Err(_) => Ok(None),
    }
}

fn delete_usages(
    context: &Context<'_>,
    range: Span,
    usages: Vec<scoop_hir::SemanticSourceLocation>,
) -> Result<CodeAction, ResponseError> {
    let project = context
        .snapshot
        .workspace
        .project_for_source(context.source_id)
        .expect("safe delete requires a selected project");
    let mut locations = Vec::new();
    for usage in usages {
        if let Some(location) =
            convert::location(context.snapshot, project, &usage, context.settings)?
        {
            locations.push(location);
        }
    }
    let position = convert::range(&context.source.text, range, context.settings.encoding)?.start;
    let title = "Show Safe Delete Usages".to_owned();
    Ok(CodeAction {
        title: title.clone(),
        kind: Some(Refactoring::SafeDelete.kind()),
        command: Some(Command {
            title,
            command: "scoop.showSafeDeleteUsages".to_owned(),
            arguments: Some(vec![serde_json::json!({
                "uri": context.uri,
                "version": context.version,
                "position": position,
                "locations": locations,
            })]),
        }),
        ..Default::default()
    })
}

fn syntax_action(context: &Context<'_>, fix: SyntaxFix) -> Result<CodeAction, ResponseError> {
    action(
        context,
        format!("Insert `{}`", fix.edit.replacement),
        [fix.edit],
        Repair::Syntax {
            missing: fix.missing,
        },
    )
}

fn action(
    context: &Context<'_>,
    title: String,
    edits: impl IntoIterator<Item = SourceEdit>,
    repair: Repair,
) -> Result<CodeAction, ResponseError> {
    let kind = match repair {
        Repair::Refactor { target, .. } => target.kind(),
        _ => CodeActionKind::QUICKFIX,
    };
    let data = context.settings.code_action_resolve.then(|| ResolveData {
        uri: context.uri.clone(),
        source: context.source.version.source.0,
        revision: context.source.version.revision.0,
        repair,
    });
    let edits = edits
        .into_iter()
        .map(|edit| {
            debug_assert_eq!(edit.source, context.source.version);
            Ok(OneOf::Left(TextEdit {
                range: convert::range(&context.source.text, edit.span, context.settings.encoding)?,
                new_text: edit.replacement,
            }))
        })
        .collect::<Result<Vec<_>, ResponseError>>()?;
    Ok(CodeAction {
        title,
        kind: Some(kind),
        edit: Some(WorkspaceEdit {
            document_changes: Some(DocumentChanges::Edits(vec![TextDocumentEdit {
                text_document: OptionalVersionedTextDocumentIdentifier {
                    uri: context.uri.clone(),
                    version: context.version,
                },
                edits,
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
