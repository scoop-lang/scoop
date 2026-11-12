use super::*;

const EQUALITY: &str = r#"
fun <T : Equals<T>> compare(expected: T, actual: T) {}
fun <T : !Equals<T>> compare(expected: T, actual: T)
    where requires(x: T, y: T) { x.equals(y): Boolean } {}
fun optional(value: String?): String? = value
"#;

#[test]
fn nested_argument_errors_survive_earlier_outer_bound_failures() {
    for call in [
        "compare(Some(\"x\"), optional(\"bad\"))",
        "compare(optional(\"bad\"), Some(\"x\"))",
        "compare(Some(Some(\"x\")), Some(optional(\"bad\")))",
    ] {
        let source = format!("{EQUALITY}\nfun main() {{ {call} }}");
        let errors =
            lower(&[complete_core_file(), scoop_parser::parse(&source).unwrap()]).unwrap_err();
        assert_eq!(errors.len(), 1, "{errors:#?}");
        let error = &errors[0];
        assert!(error.message.contains("optional"), "{error:#?}");
        assert!(error.message.contains("Option<String>"), "{error:#?}");
        assert!(!error.message.contains("compare"), "{error:#?}");
        assert!(!error.message.contains("Equals"), "{error:#?}");
        let start = source.find("\"bad\"").unwrap() as u32;
        assert_eq!(error.span, Some(Span::new(start, start + 5)));
    }
}

#[test]
fn contextual_recovery_and_valid_structural_equality_remain_available() {
    let source = format!(
        r#"{EQUALITY}
fun <T> absent(): T? = None
fun main() {{
    compare(Some("x"), optional(Some("x")))
    compare(Some("x"), absent())
}}
"#
    );
    lower(&[complete_core_file(), scoop_parser::parse(&source).unwrap()]).unwrap();
}

#[test]
fn unrelated_constraint_errors_are_not_suppressed_by_nested_failures() {
    let source = format!(
        r#"{EQUALITY}
fun <T : Equals<T>> nominal(value: T) {{}}
fun nested() {{ compare(Some("x"), optional("bad")) }}
fun unrelated() {{ nominal(Some("x")) }}
"#
    );
    let errors = lower(&[complete_core_file(), scoop_parser::parse(&source).unwrap()]).unwrap_err();
    assert_eq!(errors.len(), 2, "{errors:#?}");
    assert!(
        errors
            .iter()
            .any(|error| error.message.contains("optional"))
    );
    assert!(
        errors
            .iter()
            .any(|error| { error.message.contains("nominal") && error.message.contains("Equals") })
    );
}
