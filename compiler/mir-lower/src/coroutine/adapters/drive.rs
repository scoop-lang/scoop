//! Drive a latched continuation only after its execution owner grants the gate.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn generate_work_method(
    lowerer: &mut Lowerer,
    module: &hir::Module,
    adapter: mir::ClassId,
    frame_class: mir::ClassId,
    frame_layout: FrameLayout,
    destination: Option<FrameSlot>,
    failure_slot: &FrameSlot,
    latches: &(FrameSlot, FrameSlot),
    outer_step: &mir::Type,
    outer_continuation: mir::InterfaceId,
    outer_resume: &crate::coroutine_registry::ProtocolMethod,
    outer_failure: &crate::coroutine_registry::ProtocolMethod,
    driver: mir::FunctionId,
    state: mir::CoroutineSuspendStateId,
) -> mir::FunctionId {
    let completed = lowerer
        .coroutines
        .step_metadata_for_type(outer_step)
        .completed();
    let mut locals = Arena::new();
    let this = locals.alloc(local("this", mir::Type::Class(adapter)));
    let step = locals.alloc(local("$step", outer_step.clone()));
    let claim = locals.alloc(local(
        "$frame_claim",
        mir::Type::MachineScalar(mir::MachineScalarKind::CoroutineFrameState),
    ));
    let mut blocks = Arena::new();
    let mut exception_states = Arena::new();
    let invalid = protocol_error_block(lowerer, module, &mut locals, &mut blocks, None);
    let exits = drive_exit_blocks(
        lowerer,
        module,
        &mut locals,
        &mut blocks,
        &mut exception_states,
        this,
        step,
        adapter,
        frame_class,
        frame_layout,
        outer_step,
        outer_continuation,
        outer_resume,
        outer_failure,
    );
    let adapter_value = || mir::Expr::local(this, mir::Type::Class(adapter));
    let mut drive = |latch: &FrameSlot, target: Option<&FrameSlot>, resume_state| {
        let mut statements = vec![exits.switch.enter(frame_field(
            adapter_frame(this, adapter, frame_class),
            frame_layout.task.field_index(),
            mir::Type::Context(frame_layout.task_storage),
        ))];
        if let Some(target) = target {
            let value = mir::Expr::new(
                latch.value_ty.clone(),
                mir::ExprKind::EnumField {
                    operand: Box::new(frame_field(
                        adapter_value(),
                        latch.field,
                        latch.slot_ty.clone(),
                    )),
                    variant: latch.value_payload.variant().variant_index(),
                    index: latch.value_payload.field_index(),
                },
            );
            statements.push(field_set(
                adapter_frame(this, adapter, frame_class),
                target.field,
                slot_value(target, value),
            ));
        }
        statements.push(atomic_field_store(
            adapter_value(),
            1,
            adapter_state(ADAPTER_CONSUMED),
        ));
        statements.push(statement(mir::StatementKind::Call(
            mir::CallEffect::Value {
                destination: step,
                call: mir::Call {
                    target: mir::CallTarget::Direct(mir::Callee::User(driver)),
                    args: vec![
                        adapter_frame(this, adapter, frame_class),
                        frame_state(resume_state),
                    ],
                },
            },
        )));
        let body = blocks.alloc(mir::BasicBlock {
            terminator_origin: Default::default(),
            name: format!("drive_{resume_state:?}"),
            statements,
            terminator: mir::Terminator::Branch {
                cond: is_completed(step, outer_step.clone(), completed),
                then_block: exits.completed,
                else_block: exits.suspended,
            },
            unwind: Some(exits.catch_pad),
        });
        blocks.alloc(mir::BasicBlock {
            terminator_origin: Default::default(),
            name: format!("claim_{resume_state:?}"),
            statements: vec![statement(mir::StatementKind::ValDecl {
                local: claim,
                init: atomic_field_compare_exchange(
                    adapter_frame(this, adapter, frame_class),
                    frame_layout.state.field_index(),
                    frame_state_value(suspended_state(state)),
                    frame_state_value(STATE_RUNNING),
                ),
            })],
            terminator: mir::Terminator::Branch {
                cond: machine_eq(
                    mir::Expr::local(
                        claim,
                        mir::Type::MachineScalar(mir::MachineScalarKind::CoroutineFrameState),
                    ),
                    frame_state_value(suspended_state(state)),
                ),
                then_block: body,
                else_block: invalid,
            },
            unwind: None,
        })
    };
    let success = drive(&latches.0, destination.as_ref(), suspended_state(state));
    let failure = drive(&latches.1, Some(failure_slot), failure_state(state));
    let ready_is = |ready| {
        machine_eq(
            atomic_field_load(
                adapter_value(),
                1,
                mir::MachineScalarKind::CoroutineAdapterState,
            ),
            adapter_state_value(ready),
        )
    };
    let failure_check = blocks.alloc(mir::BasicBlock {
        terminator_origin: Default::default(),
        name: "check_failure".into(),
        statements: Vec::new(),
        terminator: mir::Terminator::Branch {
            cond: ready_is(ADAPTER_LATCHED_FAILURE),
            then_block: failure,
            else_block: invalid,
        },
        unwind: None,
    });
    let entry = blocks.alloc(mir::BasicBlock {
        terminator_origin: Default::default(),
        name: "check_success".into(),
        statements: Vec::new(),
        terminator: mir::Terminator::Branch {
            cond: ready_is(ADAPTER_LATCHED_SUCCESS),
            then_block: success,
            else_block: failure_check,
        },
        unwind: None,
    });
    let function = lowerer.functions.alloc(mir::Function {
        debug_origin: Default::default(),
        gc_effect: mir::GcEffect::Managed,
        name: format!("CoroutineAdapter.run${state}"),
        params: vec![mir::Param {
            name: "this".into(),
            ty: mir::Type::Class(adapter),
            local: this,
        }],
        return_ty: mir::Type::Unit,
        body: mir::Body {
            exception_states,
            locals,
            blocks,
            entry,
            loop_header_polls: Vec::new(),
        },
    });
    lowerer.top_level.push(function);
    function
}
