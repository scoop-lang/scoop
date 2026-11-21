use lsp_server::{ErrorCode, ResponseError};
use lsp_types::{Position, TextDocumentPositionParams, Uri};
use scoop_analysis::{
    AnalysisSnapshot, Cancellation, ProjectAnalysisError, SemanticSession, SourceId,
};
use serde::Deserialize;
use serde_json::Value;

use super::{rename::workspace_edit, semantic};
use crate::{convert, documents::file_path, initialize::Settings};

#[derive(Clone)]
pub(crate) enum RefactorQuery {
    Move {
        position: Position,
        target_uri: Uri,
    },
    PrepareSignature(Position),
    ChangeSignature {
        position: Position,
        order: Vec<usize>,
        revision: u64,
    },
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MoveParams {
    #[serde(flatten)]
    pub position: TextDocumentPositionParams,
    pub target_uri: Uri,
}

#[derive(Deserialize)]
pub(crate) struct SignatureParams {
    #[serde(flatten)]
    pub position: TextDocumentPositionParams,
    pub order: Vec<usize>,
    pub revision: u64,
}

pub(crate) fn run(
    snapshot: &AnalysisSnapshot,
    source: SourceId,
    query: RefactorQuery,
    settings: Settings,
    semantics: &mut SemanticSession,
    cancellation: &Cancellation,
) -> Result<Value, ResponseError> {
    if !settings.renames {
        return Err(convert::error(
            ErrorCode::MethodNotFound,
            "client does not support versioned workspace edits",
        ));
    }
    let position = match &query {
        RefactorQuery::Move { position, .. }
        | RefactorQuery::PrepareSignature(position)
        | RefactorQuery::ChangeSignature { position, .. } => *position,
    };
    let Some(cursor) = semantic::cursor(snapshot, source, position, settings)? else {
        return Ok(Value::Null);
    };
    let request = cancellation.clone();
    let check = move || request.check().is_err();
    let edits = match query {
        RefactorQuery::PrepareSignature(_) => {
            let parameters = semantics
                .prepare_change_signature(snapshot, source, cursor.span)
                .map_err(analysis_error)?;
            return Ok(parameters.map_or(Value::Null, |parameters| {
                serde_json::json!({
                    "parameters": parameters,
                    "revision": snapshot.revision.0,
                })
            }));
        }
        RefactorQuery::ChangeSignature {
            order, revision, ..
        } => {
            if revision != snapshot.revision.0 {
                return Err(convert::error(
                    ErrorCode::ContentModified,
                    "signature context changed",
                ));
            }
            semantics.change_signature(snapshot, source, cursor.span, &order, check)
        }
        RefactorQuery::Move { target_uri, .. } => {
            let Some(path) = file_path(&target_uri)? else {
                return Ok(Value::Null);
            };
            let (_, cone) = cursor
                .project
                .source_cone(source)
                .expect("selected source Cone");
            let Some(target) = cone.sources.iter().find(|file| file.path == path) else {
                return Ok(Value::Null);
            };
            semantics.move_declaration(snapshot, source, target.source, cursor.span, check)
        }
    }
    .map_err(analysis_error)?;
    match edits {
        Some(edits) => workspace_edit(snapshot, cursor.project, edits, settings),
        None => Ok(Value::Null),
    }
}

fn analysis_error(error: ProjectAnalysisError) -> ResponseError {
    match error {
        ProjectAnalysisError::Cancelled
        | ProjectAnalysisError::Declarations(scoop_analysis::DeclarationAnalysisError::Cancelled) => {
            semantic::cancelled()
        }
        _ => convert::error(
            ErrorCode::RequestFailed,
            "refactoring analysis is unavailable",
        ),
    }
}
