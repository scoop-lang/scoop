use super::*;

#[test]
fn codec_requirements_need_an_implementation() {
    let errors = lower_with_sysroot(
        r#"struct Row(val value: Int) {
            companion object : ValueEncoder<Row>, ValueDecoder<Row>
        }
        fun main() {}"#,
    )
    .unwrap_err();
    for method in ["ValueEncoder<Row>.encode", "ValueDecoder<Row>.decode"] {
        assert!(
            errors.iter().any(|error| error.message.contains(method)),
            "{errors:?}"
        );
    }
}

#[test]
fn ordinary_annotation_binding_is_independent_of_codec_generation() {
    lower_with_sysroot(
        r#"struct Unused(@Serialize(ignore = true, required = true) val value: Int)
        fun main() {}"#,
    )
    .unwrap();
}

#[test]
fn inspection_keeps_inherited_conditional_requirements() {
    let output = lower_with_sysroot(r#"
        struct Row<T>(val value: T)
        abstract class Base<T> {
            override interface ValueInspector<Row<T>> where T.Companion : ValueInspector<T> {}
        }
        class Derived<T> : Base<T>()
        fun <T> view(row: Row<T>): ValueView where T.Companion : ValueInspector<T> = Derived<T>().inspect(row)
        fun main() {}
    "#).unwrap();
    let method = output
        .export
        .functions
        .values()
        .find(|f| f.name == "Derived.inspect")
        .unwrap();
    let metadata = method.method.unwrap();
    assert_ne!(metadata.availability.unwrap().owner, metadata.owner);
    assert!(matches!(&method.kind, hir::FunctionKind::User(body) if !body.statements.is_empty()));
}
