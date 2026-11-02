use super::*;

#[test]
fn imported_native_calls_check_the_boundary_in_no_gc_bodies() {
    let provider = r#"
        @Extern(name = "native_safe") public fun nativeSafe(): Int
        @NoGC @Extern(abi = "scoop", name = "native_borrowed") public fun borrowed(): Int
        @GCLeaf @Extern(name = "native_leaf") public fun leaf(): Int
    "#;
    with_provider_consumer(
        provider,
        "@NoGC @Unsafe public fun good(): Int = leaf()",
        |_, _, _, _, _| {},
    )
    .unwrap();
    let errors = with_provider_consumer(
        provider,
        r#"
        @NoGC @Unsafe public fun badSafe(): Int = nativeSafe()
        @NoGC public fun badBorrowed(): Int = borrowed()
    "#,
        |_, _, _, _, _| {},
    )
    .unwrap_err();
    assert_eq!(errors.len(), 2, "{errors:#?}");
    assert!(
        errors
            .iter()
            .all(|error| error.message.contains("state transition")),
        "{errors:#?}"
    );
}

#[test]
fn ordinary_dependency_calls_share_contextual_arguments() {
    check_fixture("context");
}

#[test]
fn ordinary_dependency_calls_preserve_callable_variance() {
    check_fixture("variance");
}

fn check_fixture(name: &str) {
    let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/language/shared-native-calls");
    let provider = std::fs::read_to_string(fixtures.join("provider.scoop")).unwrap();
    let consumer = std::fs::read_to_string(fixtures.join(format!("{name}.scoop"))).unwrap();
    with_provider_consumer(&provider, &consumer, |_, _, _, _, _| {})
        .unwrap_or_else(|errors| panic!("{name}: {errors:?}"));
}
