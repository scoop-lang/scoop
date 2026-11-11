use scoop_source::{SourceEdit, SourceLengthError, Span};
use scoop_syntax::SyntaxKind;

use crate::SourceInput;

impl SourceInput {
    /// Insert a semantically selected import into this immutable syntax version.
    pub fn import_fix(&self, path: &[String]) -> Result<Option<SourceEdit>, &SourceLengthError> {
        if path.is_empty() {
            return Ok(None);
        }
        let parsed = self.parse()?;
        let root = parsed.syntax();
        let header = if let Some(source) = parsed.ast() {
            Some(source.header())
        } else {
            parsed
                .declaration_data()
                .source
                .as_ref()
                .and_then(|source| {
                    source
                        .input(Span::new(0, root.text_range().end().into()))
                        .complete()
                })
        };
        let Some(header) = header else {
            return Ok(None);
        };
        let package_end = match header.package {
            scoop_ast::PackageSyntax::RootPackage => None,
            scoop_ast::PackageSyntax::QualifiedPackage { span, .. } => Some(span.end),
        };
        let last_header = header
            .imports
            .iter()
            .map(|import| match import {
                scoop_ast::ImportSyntax::Exact { span, .. }
                | scoop_ast::ImportSyntax::Star { span, .. } => span.end,
            })
            .chain(package_end)
            .max();
        let mut offset = last_header.unwrap_or(0);
        for token in root
            .descendants_with_tokens()
            .filter_map(|item| item.into_token())
        {
            let start = u32::from(token.text_range().start());
            let end = u32::from(token.text_range().end());
            if end <= offset {
                continue;
            }
            match token.kind() {
                SyntaxKind::Whitespace => {
                    if last_header.is_some()
                        && let Some(index) = newline_end(token.text())
                    {
                        offset = start + index as u32;
                        break;
                    }
                    offset = end;
                }
                SyntaxKind::LineComment | SyntaxKind::BlockComment => offset = end,
                SyntaxKind::Semicolon if last_header.is_some() => offset = end,
                // Documentation stays attached to its original declaration.
                _ => break,
            }
        }
        let text = self.text.as_str();
        let newline = text
            .char_indices()
            .find_map(|(index, ch)| match ch {
                '\r' => Some(if text[index..].starts_with("\r\n") {
                    "\r\n"
                } else {
                    "\r"
                }),
                '\n' => Some("\n"),
                _ => None,
            })
            .unwrap_or("\n");
        let prefix = if offset != 0 && !text[..offset as usize].ends_with(['\r', '\n']) {
            newline
        } else {
            ""
        };
        let path = path
            .iter()
            .map(|part| scoop_parser::identifier_spelling(part))
            .collect::<Vec<_>>()
            .join(".");
        Ok(Some(SourceEdit {
            source: self.version,
            span: Span::new(offset, offset),
            replacement: format!("{prefix}import {path}{newline}"),
        }))
    }
}

fn newline_end(text: &str) -> Option<usize> {
    text.char_indices().find_map(|(index, ch)| match ch {
        '\r' => Some(
            index
                + if text[index..].starts_with("\r\n") {
                    2
                } else {
                    1
                },
        ),
        '\n' => Some(index + 1),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use crate::AnalysisHost;
    use scoop_source::SourceText;

    #[test]
    fn imports_preserve_headers_comments_documentation_and_line_endings() {
        for (source, expected) in [
            (
                "//! dep: scoop:scoop.std:0.1.0\n/** Entry docs. */\nfun main() {}",
                "//! dep: scoop:scoop.std:0.1.0\nimport tools.`测 试`\n/** Entry docs. */\nfun main() {}",
            ),
            (
                "package app\r\nimport tools.old // keep\r\n/** Docs */\r\nfun main() {}",
                "package app\r\nimport tools.old // keep\r\nimport tools.`测 试`\r\n/** Docs */\r\nfun main() {}",
            ),
            (
                "package app /* trailing\ncomment */\nfun main() {}",
                "package app /* trailing\ncomment */\nimport tools.`测 试`\nfun main() {}",
            ),
            (
                "/* license */\n\nfun main() {}",
                "/* license */\n\nimport tools.`测 试`\nfun main() {}",
            ),
            ("package app", "package app\nimport tools.`测 试`\n"),
        ] {
            let mut host = AnalysisHost::default();
            let id = host.add_source(SourceText::new(source.to_owned()));
            let input = host.source(id).unwrap();
            let edit = input
                .import_fix(&["tools".into(), "测 试".into()])
                .unwrap()
                .unwrap();
            assert_eq!(edit.source, input.version);
            let mut result = source.to_owned();
            result.replace_range(
                edit.span.start as usize..edit.span.end as usize,
                &edit.replacement,
            );
            assert_eq!(result, expected);
            assert!(scoop_parser::parse(&result).is_ok(), "{result}");
        }
    }

    #[test]
    fn a_broken_header_has_no_import_edit_but_a_broken_body_can_have_one() {
        for (source, available) in [
            ("package\nfun main() {}", false),
            ("fun main() { unknown(", true),
        ] {
            let mut host = AnalysisHost::default();
            let id = host.add_source(SourceText::new(source.to_owned()));
            assert_eq!(
                host.source(id)
                    .unwrap()
                    .import_fix(&["tools".into(), "value".into()])
                    .unwrap()
                    .is_some(),
                available
            );
        }
    }
}
