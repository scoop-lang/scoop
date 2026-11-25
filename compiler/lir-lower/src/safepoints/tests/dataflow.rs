use super::*;

fn callable_body() -> lir::CallableBodyIdentity {
    let site = scoop_identity::SourceDeclarationSite::new(
        scoop_identity::ConeIdentity::SINGLE_FILE,
        scoop_identity::PackagePath::root(),
        scoop_identity::DefinitionOwnerChain::top_level(),
        scoop_identity::DeclarationScope::ConeWide,
    )
    .unwrap();
    let declaration = scoop_identity::SourceDeclarationKey::function(
        site,
        scoop_identity::CanonicalIdentifier::new("dataflowTest").unwrap(),
        0,
        None,
        Vec::new(),
    );
    let function =
        scoop_identity::PersistentFunctionId::from_source_declaration(&declaration).unwrap();
    lir::CallableBodyIdentity::for_function(function).unwrap()
}

#[test]
fn variant_primitives_track_their_operand_use_and_temp_definition() {
    let mut enums = lir::EnumDefs::default();
    let enum_id = enums.alloc(lir::EnumDef {
        exact_type: crate::tests::test_physical_exact(
            "Tracked",
            scoop_identity::SourceNominalKind::Enum,
        ),
        name: "Tracked".to_string(),
        repr: lir::EnumRepr::Tagged {
            variants: vec![lir::EnumVariantRepr {
                fields: vec![lir::EnumFieldRepr {
                    ty: lir::MANAGED_PTR,
                    offset: 8,
                }],
                slot_offset: 8,
                slot_size: 8,
                slot_align: 8,
                gc_free: false,
            }],
            size: 16,
            align: 8,
        },
        scan: lir::RefScan::References(vec![8]),
    });
    let variant = enums.variant_ref(enum_id, 0).expect("variant exists");
    let field = enums
        .variant_field_ref(variant, 0)
        .expect("payload field exists");

    let mut temps = Arena::default();
    let tested = temps.alloc(lir::Temp {
        ty: lir::LirType::I1,
    });
    let projected = temps.alloc(lir::Temp {
        ty: lir::MANAGED_PTR,
    });
    let mut blocks = Arena::default();
    let entry = blocks.alloc(lir::BasicBlock {
        terminator_origin: Default::default(),
        name: "entry".to_string(),
        instructions: Default::default(),
        terminator: lir::Terminator::Unreachable,
    });
    let function = lir::Function {
        debug_name: Default::default(),
        debug_variables: Vec::new(),
        debug_signature: Default::default(),
        debug_origin: Default::default(),
        callable_body: callable_body(),
        gc_effect: lir::GcEffect::Managed,
        signature: lir::ScoopAbiSignature::new(
            vec![lir::AbiArgument::Indirect(
                lir::AbiValue::new(
                    lir::LirType::Enum(enum_id),
                    lir::AbiNonZeroLayout::new(16, 8).expect("test enum has a valid layout"),
                    lir::RefScan::References(vec![8]),
                )
                .expect("test enum is a non-void ABI value"),
            )],
            lir::AbiReturn::UnitVoid,
            lir::CallingConvention::Cdecl,
        ),
        call_targets: lir::CallTargets::default(),
        safepoints: lir::SafepointIdentities::default(),
        locals: Arena::default(),
        temps,
        blocks,
        entry,
    };
    let operand = lir::Value::Param(0);
    let test = lir::Instruction::VariantTest {
        out: tested,
        operand,
        variant,
    };
    let project = lir::Instruction::VariantPayloadProject {
        out: projected,
        operand,
        field,
    };

    let mut uses = Vec::new();
    visit_instruction_uses(&test, &function.call_targets, |value| uses.push(value));
    assert_eq!(uses, vec![operand]);
    let mut defs = Vec::new();
    visit_instruction_defs(&test, |value| defs.push(value));
    assert_eq!(defs, vec![LiveValue::Temp(tested)]);
    uses.clear();
    visit_instruction_uses(&project, &function.call_targets, |value| uses.push(value));
    assert_eq!(uses, vec![operand]);
    defs.clear();
    visit_instruction_defs(&project, |value| defs.push(value));
    assert_eq!(defs, vec![LiveValue::Temp(projected)]);
}
