use super::*;

pub(crate) fn prepare(
    emitted: EmittedConeObjectSetV2,
    generated: EmittedGeneratedCBridgeObjectSetV1,
) -> Result<PreparedLayoutObjects, BuiltinObjectProductionError> {
    let EmittedConeObjectSetV2 {
        target_selection,
        foundation,
        production,
        partition,
        members,
        ..
    } = emitted;
    let mut bindings = plan_codegen_objects(
        target_selection.target(),
        partition.producer_units(),
        members,
        generated.members,
    )?;
    let candidates = bindings
        .scoop_lir_members
        .iter()
        .map(|member| ScoopLirObjectCandidateV1::new(member.plan.member_id(), &member.bytes))
        .collect::<Vec<_>>();
    let bridges = bindings
        .generated_c_bridge_members
        .iter()
        .map(|member| {
            GeneratedCBridgeObjectCandidateV1::new(member.plan.member_id(), &member.bytes)
        })
        .collect::<Vec<_>>();
    let bridge_objects =
        read_generated_c_objects_v1(generated.production, &generated.profile, &bridges)
            .map_err(BuiltinObjectProductionError::CBridgeEnvelopes)?;
    let symbols = PlannedStrongObjectSymbolSetV1::new(
        target_selection.target(),
        production.canonical_definitions(),
        &bindings.member_plan,
    )
    .map_err(BuiltinObjectProductionError::StrongSymbolPlan)?;
    let relocations = verify_builtin_object_strong_relocations_v1(
        &bindings.member_plan,
        &symbols,
        &candidates,
        bridge_objects,
        &bridges,
    )
    .map_err(BuiltinObjectProductionError::StrongRelocations)?;
    let mut builtins = relocations;
    let mut stackmaps = read_scoop_stackmaps_v1(
        builtins.clone(),
        production.safepoint_semantics(),
        &candidates,
    )
    .map_err(BuiltinObjectProductionError::Stackmaps)?;
    drop(candidates);
    let mut records = std::collections::BTreeMap::<_, Vec<_>>::new();
    for record in stackmaps.records() {
        records.entry(record.member()).or_default().push(record);
    }
    if !records.is_empty() {
        for member in &mut bindings.scoop_lir_members {
            if let Some(sites) = records.get(&member.plan.member_id()) {
                member.bytes = slib::compact_callable_gc_object_v1(
                    target_selection.target(),
                    &member.bytes,
                    sites,
                )
                .map_err(BuiltinObjectProductionError::CompactGc)?;
            }
        }
        let changed = bindings
            .scoop_lir_members
            .iter()
            .filter(|member| records.contains_key(&member.plan.member_id()))
            .map(|member| ScoopLirObjectCandidateV1::new(member.plan.member_id(), &member.bytes))
            .collect::<Vec<_>>();
        builtins = builtins
            .after_gc_compaction(&symbols, &changed)
            .map_err(BuiltinObjectProductionError::StrongRelocations)?;
        drop(records);
        stackmaps.set_object_layout(builtins.clone());
    }
    Ok(PreparedLayoutObjects {
        target_selection,
        foundation,
        production,
        builtins,
        stackmaps,
        bindings,
    })
}
