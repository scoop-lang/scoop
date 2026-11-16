use super::*;
use scoop_source::DiagnosticCode as Code;

fn rejects(source: &str) -> Vec<ast::Diagnostic> {
    lower(&[complete_core_file(), scoop_parser::parse(source).unwrap()]).unwrap_err()
}

fn note_codes<S>(notes: &[ast::DiagnosticNote<S>]) -> Vec<scoop_source::DiagnosticIdentifier> {
    notes
        .iter()
        .flat_map(|note| std::iter::once(note.code.clone()).chain(note_codes(&note.notes)))
        .collect()
}

#[test]
fn type_errors_keep_the_rule_and_user_expression_location() {
    let source = "fun answer(): Int = true\nfun main() {}";
    let diagnostics = rejects(source);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    let diagnostic = &diagnostics[0];
    assert_eq!(diagnostic.code, Code::E0501);
    let span = diagnostic.span().unwrap();
    assert_eq!(&source[span.start as usize..span.end as usize], "true");
}

#[test]
fn function_and_constructor_candidates_keep_their_actual_argument_rules() {
    for (source, cause) in [
        (
            "fun accept(value: String) {}\nfun main() { accept(true) }",
            Code::E0501,
        ),
        (
            "struct Pair(val first: Int, val second: Int)\nfun main() { Pair(1) }",
            Code::E0701,
        ),
    ] {
        let diagnostics = rejects(source);
        assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
        assert_eq!(diagnostics[0].code, Code::E0711);
        assert_eq!(diagnostics[0].notes[0].code, Code::N0002);
        assert!(
            note_codes(&diagnostics[0].notes).contains(&cause.into()),
            "{diagnostics:#?}"
        );
        assert!(!diagnostics[0].message.contains(" — "));
    }
}

#[test]
fn pattern_repair_is_an_attachment_to_the_pattern_error() {
    let diagnostics = rejects("fun main() { when (1) { case missing -> {}; else -> {} } }");
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == Code::E0901)
        .unwrap_or_else(|| panic!("{diagnostics:#?}"));
    assert_eq!(diagnostic.notes[0].code, Code::H0901);
    assert_eq!(
        diagnostic.notes[0].message,
        "use `val missing` to bind the matched value"
    );
    assert!(!diagnostic.message.contains("; use"));
}
