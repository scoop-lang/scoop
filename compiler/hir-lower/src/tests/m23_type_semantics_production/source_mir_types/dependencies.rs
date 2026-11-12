use super::*;

// Import representations from the same core fixture that supplied the HIR
// declarations, including types used by implicit coroutine helpers.
pub(super) fn core(input: &scoop_mir::ConeMirInput) -> CanonicalParamFreeMirTypeExportsV1 {
    static TYPES: std::sync::OnceLock<CanonicalParamFreeMirTypeExportsV1> =
        std::sync::OnceLock::new();
    let types = TYPES.get_or_init(core_type_exports);
    let records = input
        .module()
        .meta
        .source_exact_types
        .iter()
        .filter(|identity| {
            identity.owner() == scoop_mir::SourceExactTypeOwner::Cone(ConeIdentity::CORE)
                && matches!(
                    identity.identity_record().key(),
                    scoop_identity::ExactTypeKey::Nominal(_)
                )
        })
        .map(|identity| {
            types
                .get(identity.identity_record().id())
                .unwrap_or_else(|| panic!("core fixture must export {:?}", identity.ty()))
                .clone()
        })
        .collect();
    CanonicalParamFreeMirTypeExportsV1::try_new(records).unwrap()
}

fn core_type_exports() -> CanonicalParamFreeMirTypeExportsV1 {
    let core = trusted_core();
    let input = lower_mir_input(&core.output);
    let hir: hir::DecodedHirFoundation = decoded(core.source_foundation.as_ref());
    let mir: scoop_mir::DecodedMirFoundation = decoded(input.foundation());
    let mut pending = PendingIdentityValidation::new();
    pending.register_authority(ConeIdentity::CORE).unwrap();
    hir.register_identities(&mut pending).unwrap();
    mir.register_identities(&mut pending).unwrap();
    hir.resolve_identities(&mut pending).unwrap();
    mir.resolve_identities(&mut pending).unwrap();
    let graph = pending.finish().unwrap();
    let source = crate::produce_cross_cone_type_semantics(
        &core.output,
        hir::SharedTypeMetadataV1 {
            provider: ConeIdentity::CORE,
            identities: &graph,
            foundation: &core.source_foundation,
            public: core.general_interface(),
        },
        &[],
    )
    .unwrap();
    scoop_mir_lower::lower_type_exports(
        core.output.output().local.module(),
        &source,
        &input,
        &graph,
    )
    .unwrap()
}
