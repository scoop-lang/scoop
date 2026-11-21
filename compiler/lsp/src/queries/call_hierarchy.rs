use lsp_server::{ErrorCode, ResponseError};
use lsp_types::{
    CallHierarchyIncomingCall, CallHierarchyItem, CallHierarchyOutgoingCall, Position, SymbolKind,
    Uri,
};
use scoop_analysis::{
    AnalysisSnapshot, CallHierarchyDirection, Cancellation, ProjectAnalysisError, ProjectInput,
    SemanticSession, SourceId,
};
use scoop_hir::EditingCallHierarchyDeclaration;
use scoop_source::Span;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::semantic;
use crate::{convert, initialize::Settings};

#[derive(Clone)]
pub(crate) enum CallQuery {
    Prepare(Position),
    Expand {
        data: CallItemData,
        direction: CallHierarchyDirection,
    },
}

#[derive(Clone, Deserialize, Serialize)]
pub(crate) struct CallItemData {
    pub anchor: Uri,
    revision: u64,
    source: u64,
    start: u32,
    end: u32,
}

impl CallItemData {
    pub fn from_item(item: CallHierarchyItem) -> Result<Self, ResponseError> {
        super::params(item.data.ok_or_else(|| {
            convert::error(
                ErrorCode::InvalidParams,
                "call hierarchy item data is required",
            )
        })?)
    }
}

pub(crate) fn run(
    snapshot: &AnalysisSnapshot,
    source: SourceId,
    anchor: &Uri,
    kind: CallQuery,
    settings: Settings,
    semantics: &mut SemanticSession,
    cancellation: &Cancellation,
) -> Result<Value, ResponseError> {
    let Some(project) = snapshot.workspace.project_for_source(source) else {
        return super::json(Vec::<CallHierarchyItem>::new());
    };
    let requested = cancellation.clone();
    let check = move || requested.check().is_err();
    match kind {
        CallQuery::Prepare(position) => {
            let Some(cursor) = semantic::cursor(snapshot, source, position, settings)? else {
                return super::json(Vec::<CallHierarchyItem>::new());
            };
            let nodes = semantics
                .prepare_call_hierarchy(snapshot, source, cursor.span, check)
                .map_err(analysis_error)?;
            let mut items = Vec::new();
            for node in nodes {
                cancellation.check().map_err(|_| semantic::cancelled())?;
                if let Some(item) = item(snapshot, project, node, anchor, settings)? {
                    items.push(item);
                }
            }
            items.sort_by(compare_items);
            super::json(items)
        }
        CallQuery::Expand { data, direction } => {
            if data.revision != snapshot.revision.0 {
                return Err(convert::error(
                    ErrorCode::ContentModified,
                    "the call hierarchy changed; prepare it again",
                ));
            }
            let relations = semantics
                .call_hierarchy(
                    snapshot,
                    source,
                    SourceId(data.source),
                    Span::new(data.start, data.end),
                    direction,
                    check,
                )
                .map_err(analysis_error)?;
            let mut items = Vec::new();
            for relation in relations {
                cancellation.check().map_err(|_| semantic::cancelled())?;
                let Some(item) = item(snapshot, project, relation.declaration, anchor, settings)?
                else {
                    continue;
                };
                let mut ranges = Vec::new();
                for site in relation.sites {
                    if let Some(location) = convert::location(snapshot, project, &site, settings)? {
                        ranges.push(location.range);
                    }
                }
                if !ranges.is_empty() {
                    items.push((item, ranges));
                }
            }
            items.sort_by(|(left, _), (right, _)| compare_items(left, right));
            match direction {
                CallHierarchyDirection::Incoming => super::json(
                    items
                        .into_iter()
                        .map(|(from, from_ranges)| CallHierarchyIncomingCall { from, from_ranges })
                        .collect::<Vec<_>>(),
                ),
                CallHierarchyDirection::Outgoing => super::json(
                    items
                        .into_iter()
                        .map(|(to, from_ranges)| CallHierarchyOutgoingCall { to, from_ranges })
                        .collect::<Vec<_>>(),
                ),
            }
        }
    }
}

fn compare_items(left: &CallHierarchyItem, right: &CallHierarchyItem) -> std::cmp::Ordering {
    (&left.name, left.uri.as_str(), left.selection_range.start).cmp(&(
        &right.name,
        right.uri.as_str(),
        right.selection_range.start,
    ))
}

fn item(
    snapshot: &AnalysisSnapshot,
    project: &ProjectInput,
    declaration: EditingCallHierarchyDeclaration,
    anchor: &Uri,
    settings: Settings,
) -> Result<Option<CallHierarchyItem>, ResponseError> {
    let Some(file) = project.source_file(&declaration.origin.source) else {
        return Ok(None);
    };
    let Some(source) = snapshot
        .source(file.source)
        .map_err(|_| semantic::cancelled())?
    else {
        return Ok(None);
    };
    let Some(selection) = convert::location(snapshot, project, &declaration.origin, settings)?
    else {
        return Ok(None);
    };
    let span = declaration.origin.span;
    let range = source
        .declaration_source_span(span)
        .map_err(|error| convert::error(ErrorCode::RequestFailed, error))?
        .map_or(Ok(selection.range), |span| {
            convert::range(&source.text, span, settings.encoding)
        })?;
    Ok(Some(CallHierarchyItem {
        name: declaration.name,
        kind: if declaration.method {
            SymbolKind::METHOD
        } else {
            SymbolKind::FUNCTION
        },
        detail: None,
        tags: None,
        uri: selection.uri,
        range,
        selection_range: selection.range,
        data: Some(super::json(CallItemData {
            anchor: anchor.clone(),
            revision: snapshot.revision.0,
            source: file.source.0,
            start: span.start,
            end: span.end,
        })?),
    }))
}

fn analysis_error(error: ProjectAnalysisError) -> ResponseError {
    match error {
        ProjectAnalysisError::Cancelled
        | ProjectAnalysisError::Declarations(scoop_analysis::DeclarationAnalysisError::Cancelled) => {
            semantic::cancelled()
        }
        _ => convert::error(
            ErrorCode::RequestFailed,
            "call hierarchy analysis is unavailable",
        ),
    }
}
