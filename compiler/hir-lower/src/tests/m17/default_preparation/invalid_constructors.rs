use super::*;

#[test]
fn invalid_constructor_parameters_keep_the_original_diagnostic() {
    for declaration in [
        "class Broken(val value: Int = 1, val missing: Missing)",
        "struct Broken(val value: Int = 1, val missing: Missing)",
        "class Broken { constructor(value: Int = 1, missing: Missing) {} }",
        "struct Broken { constructor(value: Int = 1, missing: Missing) {} }",
    ] {
        let source = format!("{declaration}\nfun main() {{ Broken() }}");
        let errors = lower_source(&source).unwrap_err();
        assert!(
            errors.iter().any(|error| error.message.contains("Missing")),
            "{source}\n{errors:#?}"
        );
        assert!(
            errors
                .iter()
                .all(|error| !error.message.contains("default")),
            "{source}\n{errors:#?}"
        );
    }
}
