use super::*;

#[test]
fn native_function_pointer_conversions_preserve_signature_and_no_gc_effect() {
    let source = r#"
        @NoGC
        fun widen(value: GCLeafFunPtr<(Int) -> Long>): FunPtr<(Int) -> Long> = value.asFunPtr()
        @NoGC
        @Unsafe
        fun narrow(value: FunPtr<(Int) -> Long>): GCLeafFunPtr<(Int) -> Long> = value.asGCLeaf()
        fun main() {}
    "#;
    let module = lower_user(scoop_parser::parse(source).unwrap()).unwrap();
    let dump = hir::dump(&module);
    for expected in [
        "NativeFunctionPointerCast NativeSafe : FunPtr<(Int) -> Long>",
        "NativeFunctionPointerCast GcLeaf : GCLeafFunPtr<(Int) -> Long>",
    ] {
        assert!(dump.contains(expected), "{dump}");
    }
}

#[test]
fn native_function_pointer_leaf_conversion_requires_unsafe_context() {
    let source = r#"
        fun reject(value: FunPtr<() -> Unit>): GCLeafFunPtr<() -> Unit> = value.asGCLeaf()
        fun main() {}
    "#;
    let errors = messages(scoop_parser::parse(source).unwrap().declarations);
    assert!(
        errors
            .iter()
            .any(|message| message.contains("asGCLeaf") && message.contains("unsafe")),
        "{errors:#?}"
    );
}

#[test]
fn native_function_pointer_conversion_references_follow_function_safety() {
    let source = r#"
        fun bound(value: GCLeafFunPtr<() -> Unit>): () -> FunPtr<() -> Unit> = value::asFunPtr
        fun unbound(): (GCLeafFunPtr<() -> Unit>) -> FunPtr<() -> Unit> =
            GCLeafFunPtr<() -> Unit>::asFunPtr
        fun main() {}
    "#;
    lower_user(scoop_parser::parse(source).unwrap()).unwrap();
    let source = r#"
        @Unsafe
        fun reject(value: FunPtr<() -> Unit>): () -> GCLeafFunPtr<() -> Unit> = value::asGCLeaf
        fun main() {}
    "#;
    let errors = messages(scoop_parser::parse(source).unwrap().declarations);
    assert!(
        errors
            .iter()
            .any(|message| message.contains("unsafe functions cannot be stored")),
        "{errors:#?}"
    );
}
