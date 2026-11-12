use super::*;

fn sample(yields: bool) -> mir::Module {
    let mut h = Harness::new();
    let mut locals = Arena::new();
    let input = locals.alloc(local("input", h.int));
    let lhs = local_ref(input, h.int);
    let rhs = int_lit(&h, 7);
    let value = integer_binary(
        &mut h,
        hir::IntegerKind::SIGNED_32,
        hir::IntegerBinaryOperator::Add,
        lhs,
        rhs,
    );
    let helper = h.user_fn_full(
        "increment",
        Vec::new(),
        vec![param("input", h.int, input)],
        h.int,
        hir::Body {
            locals,
            statements: vec![stmt(hir::StatementKind::Return { value })],
        },
    );
    let yielding = h.user_fn(
        "yieldDrive",
        hir::Body {
            locals: Arena::new(),
            statements: Vec::new(),
        },
    );
    h.functions[yielding].is_suspend = true;
    h.functions[yielding].kind = hir::FunctionKind::Intrinsic(hir::IntrinsicFunction {
        kind: hir::IntrinsicFunctionKind::CoroutineYield,
        provider: hir::IntrinsicProviderId::from_raw(0),
    });
    let mut locals = Arena::new();
    let input = locals.alloc(local("input", h.int));
    let saved = locals.alloc(local("saved", h.int));
    let mut statements = vec![val_decl(
        saved,
        call_typed(helper, vec![local_ref(input, h.int)], h.int),
    )];
    if yields {
        statements.push(expr_stmt(call_typed(yielding, Vec::new(), h.unit)));
    }
    statements.push(stmt(hir::StatementKind::Return {
        value: call_typed(helper, vec![local_ref(saved, h.int)], h.int),
    }));
    let caller = h.user_fn_full(
        "suspendCaller",
        Vec::new(),
        vec![param("input", h.int, input)],
        h.int,
        hir::Body { locals, statements },
    );
    h.functions[caller].is_suspend = true;
    let main = empty_main(&mut h);
    lower(&h.finish_coroutines(main))
}

fn calls_to(module: &mir::Module, caller: mir::FunctionId, callee: mir::FunctionId) -> usize {
    module.functions[caller]
        .body
        .blocks
        .iter()
        .flat_map(|(_, block)| &block.statements)
        .filter(|statement| {
            matches!(&statement.kind,
                mir::StatementKind::Call(mir::CallEffect::Unit(call)
                    | mir::CallEffect::Value { call, .. })
                if matches!(call.target,
                    mir::CallTarget::Direct(mir::Callee::User(target)) if target == callee))
        })
        .count()
}

#[test]
fn ordinary_helpers_inline_in_immediate_and_resumable_coroutine_bodies() {
    for yields in [false, true] {
        let mut module = sample(yields);
        let helper = module
            .functions
            .iter()
            .find(|(_, f)| f.name == "increment")
            .unwrap()
            .0;
        let (_, metadata) = module
            .meta
            .coroutine_functions
            .iter()
            .find(|(_, coroutine)| module.functions[coroutine.function].name == "suspendCaller")
            .unwrap();
        let caller = match metadata.lowering {
            mir::CoroutineLowering::Immediate => metadata.function,
            mir::CoroutineLowering::StateMachine { driver, .. } => driver,
            ref other => panic!("expected an immediate body or a yield state machine: {other:?}"),
        };
        let parameters = module.functions[caller].params.len();
        let frames = module.meta.coroutine_frames.len();
        let resumptions = module.meta.coroutine_resume_points.len();
        assert_eq!(resumptions, usize::from(yields));
        assert_eq!(calls_to(&module, caller, helper), 2);
        optimize(&mut module);
        assert_eq!(calls_to(&module, caller, helper), 0);
        assert_eq!(module.functions[caller].params.len(), parameters);
        assert_eq!(module.meta.coroutine_frames.len(), frames);
        assert_eq!(module.meta.coroutine_resume_points.len(), resumptions);
        assert_mir_foundation_projection(&module);
    }
}
