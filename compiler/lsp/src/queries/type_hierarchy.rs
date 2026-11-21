use lsp_server::{ErrorCode, ResponseError};
use lsp_types::{Position, SymbolKind, TypeHierarchyItem, Uri};
use scoop_analysis::{
    AnalysisSnapshot, Cancellation, ProjectAnalysisError, ProjectInput, SemanticSession, SourceId,
    TypeHierarchyDirection, TypeHierarchyNode, TypeHierarchyOrigin,
};
use scoop_hir::PublicNominalKindV1 as Kind;
use scoop_source::Span;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::semantic;
use crate::{convert, initialize::Settings};

#[derive(Clone)]
pub(crate) enum HierarchyQuery {
    Prepare(Position),
    Expand {
        data: ItemData,
        direction: TypeHierarchyDirection,
    },
}

#[derive(Clone, Deserialize, Serialize)]
pub(crate) struct ItemData {
    pub anchor: Uri,
    revision: u64,
    context: u64,
    source: u64,
    start: u32,
    end: u32,
}

impl ItemData {
    pub fn from_item(item: TypeHierarchyItem) -> Result<Self, ResponseError> {
        super::params(item.data.ok_or_else(|| {
            convert::error(
                ErrorCode::InvalidParams,
                "type hierarchy item data is required",
            )
        })?)
    }
}

pub(crate) fn run(
    snapshot: &AnalysisSnapshot,
    source: SourceId,
    anchor: &Uri,
    kind: HierarchyQuery,
    settings: Settings,
    semantics: &mut SemanticSession,
    cancellation: &Cancellation,
) -> Result<Value, ResponseError> {
    let Some(project) = snapshot.workspace.project_for_source(source) else {
        return super::json(Vec::<TypeHierarchyItem>::new());
    };
    let requested = cancellation.clone();
    let check = move || requested.check().is_err();
    let nodes =
        match kind {
            HierarchyQuery::Prepare(position) => {
                let Some(cursor) = semantic::cursor(snapshot, source, position, settings)? else {
                    return super::json(Vec::<TypeHierarchyItem>::new());
                };
                semantics.prepare_type_hierarchy(snapshot, source, cursor.span, check)
            }
            HierarchyQuery::Expand { data, direction } => {
                if data.revision != snapshot.revision.0 {
                    return Err(convert::error(
                        ErrorCode::ContentModified,
                        "the type hierarchy changed; prepare it again",
                    ));
                }
                semantics.type_hierarchy(
                    snapshot,
                    source,
                    TypeHierarchyOrigin {
                        context: SourceId(data.context),
                        source: SourceId(data.source),
                        span: Span::new(data.start, data.end),
                    },
                    direction,
                    check,
                )
            }
        }
        .map_err(|error| match error {
            ProjectAnalysisError::Cancelled
            | ProjectAnalysisError::Declarations(
                scoop_analysis::DeclarationAnalysisError::Cancelled,
            ) => semantic::cancelled(),
            _ => convert::error(
                ErrorCode::RequestFailed,
                "type hierarchy analysis is unavailable",
            ),
        })?;

    let mut items = Vec::new();
    for node in nodes {
        cancellation.check().map_err(|_| semantic::cancelled())?;
        if let Some(item) = item(snapshot, project, node, anchor, settings)? {
            items.push(item);
        }
    }
    items.sort_by(|left, right| {
        (&left.name, left.uri.as_str(), left.selection_range.start).cmp(&(
            &right.name,
            right.uri.as_str(),
            right.selection_range.start,
        ))
    });
    super::json(items)
}

fn item(
    snapshot: &AnalysisSnapshot,
    project: &ProjectInput,
    node: TypeHierarchyNode,
    anchor: &Uri,
    settings: Settings,
) -> Result<Option<TypeHierarchyItem>, ResponseError> {
    let declaration = node.declaration;
    let Some(file) = project.source_file(&declaration.origin.source) else {
        return Ok(None);
    };
    let Some(selection) =
        convert::definition_location(snapshot, project, &declaration.origin, settings)?
    else {
        return Ok(None);
    };
    let Some(location) = convert::location(snapshot, project, &declaration.origin, settings)?
    else {
        return Ok(None);
    };
    Ok(Some(TypeHierarchyItem {
        name: declaration.name,
        kind: match declaration.kind {
            Kind::Class => SymbolKind::CLASS,
            Kind::Enum => SymbolKind::ENUM,
            Kind::Interface => SymbolKind::INTERFACE,
            Kind::Object => SymbolKind::OBJECT,
            Kind::Struct => SymbolKind::STRUCT,
        },
        detail: None,
        tags: None,
        uri: location.uri,
        range: location.range,
        selection_range: selection.range,
        data: Some(super::json(ItemData {
            anchor: anchor.clone(),
            revision: snapshot.revision.0,
            context: node.context.0,
            source: file.source.0,
            start: declaration.origin.span.start,
            end: declaration.origin.span.end,
        })?),
    }))
}
