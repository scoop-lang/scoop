use super::*;

fn program(unit: bool, count: usize) -> hir::ExportHirOutput {
    let mut h = Harness::new();
    let result = if unit { h.unit } else { h.long };
    let main = empty_main(&mut h);
    let output = h.finish_coroutines(main);
    let entry = output.entry();
    let mut source = output.into_module();
    let previous = source.functions.len();
    let core = defined_export_core(&source).coroutines;
    let registration_ty =
        module_interface_application(&mut source, core.suspend_registration, vec![result]);
    let suspend = source.instantiations.alloc(hir::ResolvedGenericFunction {
        requirements: Vec::new(),
        generic: source.functions[core.suspend_coroutine]
            .generic_definition()
            .unwrap(),
        type_args: vec![result],
    });
    let mut locals = Arena::new();
    let registration = locals.alloc(local("registration", registration_ty));
    let call = || {
        generic_call(
            suspend,
            vec![local_ref(registration, registration_ty)],
            result,
        )
    };
    let mut statements = (1..count).map(|_| expr_stmt(call())).collect::<Vec<_>>();
    statements.push(stmt(hir::StatementKind::Return { value: call() }));
    let caller = source.functions.alloc(hir::Function {
        signature: hir::CallableSignature {
            context_parameters: Vec::new(),
            release_callability: Default::default(),
            name: "registered".into(),
            is_suspend: true,
            modifiers: Default::default(),
            params: vec![param("registration", registration_ty, registration)],
            return_ty: result,
            attributes: Default::default(),
            span: SPAN,
        },
        access: hir::DeclarationAccess::public(),
        genericity: hir::FunctionGenericity::Plain,
        kind: hir::FunctionKind::User(hir::Body { locals, statements }),
        method: None,
    });
    source.top_level.push(caller);
    extend_function_identities(&mut source, previous);
    executable_output(source, entry)
}

#[test]
fn tail_registration_passes_original_completion_and_returns_suspended() {
    for unit in [false, true] {
        let module = lower_tail(&program(unit, 1), true);
        let metadata = coroutine(&module, "registered");
        assert!(matches!(
            metadata.lowering,
            mir::CoroutineLowering::TailForwarding
        ));
        assert!(module.meta.coroutine_frames.is_empty());
        assert!(module.meta.coroutine_resume_points.is_empty());
        let function = &module.functions[metadata.function];
        let completion = function.params.last().unwrap().local;
        let suspended = module.meta.coroutine_steps[metadata.step].suspended();
        let mut registrations = 0;
        for (_, block) in function.body.blocks.iter() {
            for statement in &block.statements {
                if let mir::StatementKind::Call(mir::CallEffect::Unit(call)) = &statement.kind {
                    registrations += 1;
                    assert!(matches!(call.target, mir::CallTarget::Interface { .. }));
                    assert!(matches!(call.args.as_slice(), [mir::Expr {
                        kind: mir::ExprKind::Local(local), ..
                    }] if *local == completion));
                    assert!(matches!(&block.terminator, mir::Terminator::Return {
                        value: mir::Expr { kind: mir::ExprKind::VariantConstruct { variant, fields }, .. }
                    } if *variant == suspended && fields.is_empty()));
                }
            }
        }
        assert_eq!(registrations, 1);
        assert_mir_foundation_projection(&module);
    }
}

#[test]
fn consecutive_unit_registrations_cannot_hide_an_earlier_suspension() {
    let module = lower_tail(&program(true, 2), true);
    let metadata = coroutine(&module, "registered");
    let mir::CoroutineLowering::StateMachine { resume_points, .. } = &metadata.lowering else {
        panic!("both registrations must execute through the state machine");
    };
    assert_eq!(resume_points.len(), 2);
    assert_mir_foundation_projection(&module);
}

#[test]
fn debug_registration_retains_its_own_handshake() {
    let module = lower_tail(&program(false, 1), false);
    assert!(matches!(
        coroutine(&module, "registered").lowering,
        mir::CoroutineLowering::StateMachine { .. }
    ));
}

#[test]
fn private_registration_helpers_do_not_change_dependency_signatures() {
    let source = program(false, 1);
    let debug = lower_tail(&source, false);
    let release = lower_tail(&source, true);
    let debug = mir::CanonicalMirFoundation::from_module(&debug).unwrap();
    let release = mir::CanonicalMirFoundation::from_module(&release).unwrap();
    assert_eq!(
        mir::StrongCallableBridgeSurfaceV1::from_foundation(&debug),
        mir::StrongCallableBridgeSurfaceV1::from_foundation(&release)
    );
}
