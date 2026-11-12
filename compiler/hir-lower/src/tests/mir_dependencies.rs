use std::collections::BTreeMap;

use scoop_hir::{DependencyHirOutput, ImportedCoreProtocolCallableDefinition, concrete};
use scoop_identity::{
    DependencyCallableDeclarationId, ExactCallableSignature, ExactTypeKey, OptionalSignatureType,
    PersistentExactTypeId, SignatureTypeKey, StrongCallableDefinitionOwner,
};
use scoop_mir::{SelectedDependencyMirCallableV1, SelectedExternalMirSet};

/// Supplies external callables for library-HIR tests that enter MIR lowering.
/// The test input retains the actual imported definitions and source signatures.
pub(super) fn select_library_callables(output: &DependencyHirOutput) -> SelectedExternalMirSet {
    let module = output.output().local.module();
    let mut records = BTreeMap::new();
    for use_ in output.executable_dependency_callables().unwrap() {
        let selected = use_.callable();
        let callable = selected.capability();
        records.insert(
            (selected.provider(), callable.implementation()),
            SelectedDependencyMirCallableV1::try_new(
                selected.provider(),
                callable.direct_declaration().unwrap(),
                callable.implementation(),
                callable.signature().clone(),
            )
            .unwrap(),
        );
    }
    let protocols = module
        .initialization_units
        .values()
        .filter_map(|unit| match &unit.cycle_thrower {
            concrete::InitializationCycleThrower::Imported(callable) => Some(callable),
            concrete::InitializationCycleThrower::Local(_) => None,
        })
        .chain(module.imported_coroutine_drive_functions());
    for callable in protocols {
        let ImportedCoreProtocolCallableDefinition::Function(definition) = callable.definition()
        else {
            panic!("library initialization and coroutine drivers use ordinary core functions")
        };
        let definition = definition.persistent();
        let target = StrongCallableDefinitionOwner::Function(definition);
        records
            .entry((callable.provider(), target))
            .or_insert_with(|| {
                let signature = callable.signature();
                let receiver = match signature.receiver() {
                    OptionalSignatureType::Absent => None,
                    OptionalSignatureType::Present(ty) => Some(nominal_exact(ty)),
                };
                SelectedDependencyMirCallableV1::try_new(
                    callable.provider(),
                    DependencyCallableDeclarationId::Function(definition),
                    target,
                    ExactCallableSignature::new(
                        signature.effect(),
                        receiver,
                        signature.parameters().iter().map(nominal_exact).collect(),
                        nominal_exact(signature.result()),
                    ),
                )
                .unwrap()
            });
    }
    SelectedExternalMirSet::try_from_callables(module.cone, records.into_values().collect())
        .unwrap()
}

fn nominal_exact(ty: &SignatureTypeKey) -> PersistentExactTypeId {
    let SignatureTypeKey::Nominal(nominal) = ty else {
        panic!("library initialization and coroutine driver signatures use nominal types")
    };
    PersistentExactTypeId::from_key(&ExactTypeKey::Nominal(*nominal)).unwrap()
}
