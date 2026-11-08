use super::*;
use scoop_lir::{
    ImmortalObjectIdentity, MaterializationRoot, OptimizationMode, StrongDefinitionRole,
};

#[test]
fn immortal_members_define_only_their_own_object_and_registration() {
    for target in scoop_target_data::TARGETS {
        for mode in [OptimizationMode::Debug, OptimizationMode::Release] {
            let mut module = values_module();
            module.meta = string_metadata_for(target.selection.target());
            module.output = scoop_lir::LirOutput::Library;
            for ordinal in 0..2 {
                let value = format!("independent-odr-resource-{ordinal}");
                let identity = immortal_string_identity(&value);
                let identity = ImmortalObjectIdentity::from_key(
                    identity.identity_record().key().clone(),
                    MaterializationRoot::lir_structural_odr(test_exact_type("ImmortalOdr"))
                        .unwrap(),
                )
                .unwrap();
                module.globals.alloc(Global {
                    address_kind: PointerKind::Managed,
                    scan: RefScan::None,
                    init: GlobalInit::StringConst { identity, value },
                });
            }
            let object_count = module.globals.len();
            let input = scoop_lir::ConeLirOutput::try_new(module, Vec::new()).unwrap();
            let emitted = emit_object_set(
                &input,
                &scoop_lir::ConeCoordinate::reserved_single_file(),
                &[ConeIdentity::CORE],
                scoop_lir::EntryProductionSourceV1::Library,
                ValidatedBackendProfile::for_selection(target.selection)
                    .unwrap()
                    .with_optimization(mode),
                std::num::NonZeroUsize::MIN,
            )
            .unwrap();
            let surface = emitted.production.canonical_definitions();
            let plans = surface
                .plans()
                .iter()
                .filter(|plan| {
                    matches!(
                        plan.definition_role(),
                        StrongDefinitionRole::ImmortalObject
                            | StrongDefinitionRole::ImmortalRegistration
                    )
                })
                .collect::<Vec<_>>();
            assert_eq!(plans.len(), object_count * 2);
            let normalization = target
                .selection
                .target()
                .contract()
                .native_symbol_normalization();
            let mut defined = BTreeSet::new();
            let mut strong_members = 0;
            let mut odr_members = 0;
            for member in &emitted.members {
                let owned = plans
                    .iter()
                    .filter(|plan| {
                        member
                            .units
                            .definition_plans()
                            .contains(&plan.definition_plan())
                    })
                    .collect::<Vec<_>>();
                if let Some(first) = owned.first() {
                    assert_eq!(owned.len(), member.units.definition_plans().len());
                    if first.primary_symbol().linkage() == scoop_lir::LinkageClass::OdrWeak {
                        odr_members += 1;
                        assert_eq!(owned.len(), 1, "ODR member keys must remain independent");
                    } else {
                        strong_members += 1;
                        assert_eq!(owned.len(), 2);
                        assert_eq!(owned[0].owner(), owned[1].owner());
                        assert_eq!(
                            owned
                                .iter()
                                .map(|plan| plan.definition_role())
                                .collect::<BTreeSet<_>>(),
                            BTreeSet::from([
                                StrongDefinitionRole::ImmortalObject,
                                StrongDefinitionRole::ImmortalRegistration,
                            ])
                        );
                    }
                }
                let file = object::File::parse(member.bytes.as_slice()).unwrap();
                for plan in &plans {
                    let name = normalization
                        .compiler_generated_object_symbol(plan.primary_symbol().symbol().as_str());
                    let present = file
                        .symbol_by_name(&name)
                        .is_some_and(|symbol| symbol.is_definition());
                    let expected = member
                        .units
                        .definition_plans()
                        .contains(&plan.definition_plan());
                    assert_eq!(present, expected, "{}/{mode:?}: {name}", target.canonical());
                    if present {
                        assert!(
                            defined.insert(plan.definition_plan()),
                            "duplicate object definition"
                        );
                    }
                }
            }
            assert_eq!(defined.len(), plans.len());
            assert_eq!(strong_members, object_count - 2);
            assert_eq!(odr_members, 4);
        }
    }
}
