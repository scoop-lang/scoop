//! A real suspension is merged into its caller before frame construction.

use super::*;

#[test]
fn suspend_call_fusion_keeps_yield_and_removes_the_intermediate_resume_boundary() {
    let mut h = Harness::new();
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
    let inner = h.user_fn_full(
        "inner",
        Vec::new(),
        vec![param("input", h.int, input)],
        h.int,
        hir::Body {
            locals,
            statements: vec![
                expr_stmt(call_typed(yielding, Vec::new(), h.unit)),
                stmt(hir::StatementKind::Return {
                    value: local_ref(input, h.int),
                }),
            ],
        },
    );
    h.functions[inner].is_suspend = true;
    let mut locals = Arena::new();
    let input = locals.alloc(local("input", h.int));
    let saved = locals.alloc(local("saved", h.int));
    let left = local_ref(saved, h.int);
    let right = int_lit(&h, 1);
    let result = integer_binary(
        &mut h,
        hir::IntegerKind::SIGNED_32,
        hir::IntegerBinaryOperator::Add,
        left,
        right,
    );
    let caller = h.user_fn_full(
        "caller",
        Vec::new(),
        vec![param("input", h.int, input)],
        h.int,
        hir::Body {
            locals,
            statements: vec![
                val_decl(
                    saved,
                    call_typed(inner, vec![local_ref(input, h.int)], h.int),
                ),
                stmt(hir::StatementKind::Return { value: result }),
            ],
        },
    );
    h.functions[caller].is_suspend = true;
    let main = empty_main(&mut h);
    let source = h.finish_coroutines(main);
    for enabled in [false, true] {
        let module = super::coroutine_tail::sample::lower_tail(&source, enabled);
        let (_, metadata) = module
            .meta
            .coroutine_functions
            .iter()
            .find(|(_, entry)| module.functions[entry.function].name == "caller")
            .unwrap();
        let mir::CoroutineLowering::StateMachine { driver, .. } = metadata.lowering else {
            panic!("the caller must retain its real suspension");
        };
        let calls = module.functions[driver].body.blocks.iter()
            .flat_map(|(_, block)| &block.statements)
            .filter(|statement| matches!(&statement.kind,
                mir::StatementKind::Call(mir::CallEffect::Value { call, .. } | mir::CallEffect::Unit(call))
                if matches!(call.target, mir::CallTarget::Direct(mir::Callee::User(target))
                    if module.functions[target].name == "inner")))
            .count();
        assert_eq!(calls, usize::from(!enabled));
        assert_mir_foundation_projection(&module);
    }
}
