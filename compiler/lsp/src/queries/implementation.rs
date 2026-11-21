use lsp_server::{ErrorCode, ResponseError};
use lsp_types::{Location, Position};
use scoop_analysis::{
    AnalysisSnapshot, Cancellation, ProjectAnalysisError, SemanticSession, SourceId,
};
use serde_json::Value;

use super::semantic;
use crate::{convert, initialize::Settings};

pub(crate) fn implementation(
    snapshot: &AnalysisSnapshot,
    source_id: SourceId,
    position: Position,
    settings: Settings,
    semantics: &mut SemanticSession,
    cancellation: &Cancellation,
) -> Result<Value, ResponseError> {
    let Some(cursor) = semantic::cursor(snapshot, source_id, position, settings)? else {
        return Ok(Value::Null);
    };
    let requested = cancellation.clone();
    let result = semantics
        .member_implementations(snapshot, source_id, cursor.span, move || {
            requested.check().is_err()
        })
        .map_err(analysis_error)?;
    let origins = if let Some(result) = result {
        result
            .implementations
            .into_iter()
            .map(|implementation| implementation.origin)
            .collect::<Vec<_>>()
    } else {
        let requested = cancellation.clone();
        let result = semantics
            .type_implementations(snapshot, source_id, cursor.span, move || {
                requested.check().is_err()
            })
            .map_err(analysis_error)?;
        let Some(result) = result else {
            return Ok(Value::Null);
        };
        result
            .implementations
            .into_iter()
            .map(|implementation| implementation.origin)
            .collect()
    };
    super::json(declaration_locations(
        snapshot,
        cursor.project,
        origins,
        settings,
        cancellation,
    )?)
}

pub(crate) fn super_declarations(
    snapshot: &AnalysisSnapshot,
    source_id: SourceId,
    position: Position,
    settings: Settings,
    semantics: &mut SemanticSession,
    cancellation: &Cancellation,
) -> Result<Value, ResponseError> {
    let Some(cursor) = semantic::cursor(snapshot, source_id, position, settings)? else {
        return super::json(Vec::<Location>::new());
    };
    let Some(analysis) = semantic::declarations(semantics, snapshot, source_id, cancellation)?
    else {
        return super::json(Vec::<Location>::new());
    };
    let origins = analysis
        .super_declarations(&cursor.file.identity, cursor.span, || {
            cancellation.check().is_err() || snapshot.check_cancelled().is_err()
        })
        .map_err(|_| semantic::cancelled())?;
    super::json(declaration_locations(
        snapshot,
        cursor.project,
        origins,
        settings,
        cancellation,
    )?)
}

pub(crate) fn member_hierarchy(
    snapshot: &AnalysisSnapshot,
    source: SourceId,
    position: Position,
    settings: Settings,
    semantics: &mut SemanticSession,
    cancellation: &Cancellation,
) -> Result<Value, ResponseError> {
    let Some(cursor) = semantic::cursor(snapshot, source, position, settings)? else {
        return Ok(serde_json::json!({ "parents": [], "children": [] }));
    };
    let requested = cancellation.clone();
    let result = semantics
        .member_hierarchy(snapshot, source, cursor.span, move || {
            requested.check().is_err()
        })
        .map_err(analysis_error)?;
    let parents = declaration_locations(
        snapshot,
        cursor.project,
        result.parents,
        settings,
        cancellation,
    )?;
    let children = declaration_locations(
        snapshot,
        cursor.project,
        result.children,
        settings,
        cancellation,
    )?;
    Ok(serde_json::json!({ "parents": parents, "children": children }))
}

fn declaration_locations(
    snapshot: &AnalysisSnapshot,
    project: &scoop_analysis::ProjectInput,
    origins: Vec<scoop_hir::SemanticSourceLocation>,
    settings: Settings,
    cancellation: &Cancellation,
) -> Result<Vec<Location>, ResponseError> {
    let mut locations = Vec::new();
    for origin in origins {
        cancellation.check().map_err(|_| semantic::cancelled())?;
        snapshot
            .check_cancelled()
            .map_err(|_| semantic::cancelled())?;
        if let Some(location) = convert::definition_location(snapshot, project, &origin, settings)?
        {
            locations.push(location);
        }
    }
    locations.sort_by(|left, right| {
        (
            left.uri.as_str(),
            left.range.start.line,
            left.range.start.character,
        )
            .cmp(&(
                right.uri.as_str(),
                right.range.start.line,
                right.range.start.character,
            ))
    });
    locations.dedup();
    Ok(locations)
}

fn analysis_error(error: ProjectAnalysisError) -> ResponseError {
    match error {
        ProjectAnalysisError::Cancelled => semantic::cancelled(),
        _ => convert::error(
            ErrorCode::RequestFailed,
            "implementation analysis is unavailable for this project",
        ),
    }
}
