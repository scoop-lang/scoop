use super::*;

pub(super) fn check(input: scoop_mir_lower::MirTypeBridgeExportInputV1<'_>) {
    let local = &input.hir.output().local;
    let string = input
        .mir
        .module()
        .meta
        .source_exact_types
        .get(&mir::Type::String)
        .unwrap();
    assert_eq!(
        string.identity_record(),
        &local.exact_type_identities[local.string]
    );
    assert!(
        !local
            .functions
            .iter()
            .any(|(_, function)| function.name == "unexpanded")
    );
}
