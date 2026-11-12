use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn rewrite_registration(
    lowerer: &mut Lowerer,
    module: &hir::Module,
    body: &mut mir::Body,
    frame_local: mir::LocalId,
    frame_class: mir::ClassId,
    frame_layout: FrameLayout,
    site: &ResumeSite,
    adapter: &GeneratedAdapter,
    adapter_local: mir::LocalId,
    result_latch: &FrameSlot,
    failure_latch: &FrameSlot,
    outer_step: &mir::Type,
    post: mir::BlockId,
    unwind: Option<mir::BlockId>,
    call: mir::CallEffect,
    returned: Option<(mir::LocalId, mir::Type)>,
) {
    let throwable = crate::coroutine_registry::throwable_type(module, &lowerer.class_map);
    let outer_suspended = lowerer
        .coroutines
        .step_metadata_for_type(outer_step)
        .suspended();
    let exception = body.locals.alloc(local(
        &format!("$resume_exception.{}", site.state),
        throwable.clone(),
    ));
    let registration_claim = body.locals.alloc(local(
        &format!("$registration_claim.{}", site.state),
        mir::Type::MachineScalar(mir::MachineScalarKind::CoroutineAdapterState),
    ));
    let frame_claim = body.locals.alloc(local(
        &format!("$registration_frame_claim.{}", site.state),
        mir::Type::MachineScalar(mir::MachineScalarKind::CoroutineFrameState),
    ));

    let register_call = body.blocks.alloc(mir::BasicBlock {
        terminator_origin: Default::default(),
        name: format!("coroutine.register.{}", site.state),
        statements: vec![statement(mir::StatementKind::Call(call))],
        terminator: mir::Terminator::Unreachable,
        unwind: None,
    });
    body.blocks[site.block].terminator = mir::Terminator::Goto(register_call);

    let success = body.blocks.alloc(mir::BasicBlock {
        terminator_origin: Default::default(),
        name: format!("coroutine.registered_success.{}", site.state),
        statements: {
            let mut statements = vec![atomic_field_store(
                mir::Expr::local(adapter_local, mir::Type::Class(adapter.class)),
                1,
                adapter_state(ADAPTER_CONSUMED),
            )];
            if let Some(destination) = site.destination {
                statements.push(statement(mir::StatementKind::ValDecl {
                    local: destination,
                    init: mir::Expr::new(
                        site.result.clone(),
                        mir::ExprKind::EnumField {
                            operand: Box::new(frame_field(
                                mir::Expr::local(adapter_local, mir::Type::Class(adapter.class)),
                                result_latch.field,
                                result_latch.slot_ty.clone(),
                            )),
                            variant: result_latch.value_payload.variant().variant_index(),
                            index: result_latch.value_payload.field_index(),
                        },
                    ),
                }));
            }
            statements
        },
        terminator: mir::Terminator::Goto(post),
        unwind,
    });
    let failure = body.blocks.alloc(mir::BasicBlock {
        terminator_origin: Default::default(),
        name: format!("coroutine.registered_failure.{}", site.state),
        statements: vec![
            atomic_field_store(
                mir::Expr::local(adapter_local, mir::Type::Class(adapter.class)),
                1,
                adapter_state(ADAPTER_CONSUMED),
            ),
            statement(mir::StatementKind::ValDecl {
                local: exception,
                init: mir::Expr::new(
                    throwable.clone(),
                    mir::ExprKind::EnumField {
                        operand: Box::new(frame_field(
                            mir::Expr::local(adapter_local, mir::Type::Class(adapter.class)),
                            failure_latch.field,
                            failure_latch.slot_ty.clone(),
                        )),
                        variant: failure_latch.value_payload.variant().variant_index(),
                        index: failure_latch.value_payload.field_index(),
                    },
                ),
            }),
        ],
        terminator: mir::Terminator::Throw {
            exception: mir::Expr::local(exception, throwable.clone()),
            unwind,
        },
        unwind,
    });
    let suspended = body.blocks.alloc(mir::BasicBlock {
        terminator_origin: Default::default(),
        name: format!("coroutine.registered_suspend.{}", site.state),
        statements: Vec::new(),
        terminator: mir::Terminator::Return {
            value: suspended_value(outer_step, outer_suspended),
        },
        unwind: None,
    });
    let invalid = protocol_error_block(lowerer, module, &mut body.locals, &mut body.blocks, unwind);
    let claim_success_frame = body.blocks.alloc(mir::BasicBlock {
        terminator_origin: Default::default(),
        name: format!("coroutine.claim_success_frame.{}", site.state),
        statements: vec![statement(mir::StatementKind::ValDecl {
            local: frame_claim,
            init: atomic_field_compare_exchange(
                mir::Expr::local(frame_local, mir::Type::Class(frame_class)),
                frame_layout.state.field_index(),
                frame_state_value(suspended_state(site.state)),
                frame_state_value(STATE_RUNNING),
            ),
        })],
        terminator: mir::Terminator::Branch {
            cond: machine_eq(
                mir::Expr::local(
                    frame_claim,
                    mir::Type::MachineScalar(mir::MachineScalarKind::CoroutineFrameState),
                ),
                frame_state_value(suspended_state(site.state)),
            ),
            then_block: success,
            else_block: invalid,
        },
        unwind,
    });
    let claim_failure_frame = body.blocks.alloc(mir::BasicBlock {
        terminator_origin: Default::default(),
        name: format!("coroutine.claim_failure_frame.{}", site.state),
        statements: vec![statement(mir::StatementKind::ValDecl {
            local: frame_claim,
            init: atomic_field_compare_exchange(
                mir::Expr::local(frame_local, mir::Type::Class(frame_class)),
                frame_layout.state.field_index(),
                frame_state_value(suspended_state(site.state)),
                frame_state_value(STATE_RUNNING),
            ),
        })],
        terminator: mir::Terminator::Branch {
            cond: machine_eq(
                mir::Expr::local(
                    frame_claim,
                    mir::Type::MachineScalar(mir::MachineScalarKind::CoroutineFrameState),
                ),
                frame_state_value(suspended_state(site.state)),
            ),
            then_block: failure,
            else_block: invalid,
        },
        unwind,
    });
    let check_latched_failure = body.blocks.alloc(mir::BasicBlock {
        terminator_origin: Default::default(),
        name: format!("coroutine.check_latched_failure.{}", site.state),
        statements: Vec::new(),
        terminator: mir::Terminator::Branch {
            cond: machine_eq(
                atomic_field_load(
                    mir::Expr::local(adapter_local, mir::Type::Class(adapter.class)),
                    1,
                    mir::MachineScalarKind::CoroutineAdapterState,
                ),
                adapter_state_value(ADAPTER_LATCHED_FAILURE),
            ),
            then_block: claim_failure_frame,
            else_block: invalid,
        },
        unwind,
    });
    let check_latched_success = body.blocks.alloc(mir::BasicBlock {
        terminator_origin: Default::default(),
        name: format!("coroutine.check_latched_success.{}", site.state),
        statements: Vec::new(),
        terminator: mir::Terminator::Branch {
            cond: machine_eq(
                atomic_field_load(
                    mir::Expr::local(adapter_local, mir::Type::Class(adapter.class)),
                    1,
                    mir::MachineScalarKind::CoroutineAdapterState,
                ),
                adapter_state_value(ADAPTER_LATCHED_SUCCESS),
            ),
            then_block: claim_success_frame,
            else_block: check_latched_failure,
        },
        unwind,
    });
    let completion_spin = body.blocks.alloc(mir::BasicBlock {
        terminator_origin: Default::default(),
        name: format!("coroutine.completion_backoff.{}", site.state),
        statements: Vec::new(),
        terminator: mir::Terminator::Unreachable,
        unwind: None,
    });
    let check_completing_failure = body.blocks.alloc(mir::BasicBlock {
        terminator_origin: Default::default(),
        name: format!("coroutine.check_completing_failure.{}", site.state),
        statements: Vec::new(),
        terminator: mir::Terminator::Branch {
            cond: machine_eq(
                atomic_field_load(
                    mir::Expr::local(adapter_local, mir::Type::Class(adapter.class)),
                    1,
                    mir::MachineScalarKind::CoroutineAdapterState,
                ),
                adapter_state_value(ADAPTER_COMPLETING_FAILURE),
            ),
            then_block: completion_spin,
            else_block: check_latched_success,
        },
        unwind,
    });
    let completion_wait = body.blocks.alloc(mir::BasicBlock {
        terminator_origin: Default::default(),
        name: format!("coroutine.completion_wait.{}", site.state),
        statements: Vec::new(),
        terminator: mir::Terminator::Branch {
            cond: machine_eq(
                atomic_field_load(
                    mir::Expr::local(adapter_local, mir::Type::Class(adapter.class)),
                    1,
                    mir::MachineScalarKind::CoroutineAdapterState,
                ),
                adapter_state_value(ADAPTER_COMPLETING_SUCCESS),
            ),
            then_block: completion_spin,
            else_block: check_completing_failure,
        },
        unwind,
    });
    body.loop_header_polls
        .push(mir::LoopHeaderPollTarget::new(completion_wait));
    body.blocks[completion_spin].terminator = mir::Terminator::Goto(completion_wait);
    let register_return = body.blocks.alloc(mir::BasicBlock {
        terminator_origin: Default::default(),
        name: format!("coroutine.register_return.{}", site.state),
        statements: vec![statement(mir::StatementKind::ValDecl {
            local: registration_claim,
            init: atomic_field_compare_exchange(
                mir::Expr::local(adapter_local, mir::Type::Class(adapter.class)),
                1,
                adapter_state_value(ADAPTER_REGISTERING),
                adapter_state_value(ADAPTER_WAITING),
            ),
        })],
        terminator: mir::Terminator::Branch {
            cond: machine_eq(
                mir::Expr::local(
                    registration_claim,
                    mir::Type::MachineScalar(mir::MachineScalarKind::CoroutineAdapterState),
                ),
                adapter_state_value(ADAPTER_REGISTERING),
            ),
            then_block: suspended,
            else_block: completion_wait,
        },
        unwind,
    });
    let registration_propagate = body.blocks.alloc(mir::BasicBlock {
        terminator_origin: Default::default(),
        name: format!("coroutine.registration_propagate.{}", site.state),
        statements: Vec::new(),
        terminator: mir::Terminator::Throw {
            exception: mir::Expr::local(exception, throwable.clone()),
            unwind,
        },
        unwind,
    });
    let registration_claim_frame = body.blocks.alloc(mir::BasicBlock {
        terminator_origin: Default::default(),
        name: format!("coroutine.registration_claim_frame.{}", site.state),
        statements: vec![statement(mir::StatementKind::ValDecl {
            local: frame_claim,
            init: atomic_field_compare_exchange(
                mir::Expr::local(frame_local, mir::Type::Class(frame_class)),
                frame_layout.state.field_index(),
                frame_state_value(suspended_state(site.state)),
                frame_state_value(STATE_RUNNING),
            ),
        })],
        terminator: mir::Terminator::Branch {
            cond: machine_eq(
                mir::Expr::local(
                    frame_claim,
                    mir::Type::MachineScalar(mir::MachineScalarKind::CoroutineFrameState),
                ),
                frame_state_value(suspended_state(site.state)),
            ),
            then_block: registration_propagate,
            else_block: invalid,
        },
        unwind,
    });
    let registration_protocol = body.blocks.alloc(mir::BasicBlock {
        terminator_origin: Default::default(),
        name: format!("coroutine.registration_protocol.{}", site.state),
        statements: vec![atomic_field_store(
            mir::Expr::local(adapter_local, mir::Type::Class(adapter.class)),
            1,
            adapter_state(ADAPTER_CONSUMED),
        )],
        terminator: mir::Terminator::Goto(invalid),
        unwind,
    });
    let registration_protocol_claim_frame = body.blocks.alloc(mir::BasicBlock {
        terminator_origin: Default::default(),
        name: format!("coroutine.registration_protocol_claim_frame.{}", site.state),
        statements: vec![statement(mir::StatementKind::ValDecl {
            local: frame_claim,
            init: atomic_field_compare_exchange(
                mir::Expr::local(frame_local, mir::Type::Class(frame_class)),
                frame_layout.state.field_index(),
                frame_state_value(suspended_state(site.state)),
                frame_state_value(STATE_RUNNING),
            ),
        })],
        terminator: mir::Terminator::Branch {
            cond: machine_eq(
                mir::Expr::local(
                    frame_claim,
                    mir::Type::MachineScalar(mir::MachineScalarKind::CoroutineFrameState),
                ),
                frame_state_value(suspended_state(site.state)),
            ),
            then_block: registration_protocol,
            else_block: invalid,
        },
        unwind,
    });
    let registration_spin = body.blocks.alloc(mir::BasicBlock {
        terminator_origin: Default::default(),
        name: format!("coroutine.registration_backoff.{}", site.state),
        statements: Vec::new(),
        terminator: mir::Terminator::Unreachable,
        unwind: None,
    });
    let registration_check_completing_failure = body.blocks.alloc(mir::BasicBlock {
        terminator_origin: Default::default(),
        name: format!(
            "coroutine.registration_check_completing_failure.{}",
            site.state
        ),
        statements: Vec::new(),
        terminator: mir::Terminator::Branch {
            cond: machine_eq(
                atomic_field_load(
                    mir::Expr::local(adapter_local, mir::Type::Class(adapter.class)),
                    1,
                    mir::MachineScalarKind::CoroutineAdapterState,
                ),
                adapter_state_value(ADAPTER_COMPLETING_FAILURE),
            ),
            then_block: registration_spin,
            else_block: registration_protocol_claim_frame,
        },
        unwind,
    });
    let registration_wait = body.blocks.alloc(mir::BasicBlock {
        terminator_origin: Default::default(),
        name: format!("coroutine.registration_wait.{}", site.state),
        statements: Vec::new(),
        terminator: mir::Terminator::Branch {
            cond: machine_eq(
                atomic_field_load(
                    mir::Expr::local(adapter_local, mir::Type::Class(adapter.class)),
                    1,
                    mir::MachineScalarKind::CoroutineAdapterState,
                ),
                adapter_state_value(ADAPTER_COMPLETING_SUCCESS),
            ),
            then_block: registration_spin,
            else_block: registration_check_completing_failure,
        },
        unwind,
    });
    body.loop_header_polls
        .push(mir::LoopHeaderPollTarget::new(registration_wait));
    body.blocks[registration_spin].terminator = mir::Terminator::Goto(registration_wait);
    let exception_state = body.exception_states.alloc(mir::ExceptionState);
    let registration_failure = body.blocks.alloc(mir::BasicBlock {
        terminator_origin: Default::default(),
        name: format!("coroutine.registration_failure.{}", site.state),
        statements: vec![
            statement(mir::StatementKind::Eh(mir::EhStatement::LandingPad {
                cleanup: false,
                state: exception_state,
            })),
            statement(mir::StatementKind::Eh(mir::EhStatement::BeginCatch {
                state: exception_state,
            })),
            statement(mir::StatementKind::ValDecl {
                local: exception,
                init: mir::Expr::new(
                    throwable.clone(),
                    mir::ExprKind::CaughtException {
                        state: exception_state,
                    },
                ),
            }),
            statement(mir::StatementKind::Eh(mir::EhStatement::EndCatch)),
            statement(mir::StatementKind::ValDecl {
                local: registration_claim,
                init: atomic_field_compare_exchange(
                    mir::Expr::local(adapter_local, mir::Type::Class(adapter.class)),
                    1,
                    adapter_state_value(ADAPTER_REGISTERING),
                    adapter_state_value(ADAPTER_CONSUMED),
                ),
            }),
        ],
        terminator: mir::Terminator::Branch {
            cond: machine_eq(
                mir::Expr::local(
                    registration_claim,
                    mir::Type::MachineScalar(mir::MachineScalarKind::CoroutineAdapterState),
                ),
                adapter_state_value(ADAPTER_REGISTERING),
            ),
            then_block: registration_claim_frame,
            else_block: registration_wait,
        },
        unwind: None,
    });
    body.blocks[register_call].terminator = match returned {
        None => mir::Terminator::Goto(register_return),
        Some((result, ty)) => {
            let step = lowerer.coroutines.step_metadata_for_type(&ty);
            let completed = step.completed();
            let payload = step.completed_payload();
            let ready = body.blocks.alloc(mir::BasicBlock {
                terminator_origin: Default::default(),
                name: format!("coroutine.returned_value.{}", site.state),
                statements: vec![
                    field_set(
                        mir::Expr::local(adapter_local, mir::Type::Class(adapter.class)),
                        result_latch.field,
                        slot_value(
                            result_latch,
                            mir::Expr::new(
                                site.result.clone(),
                                mir::ExprKind::EnumField {
                                    operand: Box::new(mir::Expr::local(result, ty.clone())),
                                    variant: payload.variant().variant_index(),
                                    index: payload.field_index(),
                                },
                            ),
                        ),
                    ),
                    atomic_field_store(
                        mir::Expr::local(adapter_local, mir::Type::Class(adapter.class)),
                        1,
                        adapter_state(ADAPTER_LATCHED_SUCCESS),
                    ),
                ],
                terminator: mir::Terminator::Goto(register_return),
                unwind,
            });
            let claim = body.blocks.alloc(mir::BasicBlock {
                terminator_origin: Default::default(),
                name: format!("coroutine.returned_claim.{}", site.state),
                statements: vec![statement(mir::StatementKind::ValDecl {
                    local: registration_claim,
                    init: atomic_field_compare_exchange(
                        mir::Expr::local(adapter_local, mir::Type::Class(adapter.class)),
                        1,
                        adapter_state_value(ADAPTER_REGISTERING),
                        adapter_state_value(ADAPTER_COMPLETING_SUCCESS),
                    ),
                })],
                terminator: mir::Terminator::Branch {
                    cond: machine_eq(
                        mir::Expr::local(
                            registration_claim,
                            mir::Type::MachineScalar(mir::MachineScalarKind::CoroutineAdapterState),
                        ),
                        adapter_state_value(ADAPTER_REGISTERING),
                    ),
                    then_block: ready,
                    else_block: registration_wait,
                },
                unwind,
            });
            mir::Terminator::Branch {
                cond: is_completed(result, ty, completed),
                then_block: claim,
                else_block: register_return,
            }
        }
    };
    body.blocks[register_call].unwind = Some(registration_failure);
}
