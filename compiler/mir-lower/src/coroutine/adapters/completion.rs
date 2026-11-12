//! Publish one result before asking the task owner to schedule its next drive.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn generate_completion_method(
    lowerer: &mut Lowerer,
    module: &hir::Module,
    adapter: mir::ClassId,
    frame_class: mir::ClassId,
    frame_layout: FrameLayout,
    state: mir::CoroutineSuspendStateId,
    latch: &FrameSlot,
    failure: bool,
    external: bool,
    work_interface: mir::InterfaceId,
    resume: crate::coroutine_registry::ProtocolMethod,
) -> mir::FunctionId {
    let mut locals = Arena::new();
    let this = locals.alloc(local("this", mir::Type::Class(adapter)));
    let value = locals.alloc(local("value", latch.value_ty.clone()));
    let claim = locals.alloc(local(
        "$claim",
        mir::Type::MachineScalar(mir::MachineScalarKind::CoroutineAdapterState),
    ));
    let mut blocks = Arena::new();
    let invalid = protocol_error_block(lowerer, module, &mut locals, &mut blocks, None);
    let completing = if failure {
        ADAPTER_COMPLETING_FAILURE
    } else {
        ADAPTER_COMPLETING_SUCCESS
    };
    let ready = if failure {
        ADAPTER_LATCHED_FAILURE
    } else {
        ADAPTER_LATCHED_SUCCESS
    };
    let adapter_value = || mir::Expr::local(this, mir::Type::Class(adapter));
    let publish = || {
        vec![
            field_set(
                adapter_value(),
                latch.field,
                slot_value(latch, mir::Expr::local(value, latch.value_ty.clone())),
            ),
            atomic_field_store(adapter_value(), 1, adapter_state(ready)),
        ]
    };
    let binding_ty = mir::Type::Context(mir::ContextStorageType::new(
        frame_layout.task_storage.core,
        mir::ContextStorageRole::Binding,
    ));
    let saved_execution = mir::Expr::new(
        binding_ty.clone(),
        mir::ExprKind::Context(mir::ContextOperation::Execution {
            task: Box::new(frame_field(
                adapter_frame(this, adapter, frame_class),
                frame_layout.task.field_index(),
                mir::Type::Context(frame_layout.task_storage),
            )),
        }),
    );
    let external = if external {
        mir::Expr::new(mir::Type::Boolean, mir::ExprKind::BoolLiteral(true))
    } else {
        // A tail-forwarded primitive can complete this ordinary call adapter
        // from outside the task. Only the same execution's completion chain
        // may be consumed internally by its current drive.
        mir::Expr::new(
            mir::Type::Boolean,
            mir::ExprKind::Binary {
                op: mir::BinOp::RefNe,
                lhs: Box::new(saved_execution.clone()),
                rhs: Box::new(mir::Expr::new(
                    binding_ty,
                    mir::ExprKind::Context(mir::ContextOperation::CurrentExecution),
                )),
            },
        )
    };
    let execution = mir::Expr::new(
        mir::Type::Interface(resume.interface),
        mir::ExprKind::Context(mir::ContextOperation::UnwrapBinding {
            binding: Box::new(saved_execution),
        }),
    );
    let work_ty = mir::Type::Interface(work_interface);
    let work = mir::Expr::new(
        work_ty.clone(),
        mir::ExprKind::Retype {
            operand: Box::new(adapter_value()),
            ty: Box::new(work_ty),
        },
    );
    let mut submitted = publish();
    submitted.push(statement(mir::StatementKind::Call(mir::CallEffect::Unit(
        mir::Call {
            target: resume.call(execution),
            args: vec![work, external],
        },
    ))));
    let dispatch = blocks.alloc(mir::BasicBlock {
        terminator_origin: Default::default(),
        name: "publish_and_schedule".into(),
        statements: submitted,
        terminator: mir::Terminator::Return {
            value: mir::Expr::unit(),
        },
        unwind: None,
    });
    let latched = blocks.alloc(mir::BasicBlock {
        terminator_origin: Default::default(),
        name: "publish_during_call".into(),
        statements: publish(),
        terminator: mir::Terminator::Return {
            value: mir::Expr::unit(),
        },
        unwind: None,
    });
    let claim_block = |expected, target, otherwise| mir::BasicBlock {
        terminator_origin: Default::default(),
        name: format!("claim_{expected:?}"),
        statements: vec![statement(mir::StatementKind::ValDecl {
            local: claim,
            init: atomic_field_compare_exchange(
                adapter_value(),
                1,
                adapter_state_value(expected),
                adapter_state_value(completing),
            ),
        })],
        terminator: mir::Terminator::Branch {
            cond: machine_eq(
                mir::Expr::local(
                    claim,
                    mir::Type::MachineScalar(mir::MachineScalarKind::CoroutineAdapterState),
                ),
                adapter_state_value(expected),
            ),
            then_block: target,
            else_block: otherwise,
        },
        unwind: None,
    };
    let waiting = blocks.alloc(claim_block(ADAPTER_WAITING, dispatch, invalid));
    let entry = blocks.alloc(claim_block(ADAPTER_REGISTERING, latched, waiting));
    let function = lowerer.functions.alloc(mir::Function {
        debug_origin: Default::default(),
        gc_effect: mir::GcEffect::Managed,
        name: format!(
            "CoroutineAdapter.{}${state}",
            if failure {
                "resumeWithException"
            } else {
                "resume"
            }
        ),
        params: vec![
            mir::Param {
                name: "this".into(),
                ty: mir::Type::Class(adapter),
                local: this,
            },
            mir::Param {
                name: "value".into(),
                ty: latch.value_ty.clone(),
                local: value,
            },
        ],
        return_ty: mir::Type::Unit,
        body: mir::Body {
            exception_states: Default::default(),
            locals,
            blocks,
            entry,
            loop_header_polls: Vec::new(),
        },
    });
    lowerer.top_level.push(function);
    function
}
