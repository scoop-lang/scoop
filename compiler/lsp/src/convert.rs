use lsp_server::{ErrorCode, ResponseError};
use lsp_types::{
    Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, Location, Position, Range,
};
use scoop_source::{PositionEncoding, SourceText, Span, TextPosition};

mod location;
pub(crate) use location::{definition_location, location};

pub(crate) fn error(code: ErrorCode, message: impl ToString) -> ResponseError {
    ResponseError {
        code: code as i32,
        message: message.to_string(),
        data: match code {
            ErrorCode::RequestCanceled | ErrorCode::ServerCancelled => None,
            ErrorCode::ContentModified => {
                Some(serde_json::json!({"code": scoop_source::DiagnosticCode::E2504}))
            }
            ErrorCode::InternalError => {
                Some(serde_json::json!({"code": scoop_source::DiagnosticCode::ICE2502}))
            }
            ErrorCode::RequestFailed => {
                Some(serde_json::json!({"code": scoop_source::DiagnosticCode::E2505}))
            }
            ErrorCode::MethodNotFound => {
                Some(serde_json::json!({"code": scoop_source::DiagnosticCode::E2501}))
            }
            ErrorCode::InvalidParams => {
                Some(serde_json::json!({"code": scoop_source::DiagnosticCode::E2503}))
            }
            _ => Some(serde_json::json!({"code": scoop_source::DiagnosticCode::E2502})),
        },
    }
}

pub(crate) fn offset(
    text: &SourceText,
    position: Position,
    encoding: PositionEncoding,
) -> Result<u32, ResponseError> {
    let offset = text
        .line_index()
        .offset(
            TextPosition::new(position.line as usize, position.character as usize),
            encoding,
        )
        .map_err(|err| error(ErrorCode::InvalidParams, err))?;
    u32::try_from(offset).map_err(|err| error(ErrorCode::InvalidParams, err))
}

pub(crate) fn range(
    text: &SourceText,
    span: Span,
    encoding: PositionEncoding,
) -> Result<Range, ResponseError> {
    let index = text.line_index();
    let position = |offset| {
        let point = index
            .position(offset as usize, encoding)
            .map_err(|err| error(ErrorCode::InternalError, err))?;
        Ok(Position::new(point.line as u32, point.column as u32))
    };
    Ok(Range {
        start: position(span.start)?,
        end: position(span.end)?,
    })
}

pub(crate) fn diagnostic<S>(
    text: &SourceText,
    diagnostic: &scoop_source::Diagnostic<S>,
    encoding: PositionEncoding,
    mut note_location: impl FnMut(
        &scoop_source::DiagnosticNote<S>,
    ) -> Result<Option<Location>, ResponseError>,
) -> Result<Option<Diagnostic>, ResponseError> {
    let Some(span) = diagnostic.span() else {
        return Ok(None);
    };
    let mut notes = Vec::new();
    let mut imports = std::collections::BTreeSet::new();
    fn attachment<S>(
        note: &scoop_source::DiagnosticNote<S>,
        note_location: &mut impl FnMut(
            &scoop_source::DiagnosticNote<S>,
        ) -> Result<Option<Location>, ResponseError>,
        related: &mut Vec<DiagnosticRelatedInformation>,
        imports: &mut std::collections::BTreeSet<Vec<String>>,
    ) -> Result<serde_json::Value, ResponseError> {
        if !note.import_path.is_empty() {
            imports.insert(note.import_path.clone());
        }
        let location = note_location(note)?;
        if let Some(location) = &location {
            related.push(DiagnosticRelatedInformation {
                location: location.clone(),
                message: format!("[{}] {}", note.code, note.message),
            });
        }
        let children = note
            .notes
            .iter()
            .map(|child| attachment(child, note_location, related, imports))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(
            serde_json::json!({ "code": note.code.as_str(), "kind": note.code.role().as_str(), "message": note.message, "location": location, "importPath": note.import_path, "notes": children }),
        )
    }
    let attachments = diagnostic
        .notes
        .iter()
        .map(|note| attachment(note, &mut note_location, &mut notes, &mut imports))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Some(Diagnostic {
        range: range(text, span, encoding)?,
        severity: Some(match diagnostic.severity {
            scoop_source::DiagnosticSeverity::Error => DiagnosticSeverity::ERROR,
            scoop_source::DiagnosticSeverity::Warning => DiagnosticSeverity::WARNING,
        }),
        code: Some(lsp_types::NumberOrString::String(
            diagnostic.code.to_string(),
        )),
        source: Some("scoop".to_owned()),
        message: diagnostic.message.clone(),
        related_information: (!notes.is_empty()).then_some(notes),
        data: (!attachments.is_empty())
            .then(|| serde_json::json!({ "imports": imports, "notes": attachments })),
        ..Diagnostic::default()
    }))
}
