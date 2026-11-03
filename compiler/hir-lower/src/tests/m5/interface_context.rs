use super::*;

#[test]
fn array_literals_infer_from_actual_interface_supertypes() {
    let output = lower_with_sysroot(
        r#"
fun <T> emptyValues(): List<T> = []
fun <T> seeded(values: List<T>, seed: T): List<T> = values
fun main() {
    val direct: List<Int> = []
    val inherited: Collection<String> = []
    val iterable: Iterable<Long> = [1, 2]
    val mutable: IndexSet<Long, Int> = [1]
    val nested: List<List<Int>> = [[], [1]]
    val generic = seeded([], 1)
    val rigid: List<String> = emptyValues()
}
"#,
    )
    .expect("array interfaces provide a concrete element context");
    let module = &output.export;
    let hir::FunctionKind::User(body) = &module.functions[module.entry()].kind else {
        panic!("main is a user body")
    };
    for (name, expected) in [
        ("direct", "Array<Int>"),
        ("inherited", "Array<String>"),
        ("iterable", "Array<Long>"),
        ("mutable", "MutableArray<Int>"),
        ("nested", "Array<List<Int>>"),
    ] {
        let hir::ExprKind::Upcast(literal) = &local_init(body, name).kind else {
            panic!("{name} must upcast an actual array")
        };
        assert!(matches!(literal.kind, hir::ExprKind::ArrayLiteral(_)));
        assert_eq!(hir::type_name(module, literal.ty), expected);
    }
}

#[test]
fn array_interface_context_keeps_value_boxing_explicit() {
    let errors = lower_with_sysroot("fun main() { val values: List<Any> = [1] }")
        .expect_err("interface context cannot implicitly box array elements");
    assert!(
        errors
            .iter()
            .any(|error| error.message == "array literal element must be of type Any, found Int")
    );
    lower_with_sysroot("fun main() { val values: List<Any> = [1 as Any] }")
        .expect("an explicitly boxed element is valid");
}

#[test]
fn unrelated_interface_and_any_do_not_infer_an_array_element() {
    for ty in ["MutableList<Int>", "Iterator<Int>", "Any"] {
        let errors = lower_with_sysroot(&format!("fun main() {{ val values: {ty} = [] }}"))
            .expect_err("only an implemented interface can supply an array context");
        assert!(errors.iter().any(
            |error| error.message == "cannot infer the element type of an empty array literal"
        ));
    }
}
