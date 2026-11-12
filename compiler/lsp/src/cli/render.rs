use std::fmt::Write;

use lsp_types::{
    CompletionResponse, DocumentSymbol, Documentation, Hover, HoverContents, Location,
    MarkedString, SignatureHelp,
};

use super::query::Output;

pub(super) fn text(output: &Output) -> serde_json::Result<String> {
    let mut text = format!("{}: {}\n", output.query, output.status);
    if !output.result.is_null() {
        match output.query {
            "hover" => {
                let hover: Hover = serde_json::from_value(output.result.clone())?;
                match hover.contents {
                    HoverContents::Scalar(value) => marked(&mut text, &value),
                    HoverContents::Array(values) => {
                        for value in values {
                            marked(&mut text, &value);
                        }
                    }
                    HoverContents::Markup(value) => {
                        let _ = writeln!(text, "{}", value.value);
                    }
                }
            }
            "signature" => {
                let signatures: SignatureHelp = serde_json::from_value(output.result.clone())?;
                for (index, signature) in signatures.signatures.iter().enumerate() {
                    let active = signatures.active_signature == Some(index as u32);
                    let _ = writeln!(
                        text,
                        "{}{}",
                        if active { "> " } else { "  " },
                        signature.label
                    );
                    if let Some(documentation) = &signature.documentation {
                        documentation_text(&mut text, documentation);
                    }
                }
                if let Some(parameter) = signatures.active_parameter {
                    let _ = writeln!(text, "active parameter: {}", parameter + 1);
                }
            }
            "completion" => {
                let completions: CompletionResponse =
                    serde_json::from_value(output.result.clone())?;
                let items = match completions {
                    CompletionResponse::Array(items) => items,
                    CompletionResponse::List(list) => list.items,
                };
                for item in items {
                    let _ = writeln!(
                        text,
                        "{}{}",
                        item.label,
                        item.detail.map(|d| format!(" — {d}")).unwrap_or_default()
                    );
                    if let Some(documentation) = &item.documentation {
                        documentation_text(&mut text, documentation);
                    }
                }
            }
            "definition" | "type-definition" | "implementation" | "references" => {
                let values = output
                    .result
                    .as_array()
                    .map(Vec::as_slice)
                    .unwrap_or_else(|| std::slice::from_ref(&output.result));
                for value in values {
                    if value.get("targetUri").is_some() {
                        let location: lsp_types::LocationLink =
                            serde_json::from_value(value.clone())?;
                        position(
                            &mut text,
                            location.target_uri.as_str(),
                            location.target_selection_range.start,
                        );
                    } else {
                        let location: Location = serde_json::from_value(value.clone())?;
                        position(&mut text, location.uri.as_str(), location.range.start);
                    }
                }
            }
            "symbols" => {
                let symbols: Vec<DocumentSymbol> = serde_json::from_value(output.result.clone())?;
                for symbol in symbols {
                    symbol_text(&mut text, &symbol, 0);
                }
            }
            "workspace-symbols" => {
                let symbols: Vec<lsp_types::SymbolInformation> =
                    serde_json::from_value(output.result.clone())?;
                for symbol in symbols {
                    let _ = write!(text, "{} — ", symbol.name);
                    position(
                        &mut text,
                        symbol.location.uri.as_str(),
                        symbol.location.range.start,
                    );
                }
            }
            "diagnostics" => {}
            "code-actions" => {
                let actions: Vec<lsp_types::CodeAction> =
                    serde_json::from_value(output.result.clone())?;
                for action in actions {
                    let _ = writeln!(text, "{}", action.title);
                    if let Some(lsp_types::DocumentChanges::Edits(documents)) =
                        action.edit.and_then(|edit| edit.document_changes)
                    {
                        for document in documents {
                            for edit in document.edits {
                                let edit = match edit {
                                    lsp_types::OneOf::Left(edit) => edit,
                                    lsp_types::OneOf::Right(edit) => edit.text_edit,
                                };
                                let _ = writeln!(
                                    text,
                                    "{}:{}:{}..{}:{} (buffer version {:?})\n{}",
                                    document.text_document.uri.as_str(),
                                    edit.range.start.line + 1,
                                    edit.range.start.character + 1,
                                    edit.range.end.line + 1,
                                    edit.range.end.character + 1,
                                    document.text_document.version,
                                    edit.new_text
                                );
                            }
                        }
                    }
                }
            }
            _ => {
                let _ = writeln!(text, "{}", serde_json::to_string_pretty(&output.result)?);
            }
        }
    }
    for params in &output.diagnostics {
        for diagnostic in &params.diagnostics {
            position(&mut text, params.uri.as_str(), diagnostic.range.start);
            let _ = writeln!(text, "  {}", diagnostic.message);
            for note in diagnostic.related_information.iter().flatten() {
                position(
                    &mut text,
                    note.location.uri.as_str(),
                    note.location.range.start,
                );
                let _ = writeln!(text, "  note: {}", note.message);
            }
        }
    }
    for message in &output.messages {
        let _ = writeln!(text, "server: {}", message.message);
    }
    Ok(text)
}

fn marked(text: &mut String, value: &MarkedString) {
    let value = match value {
        MarkedString::String(value) => value,
        MarkedString::LanguageString(value) => &value.value,
    };
    let _ = writeln!(text, "{value}");
}

fn documentation_text(text: &mut String, value: &Documentation) {
    let value = match value {
        Documentation::String(value) => value,
        Documentation::MarkupContent(value) => &value.value,
    };
    let _ = writeln!(text, "{value}");
}

fn position(text: &mut String, uri: &str, point: lsp_types::Position) {
    let path = url::Url::parse(uri)
        .ok()
        .and_then(|u| u.to_file_path().ok())
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| uri.to_owned());
    let _ = writeln!(
        text,
        "{}:{}:{}",
        path,
        u64::from(point.line) + 1,
        u64::from(point.character) + 1
    );
}

fn symbol_text(text: &mut String, symbol: &DocumentSymbol, depth: usize) {
    let _ = writeln!(
        text,
        "{}{}{} ({}:{})",
        "  ".repeat(depth),
        symbol.name,
        symbol
            .detail
            .as_ref()
            .map(|d| format!(" — {d}"))
            .unwrap_or_default(),
        u64::from(symbol.selection_range.start.line) + 1,
        u64::from(symbol.selection_range.start.character) + 1
    );
    for child in symbol.children.iter().flatten() {
        symbol_text(text, child, depth + 1);
    }
}
