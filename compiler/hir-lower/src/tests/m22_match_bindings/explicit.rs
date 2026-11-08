use super::*;

#[test]
fn bare_typo_is_an_error_at_the_name_even_with_an_else() {
    let source = "enum E { N(Int), S(String) }\nfun main() { when (E.N(100)) { case Bad -> {}; else -> {} } }";
    let errors = lower_user_output(scoop_parser::parse(source).unwrap()).unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].severity, ast::DiagnosticSeverity::Error);
    assert_eq!(
        errors[0].message,
        "enum `E` has no variant `Bad`; use `val Bad` to bind the matched value"
    );
    let span = errors[0].span.unwrap();
    assert_eq!(&source[span.start as usize..span.end as usize], "Bad");
}

#[test]
fn implicit_bindings_are_rejected_at_every_match_depth() {
    for (parameter, pattern, name, ty) in [
        ("value: Int", "number", "number", "Int"),
        ("value: E", "N(number)", "number", "Int"),
        ("value: (Int, E)", "(number, _)", "number", "Int"),
        ("value: Box", "Box { payload: N(number) }", "number", "Int"),
        ("value: Box", "Box { payload }", "payload", "E"),
    ] {
        let source = format!(
            "enum E {{ N(Int), S(String) }}\nstruct Box(val payload: E)\nfun check({parameter}) {{ when(value) {{ case {pattern} -> {{}}; else -> {{}} }} }}\nfun main() {{}}"
        );
        let errors = lower_user_output(scoop_parser::parse(&source).unwrap()).unwrap_err();
        assert_eq!(errors.len(), 1, "{pattern}: {errors:?}");
        assert!(errors[0].message.contains(&format!("`{name}`")));
        assert!(errors[0].message.contains(ty));
        assert!(errors[0].message.contains(&format!("use `val {name}`")));
    }
}

#[test]
fn explicit_bindings_preserve_payloads_guards_captures_and_whole_values() {
    let source = r#"
enum E { N(Int), S(String) }
struct Box(val payload: E, val number: Int)
fun consume(value: E): Int = 10
fun whole(value: E): Int = when (value) {
    case S(val text) if text == "hello" -> 1
    case val NotBad -> consume(NotBad)
}
fun nested(value: Box): Int = when (value) {
    case Box { payload: N(val n), number: val count } -> {
        val read: () -> Int = { n + count }
        read()
    }
    case Box { val payload, number: _ } -> whole(payload)
}
fun main() { nested(Box(E.N(100), 2)) }
"#;
    let output = lower_user_output(scoop_parser::parse(source).unwrap()).unwrap();
    assert!(output.warnings.is_empty(), "{:?}", output.warnings);
}

#[test]
fn an_explicit_binding_can_share_the_subjects_variant_name() {
    let source = "enum E { N(Int), S(String) }\nfun identity(value: E): E = when(value) { case val N -> N }\nfun main() { identity(E.N(100)) }";
    let output = lower_user_output(scoop_parser::parse(source).unwrap()).unwrap();
    assert_eq!(output.warnings.len(), 1, "{:?}", output.warnings);
    let span = output.warnings[0].span.unwrap();
    assert_eq!(&source[span.start as usize..span.end as usize], "N");
}

#[test]
fn a_guarded_whole_value_binding_does_not_prove_exhaustiveness() {
    let source = "enum E { N(Int), S(String) }\nfun main() { when(E.N(100)) { case val value if false -> {} } }";
    let errors = lower_user_output(scoop_parser::parse(source).unwrap()).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|error| error.message.contains("non-exhaustive")),
        "{errors:?}"
    );
}
