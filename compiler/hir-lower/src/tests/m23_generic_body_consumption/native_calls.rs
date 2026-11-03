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

#[test]
fn imported_callback_registration_retains_implicit_protocol_types() {
    with_provider_consumer(
        r#"
        public fun <T> register(value: T): ForeignCallback<(Ptr<Unit>) -> Int> {
            @Unsafe {
                return foreignCallback<(Ptr<Unit>) -> Int>(
                    { val captured = value; 42 }, 0, ForeignCallbackMode.OneShot
                )
            }
        }
        "#,
        "public fun use(): ForeignCallback<(Ptr<Unit>) -> Int> = register(42)",
        |output, _, _, _, _| {
            let local = output.output().local.module();
            assert_eq!(local.foreign_callback_registrations.len(), 1);
            for name in ["ForeignCallbackMode", "ForeignCallbackState"] {
                assert!(
                    local
                        .enums
                        .iter()
                        .any(|(_, enumeration)| enumeration.name == name)
                );
            }
        },
    )
    .expect("an imported registration materializes its implicit protocol types");
}

#[test]
fn imported_callback_operation_retains_support_without_a_registration() {
    with_provider_consumer(
        r#"
        public fun <T> release(callback: ForeignCallback<(Ptr<Unit>) -> Int>, value: T) {
            @Unsafe { releaseForeignCallback(callback) }
        }
        "#,
        "public fun use(callback: ForeignCallback<(Ptr<Unit>) -> Int>) { release(callback, 42) }",
        |output, _, _, _, _| {
            let local = output.output().local.module();
            assert!(local.foreign_callback_registrations.is_empty());
            assert!(
                local
                    .enums
                    .iter()
                    .any(|(_, enumeration)| enumeration.name == "ForeignCallbackState")
            );
        },
    )
    .expect("an imported callback operation also retains its implicit protocol types");
}

fn check_fixture(name: &str) {
    let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/language/shared-native-calls");
    let provider = std::fs::read_to_string(fixtures.join("provider.scoop")).unwrap();
    let consumer = std::fs::read_to_string(fixtures.join(format!("{name}.scoop"))).unwrap();
    with_provider_consumer(&provider, &consumer, |_, _, _, _, _| {})
        .unwrap_or_else(|errors| panic!("{name}: {errors:?}"));
}
