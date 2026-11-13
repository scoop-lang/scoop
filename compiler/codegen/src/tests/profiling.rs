use super::*;
use object::{Object, ObjectSection};
use scoop_profile::{BodyId, BodyKey, BodyLayout, Counter, Location, Position, PositionKind};

#[test]
fn profiling_counters_survive_production_object_emission() {
    for target in [
        scoop_target_data::DARWIN_AARCH64_ID,
        scoop_target_data::LINUX_X86_64_GNU_ID,
        scoop_target_data::WINDOWS_X86_64_MINGW_UCRT_ID,
    ] {
        let mut module = values_module();
        module.output = scoop_lir::LirOutput::Library;
        module.meta = string_metadata_for(scoop_target_data::layout(target).unwrap());
        for function in &mut module.functions {
            let key = BodyKey {
                body: BodyId::from_key(function.callable_body.identity_record().key()),
                fingerprint: scoop_wire::Digest256::from_array([7; 32]).into(),
            };
            module.meta.profile_bodies.push(BodyLayout {
                key,
                name: function.debug_name.clone(),
                positions: vec![Position {
                    kind: PositionKind::Entry,
                }],
            });
            function.blocks[function.entry].instructions.insert(
                0,
                Instruction::ProfileCounter(Counter {
                    location: Location {
                        body: key,
                        position: 0,
                    },
                    inlined_at: Vec::new(),
                }),
            );
        }
        let map = crate::profiling::profile_map(&module, "test-target").unwrap();
        assert_eq!(map.records.len(), module.functions.len());
        let input = scoop_lir::ConeLirOutput::try_new(module, Vec::new()).unwrap();
        let profile =
            ValidatedBackendProfile::for_selection(scoop_target_data::selection(target).unwrap())
                .unwrap()
                .with_optimization(scoop_lir::OptimizationMode::Release);
        let emitted = emit_object_set(
            &input,
            &scoop_lir::ConeCoordinate::reserved_single_file(),
            &[scoop_identity::ConeIdentity::CORE],
            scoop_lir::EntryProductionSourceV1::Library,
            profile,
            std::num::NonZeroUsize::MIN,
        )
        .unwrap_or_else(|error| panic!("{target:?}: {error}"));
        let mut instrumented = 0;
        for member in &emitted.members {
            let file = object::File::parse(member.bytes.as_slice()).unwrap();
            let has_counters = file.sections().any(|section| {
                let name = section.name().unwrap_or_default();
                name.contains("prf_cnts") || name.starts_with(".lprfc")
            });
            if matches!(member.units.kind(), ScoopLirObjectKindV1::CallableBody(_)) {
                assert!(has_counters, "{target:?}: callable lost profile counters");
                instrumented += 1;
            } else {
                assert!(!has_counters, "metadata does not own callable counts");
            }
        }
        assert_eq!(instrumented, map.records.len());
    }
}
