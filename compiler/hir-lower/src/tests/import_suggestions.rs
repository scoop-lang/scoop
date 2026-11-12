use super::*;

fn compile(declarations: &[&str], consumer: &str) -> Result<hir::Output, Vec<ast::Diagnostic>> {
    let core = complete_core_file();
    let parsed = identified_test_sources(
        declarations
            .iter()
            .chain(std::iter::once(&consumer))
            .map(|text| scoop_parser::parse(text).unwrap())
            .collect(),
    );
    let input = DefinedTestSources::try_new(
        vec![ProviderSource {
            source: &core,
            identity: core_source_identity("src/core.scoop"),
            provider: hir::IntrinsicProviderId::from_raw(0),
            name: "<core>",
            source_text: "",
        }],
        hir::IntrinsicProviderId::from_raw(1),
        parsed,
        |_| CurrentSourceDetails {
            display_locator: "<user>",
            source_text: "",
        },
    )
    .unwrap();
    lower_defined_for_test(
        scoop_identity::RequestedConeKind::Library,
        &input,
        IntrinsicDeclarationPolicy::CoreOnly,
    )
}

fn diagnose(declarations: &[&str], consumer: &str) -> Vec<ast::Diagnostic> {
    compile(declarations, consumer).unwrap_err()
}

fn paths(diagnostic: &ast::Diagnostic) -> Vec<Vec<&str>> {
    diagnostic
        .notes
        .iter()
        .filter(|note| !note.import_path.is_empty())
        .map(|note| note.import_path.iter().map(String::as_str).collect())
        .collect()
}

#[test]
fn missing_names_offer_visible_paths_and_preserve_ambiguous_choices() {
    let declarations = [
        "package left\npublic fun pick(): Int = 1",
        "package right\npublic fun pick(): Int = 2",
        "package hidden\nprivate fun pick(): Int = 3",
    ];
    let errors = diagnose(&declarations, "package app\nfun main() { pick() }");
    assert_eq!(errors.len(), 1, "{errors:#?}");
    assert_eq!(
        paths(&errors[0]),
        vec![vec!["left", "pick"], vec!["right", "pick"]]
    );
    for package in ["left", "right"] {
        compile(
            &declarations,
            &format!("package app\nimport {package}.pick\nfun main() {{ pick() }}"),
        )
        .unwrap();
    }
}

#[test]
fn extension_suggestions_check_receiver_types_and_structural_requirements() {
    let declarations = [r#"package tools
public fun <T> T.inspect(): Int where requires(x: T) { x.read(): Int } = 1
public val String.mark: Int get() = 7
"#];
    compile(&declarations, "package app\nimport tools.inspect\nstruct Good() { public fun read(): Int = 1 }\nfun good(value: Good) { value.inspect() }").unwrap();
    let errors = diagnose(
        &declarations,
        r#"package app
struct Good() { public fun read(): Int = 1 }
fun good(value: Good) { value.inspect() }
fun bad(value: Long) { value.inspect() }
fun text() { "hi".mark }
"#,
    );
    assert_eq!(errors.len(), 3, "{errors:#?}");
    assert_eq!(
        errors
            .iter()
            .filter(|error| paths(error) == vec![vec!["tools", "inspect"]])
            .count(),
        1,
        "{errors:#?}"
    );
    assert_eq!(
        errors
            .iter()
            .filter(|error| error.message.contains("inspect") && paths(error).is_empty())
            .count(),
        1,
        "{errors:#?}"
    );
    assert!(
        errors
            .iter()
            .any(|error| paths(error) == vec![vec!["tools", "mark"]]),
        "{errors:#?}"
    );
}

#[test]
fn missing_type_and_value_have_different_candidate_namespaces() {
    let declarations = [
        "package tools\npublic struct Record(val value: Int)\npublic val missing: Int = 1\nprivate struct Secret()",
    ];
    let errors = diagnose(
        &declarations,
        "package app\nfun value(): Int = missing\nfun type(value: Record): Int = 1\nfun hidden(value: Secret): Int = 2",
    );
    assert!(
        errors
            .iter()
            .any(|error| paths(error) == vec![vec!["tools", "Record"]]),
        "{errors:#?}"
    );
    assert!(
        errors
            .iter()
            .filter(|error| error.message.contains("Secret"))
            .all(|error| paths(error).is_empty()),
        "{errors:#?}"
    );
    let errors = diagnose(&declarations, "package app\nfun main() { missing }");
    assert_eq!(paths(&errors[0]), vec![vec!["tools", "missing"]]);
}

#[test]
fn an_existing_exact_import_is_not_repaired_with_a_conflicting_import() {
    let declarations = [
        "package first\npublic fun Long.inspect(): Int = 1",
        "package second\npublic fun String.inspect(): Int = 2",
    ];
    let errors = diagnose(
        &declarations,
        "package app\nimport first.inspect\nfun main() { \"text\".inspect() }",
    );
    assert!(!errors.is_empty());
    assert!(
        errors.iter().all(|error| paths(error).is_empty()),
        "{errors:#?}"
    );
}
