use super::*;

fn warning_names(source: &str) -> Vec<String> {
    let output = lower_user_output(scoop_parser::parse(source).unwrap()).unwrap();
    output
        .warnings
        .iter()
        .map(|warning| {
            assert_eq!(warning.severity, ast::DiagnosticSeverity::Warning);
            assert!(warning.message.contains("shadows an enum variant"));
            let span = warning.span.unwrap();
            source[span.start as usize..span.end as usize].to_owned()
        })
        .collect()
}

#[test]
fn prelude_variant_shadowing_warns_without_rejecting_the_binding() {
    assert_eq!(
        warning_names("fun main() { val None = 1; println(None) }"),
        ["None"]
    );
    assert_eq!(
        warning_names("val None = 1\nfun main() { println(None) }"),
        ["None"]
    );
    assert_eq!(
        warning_names("fun main() { var None = 1; None = 2 }"),
        ["None"]
    );
}

#[test]
fn variant_warnings_follow_imports_aliases_and_exact_binding_types() {
    for source in [
        "import Signal.*\nenum Signal { Ready }\nfun main() { val Ready = 1 }",
        "enum Signal { Ready }\nfun main() { val Ready = Signal.Ready }",
    ] {
        assert_eq!(warning_names(source), ["Ready"]);
    }
    assert_eq!(
        warning_names(
            "import Signal.Ready as Next\nenum Signal { Ready }\nfun main() { val Next = 1 }"
        ),
        ["Next"]
    );
    assert_eq!(
        warning_names("import Signal.*\nenum Signal { Ready }\nval Ready = 1\nfun main() {}"),
        ["Ready"]
    );
}

#[test]
fn unrelated_or_already_shadowed_variants_do_not_warn() {
    assert!(
        warning_names("enum Signal { Ready }\nfun main() { val Ready = 1; val Uppercase = 2 }")
            .is_empty()
    );
    assert!(
        warning_names("class Box(val None: Int) { fun read() { val None = 2 } }\nfun main() {}")
            .is_empty()
    );
    assert_eq!(
        warning_names("fun main() { val None = 1; if (true) { val None = 2; println(None) } }"),
        ["None"]
    );
    assert_eq!(
        warning_names("val None = 1\nfun main() { val None = 2 }"),
        ["None"]
    );
}

#[test]
fn nested_declaration_lambda_and_for_bindings_share_the_warning_rule() {
    let source = r#"
fun main() {
    val (None, _) = (1, 2)
    println(None)
}
fun lambda() { val read: (Int) -> Int = { Some -> Some }; read(1) }
fun iterate(values: Iterable<Int>) { for (None in values) { println(None) } }
"#;
    assert_eq!(warning_names(source), ["None", "Some", "None"]);
}

#[test]
fn speculative_result_typing_reports_a_binding_warning_once() {
    let source = r#"
enum E { A, B }
fun choose(value: E): Int = when(value) {
    case A -> 1
    case val B -> 2
}
fun main() { choose(E.B) }
"#;
    assert_eq!(warning_names(source), ["B"]);
}
