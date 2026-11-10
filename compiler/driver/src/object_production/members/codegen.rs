//! Consume the in-memory Scoop and generated-C objects.

use super::*;

pub(in crate::object_production) fn plan_codegen_objects(
    target: LirTargetProfile,
    producer_units: &ProducerUnitPartitionV1,
    members: Vec<scoop_codegen::EmittedConeObjectMemberV1>,
    generated_c_bridge: Vec<scoop_codegen::EmittedGeneratedCBridgeObjectMemberV1>,
) -> Result<PlannedObjectBindings, BuiltinObjectProductionError> {
    let mut scoop_lir_sources = Vec::with_capacity(members.len());
    for member in members {
        let scoop_codegen::EmittedConeObjectMemberV1 { units, bytes, .. } = member;
        scoop_lir_sources.push(UnboundScoopLirObject {
            units: units.definition_plans().to_vec(),
            bytes,
        });
    }

    let generated_c_bridge_sources = generated_c_bridge
        .into_iter()
        .map(|member| UnboundGeneratedCBridgeObject {
            units: member
                .units
                .iter()
                .map(scoop_lir::GeneratedBridgeUnitPlanV1::unit)
                .collect(),
            bytes: member.bytes,
        })
        .collect();

    plan_objects(
        target,
        producer_units,
        scoop_lir_sources,
        generated_c_bridge_sources,
    )
}
