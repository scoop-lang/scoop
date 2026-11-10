use super::*;

#[cfg(windows)]
mod windows;

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn generated_c_batch_preserves_units_and_exact_unwind_boundaries() {
    use object::{Object, ObjectSection, ObjectSymbol};
    let target = scoop_toolchain::ResolvedTargetProfile::resolve("aarch64-apple-darwin").unwrap();
    let input = scoop_lir::ConeLirOutput::try_new(bridge_module(2), Vec::new()).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let invocation = target.c_bridge_toolchain();
    let mut compilations = 0;
    let emitted = emit_c_bridge_object_set(
        &input,
        directory.path(),
        invocation.profile(),
        |source, object| {
            compilations += 1;
            invocation
                .compile_object(source, object)
                .map_err(|error| CodegenError(error.to_string()))?;
            assert_eq!(
                std::fs::read_dir(source.parent().unwrap()).unwrap().count(),
                2
            );
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(compilations, 1);
    assert_eq!(emitted.members.len(), 1);
    assert_eq!(emitted.members[0].units.len(), 2);
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
    let surface = scoop_lir::ObjectSymbolSurfaceV1::from_foundation(input.foundation()).unwrap();
    let file = object::File::parse(emitted.members[0].bytes.as_slice()).unwrap();
    let mut ranges = Vec::new();
    for unit in &emitted.members[0].units {
        let plan = surface
            .plans()
            .iter()
            .find(|plan| {
                plan.owner().kind()
                    == scoop_lir::StrongDefinitionEntityKind::GeneratedBridgeAtom(
                        unit.primary_atom(),
                    )
            })
            .unwrap();
        for boundary in plan.atom_boundaries() {
            let symbol = |request: scoop_lir::PersistentSymbolRequest| {
                file.symbol_by_name(&format!("_{}", request.symbol()))
                    .unwrap()
            };
            let start = symbol(boundary.start());
            let end = symbol(boundary.end());
            assert_eq!(start.section_index(), end.section_index());
            assert!(end.address() > start.address());
            let section = file
                .section_by_index(start.section_index().unwrap())
                .unwrap();
            assert!(end.address() <= section.address() + section.size());
            ranges.push((section.index().0, start.address(), end.address()));
        }
    }
    ranges.sort_unstable();
    for pair in ranges.windows(2) {
        assert!(pair[0].0 != pair[1].0 || pair[0].2 <= pair[1].1);
    }
}

#[test]
fn empty_bridge_plan_neither_compiles_nor_creates_a_directory() {
    let input = scoop_lir::ConeLirOutput::try_new(values_module(), Vec::new()).unwrap();
    let parent = tempfile::tempdir().unwrap();
    let scratch = parent.path().join("unused");
    let emitted = emit_c_bridge_object_set(
        &input,
        &scratch,
        &generated_c_profile(),
        |_source, _object| panic!("empty bridge plan must not compile"),
    )
    .unwrap();
    assert!(emitted.members.is_empty());
    assert_eq!(
        emitted.production,
        scoop_lir::CBridgeProductionSetV1::NotUsed
    );
    assert!(!scratch.exists());
}

#[test]
fn failed_bridge_batch_discards_partial_compiler_output() {
    let input = scoop_lir::ConeLirOutput::try_new(bridge_module(2), Vec::new()).unwrap();
    let parent = tempfile::tempdir().unwrap();
    let error = emit_c_bridge_object_set(
        &input,
        parent.path(),
        &generated_c_profile(),
        |_source, object| {
            std::fs::write(object, b"partial compiler output").unwrap();
            Err(CodegenError("test compiler failure".into()))
        },
    )
    .unwrap_err();
    assert!(error.0.contains("test compiler failure"), "{error}");
    assert_eq!(std::fs::read_dir(parent.path()).unwrap().count(), 0);
}

fn bridge_module(count: u8) -> Module {
    bridge_module_for_target(count, scoop_target_data::DARWIN_AARCH64)
}

fn bridge_module_for_target(count: u8, target: scoop_lir::LirTargetProfile) -> Module {
    let mut module = values_module();
    module.meta = string_metadata_for(target);
    for seed in 1..=count {
        let contract = native_function_contract_for_target(
            seed,
            c_signature(seed).signature().clone(),
            target,
        );
        let entry = outbound_bridge_for_contract(&contract);
        install_test_native_contract(&mut module, contract);
        module.extern_functions.alloc_c(scoop_lir::CExternFunction {
            call_mode: scoop_identity::CAbiCallMode::NativeSafe,
            identity: scoop_lir::ExternFunctionIdentity {
                source_name: format!("bridge{seed}"),
                native_symbol: format!("test_bridge_{seed}"),
                library: "fixture".to_owned(),
                calling_convention: scoop_lir::CallingConvention::Cdecl,
            },
            call_plan: scoop_lir::CAbiCallPlan::StorageBridge {
                entry: Box::new(entry),
                result: scoop_identity::CResultAdaptation::Direct,
            },
            signature: scoop_lir::CFunctionType {
                params: vec![scoop_lir::CType::Integer(IntegerKind::SIGNED_32)],
                return_type: scoop_lir::CReturnType::Void,
            },
        });
    }
    module
}

fn generated_c_profile() -> scoop_lir::CBridgeToolchainProfileV1 {
    let deployment = scoop_lir::DarwinCBridgeDeploymentContractV1::new(
        scoop_lir::DarwinPackedVersionV1::from_components(15, 0, 0).unwrap(),
        scoop_lir::DarwinPackedVersionV1::from_components(26, 0, 0).unwrap(),
        Vec::new(),
    )
    .unwrap();
    let compiler = scoop_lir::AppleClangCompilerIdentityV1::new(21, 0, 0, "test-clang").unwrap();
    scoop_target_data::darwin_c_bridge_profile(
        scoop_target_data::DARWIN_AARCH64,
        deployment,
        compiler,
    )
    .unwrap()
}
