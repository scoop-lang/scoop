use crate::query::ResultPage;
use std::fmt::Write;

pub fn text(page: &ResultPage<'_>) -> String {
    let mut output = format!(
        "{} · {} matches, showing {} from {}\n",
        page.target,
        page.total,
        page.items.len(),
        page.offset
    );
    for entry in &page.items {
        output.push('\n');
        if let Some(declaration) = &entry.declaration {
            let _ = writeln!(
                output,
                "{} [{}]",
                declaration.qualified_name,
                entry.coordinate.as_deref().unwrap_or("")
            );
            let _ = writeln!(output, "{}\nkey: {}", declaration.signature, entry.key);
            if let Some(receiver) = &declaration.receiver {
                let _ = writeln!(output, "receiver: {receiver}");
            }
            if let Some(owner) = &entry.declaring_type {
                let _ = writeln!(output, "declared on: {}", owner.qualified_name);
            }
            if let Some(property) = &declaration.property {
                let _ = writeln!(
                    output,
                    "getter: {}; setter: {}",
                    property.getter.visibility,
                    property
                        .setter
                        .as_ref()
                        .map(|setter| setter.visibility.as_str())
                        .unwrap_or("none")
                );
            }
            if let Some(requirements) = &declaration.equality_requirements {
                let _ = writeln!(
                    output,
                    "derived equality requires each component: {}",
                    requirements.join(", ")
                );
            }
            if page.kind == "show" {
                if let Some(documentation) = &declaration.documentation {
                    let _ = writeln!(output, "{}", documentation.markdown);
                    for tag in &documentation.tags {
                        let _ = writeln!(output, "@{} {} {}", tag.kind, tag.label, tag.markdown);
                    }
                } else {
                    output.push_str("Documentation: not recorded.\n");
                }
                let _ = writeln!(
                    output,
                    "source: {}:{}\ndocumentation: {}",
                    declaration.source.path.as_deref().unwrap_or("not provided"),
                    declaration.source.line,
                    declaration.url
                );
            }
        } else {
            let _ = writeln!(
                output,
                "{}: declaration details were not published",
                entry.key
            );
        }
        for import in &entry.imports {
            let _ = writeln!(
                output,
                "import {} · dependency {}{}",
                import.path,
                import.coordinate,
                if import.reexport { " (re-export)" } else { "" }
            );
        }
    }
    output.push_str("\nThese APIs are documented for this target; add the actual dependency and import before use. Use scoop-lsp query for expressions in your project.\n");
    output
}
