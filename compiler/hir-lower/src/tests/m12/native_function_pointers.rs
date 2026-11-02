use super::*;

#[test]
fn native_function_pointer_calls_keep_static_mode_in_no_gc_code() {
    let source = r#"
        @Unsafe
        fun safe(pointer: FunPtr<(Int) -> Long>): Long = pointer(1)
        @NoGC @Unsafe
        fun leaf(pointer: GCLeafFunPtr<(Int) -> Long>): Long = pointer.invoke(2)
        fun main() {}
    "#;
    let module = lower_user(scoop_parser::parse(source).unwrap()).unwrap();
    let dump = hir::dump(&module);
    for mode in ["NativeSafe", "GcLeaf"] {
        assert!(
            dump.contains(&format!("NativePointerCall {mode}")),
            "{dump}"
        );
    }
}

#[test]
fn native_function_pointer_calls_require_unsafe_and_exact_positional_arguments() {
    for (body, expected) in [
        (
            "p(1)",
            "calling a native function pointer requires an unsafe context",
        ),
        (
            "p.invoke(1)",
            "calling a native function pointer requires an unsafe context",
        ),
        ("@Unsafe { p(value = 1) }", "do not accept named arguments"),
        ("@Unsafe { p(*[1]) }", "do not accept spread arguments"),
        ("@Unsafe { p() }", "takes exactly 1 argument(s)"),
        (
            "@Unsafe { val value: Long = 1L; p(value) }",
            "function argument must be of type Int, found Long",
        ),
        (
            "@Unsafe { p<Int>(1) }",
            "do not accept explicit type arguments",
        ),
    ] {
        for family in ["FunPtr", "GCLeafFunPtr"] {
            let source =
                format!("fun reject(p: {family}<(Int) -> Unit>) {{ {body} }} fun main() {{}}");
            let errors = messages(scoop_parser::parse(&source).unwrap().declarations);
            assert!(
                errors.iter().any(|message| message.contains(expected)),
                "{source}: {errors:#?}"
            );
        }
    }
}

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
fn no_gc_rejects_native_transitions_before_artifact_production() {
    let source = r#"
        @Extern(name = "native_safe") fun nativeSafe(): Int
        @NoGC @Extern(abi = "scoop", name = "native_borrowed") fun borrowed(): Int
        @NoGC @Unsafe fun pointer(p: FunPtr<() -> Int>): Int = p()
        @NoGC @Unsafe fun direct(): Int = nativeSafe()
        @NoGC fun native(): Int = borrowed()
        fun main() {}
    "#;
    let errors = messages(scoop_parser::parse(source).unwrap().declarations);
    assert_eq!(
        errors
            .iter()
            .filter(|message| message.contains("state transition"))
            .count(),
        2,
        "{errors:#?}"
    );
    assert!(
        errors
            .iter()
            .any(|message| message.contains("NativeSafe function pointer calls")),
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
