use super::*;
use hir::concrete::{ExprKind, TypeKind};
use source_dispatch::with_hir_source;

mod resources;
mod type_operands;

fn fixture(name: &str) -> String {
    std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/language/materialized-type-uses")
            .join(format!("{name}.scoop")),
    )
    .unwrap()
}

fn names(output: &hir::LocalConcreteHirOutput) -> BTreeSet<String> {
    let module = output.module();
    output
        .materialized_type_closure()
        .unwrap()
        .iter()
        .filter_map(|ty| match &module.types[*ty].kind {
            TypeKind::Unit => Some("Unit".to_owned()),
            TypeKind::Boolean => Some("Boolean".to_owned()),
            TypeKind::String => Some("String".to_owned()),
            TypeKind::Any => Some("Any".to_owned()),
            TypeKind::Integer(kind) => Some(kind.canonical_name().to_owned()),
            TypeKind::Struct(id) => Some(module.structs[*id].name.clone()),
            TypeKind::Class(id) => Some(module.classes[*id].name.clone()),
            TypeKind::Enum(id) => Some(module.enums[*id].name.clone()),
            TypeKind::Interface(id) => Some(module.interfaces[*id].name.clone()),
            TypeKind::Tuple(_)
            | TypeKind::Function(_)
            | TypeKind::Ptr(_)
            | TypeKind::FunPtr(_)
            | TypeKind::GcLeafFunPtr(_) => None,
        })
        .collect()
}

#[test]
fn materialized_types_include_parents_and_exclude_unevaluated_defaults() {
    let declaration = fixture("standalone");
    for uses_default in [false, true] {
        let source = if uses_default {
            format!("{declaration}\nfun useDefault(): Boolean = flag()\n")
        } else {
            declaration.clone()
        };
        with_hir_source(&source, |output, _| {
            let local = &output.output().local;
            let materialized = names(local);
            for required in ["Deferred", "SourceOnly", "ParentPayload", "Long"] {
                assert!(
                    materialized.contains(required),
                    "parent closure must retain {required}"
                );
            }
            assert_eq!(
                materialized.contains("DefaultOnly"),
                uses_default,
                "a type used only by the default follows the actual omitted-argument call"
            );
            assert_eq!(
                local
                    .functions
                    .iter()
                    .any(|(_, function)| function.name == "unexpanded"),
                uses_default,
                "the generic default body is instantiated only when used"
            );
        });
    }
}

#[test]
fn materialized_types_include_shapes_signatures_captures_and_type_tests() {
    with_hir_source(&fixture("combined"), |output, _| {
        let local = &output.output().local;
        let used = names(local);
        assert_eq!(
            used,
            BTreeSet::from(
                [
                    "Any", "Array", "Boolean", "Cell", "Envelope", "Packet", "Reader", "String",
                    "UInt8", "Unit"
                ]
                .map(str::to_owned)
            )
        );
        assert!(!local.lambdas.is_empty());
        assert!(!local.class_constructors.is_empty());
    });
}

#[test]
fn materialized_types_include_explicit_non_result_type_operands() {
    with_hir_source(&fixture("type-operands"), |output, _| {
        let local = type_operands::with_layout_operations(&output.output().local);
        assert_eq!(
            names(&local),
            BTreeSet::from(
                [
                    "Any", "Array", "Boolean", "Int8", "UInt8", "ULong", "UInt16"
                ]
                .map(str::to_owned)
            )
        );
        let mut operations = [false; 3];
        local
            .visit_executable_expressions(|occurrence| {
                match occurrence.expression.kind {
                    ExprKind::SizeOf(_) => operations[0] = true,
                    ExprKind::AlignOf(_) => operations[1] = true,
                    ExprKind::IsInstance { .. } => operations[2] = true,
                    _ => {}
                }
                Ok::<_, std::convert::Infallible>(())
            })
            .unwrap();
        assert_eq!(operations, [true; 3]);
    });
}

#[test]
fn materialized_initialization_adds_its_implicit_string_type() {
    with_hir_source(&fixture("initialization"), |output, _| {
        let local = &output.output().local;
        assert_eq!(local.initialization_units.len(), 1);
        local
            .visit_executable_expressions(|occurrence| {
                assert!(!matches!(
                    occurrence.expression.kind,
                    ExprKind::StringLiteral { .. }
                ));
                Ok::<_, std::convert::Infallible>(())
            })
            .unwrap();
        assert_eq!(
            names(local),
            BTreeSet::from(["Array", "Boolean", "String", "UInt8", "Unit"].map(str::to_owned))
        );
    });
}
