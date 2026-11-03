use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn rewrite_suspension_site(
    lowerer: &mut Lowerer,
    module: &hir::Module,
    body: &mut mir::Body,
    frame_local: mir::LocalId,
    frame_class: mir::ClassId,
    frame: mir::CoroutineFrameId,
    frame_layout: FrameLayout,
    frame_slots: &HashMap<mir::LocalId, FrameSlot>,
    parents: Vec<mir::CoroutinePendingTransfer>,
    failure_slot: FrameSlot,
    failure_value: mir::CoroutineFailureValueId,
    outer_step: &mir::Type,
    outer_continuation: mir::InterfaceId,
    outer_resume: &crate::coroutine_registry::ProtocolMethod,
    outer_failure: &crate::coroutine_registry::ProtocolMethod,
    source_name: &str,
    driver: mir::FunctionId,
    source: hir::CallableMaterialization,
    source_odr_group: Option<hir::OdrGroupId>,
    site: ResumeSite,
    kind: SuspendKind,
    post: mir::BlockId,
    unwind: Option<mir::BlockId>,
) -> GeneratedSite {
    let throwable = crate::coroutine_registry::throwable_type(module, &lowerer.class_map);
    let (result_latch_id, result_latch_ty) = lowerer.coroutine_slot_for(&site.result);
    let (failure_latch_id, failure_latch_ty) = lowerer.coroutine_slot_for(&throwable);
    let result_latch = FrameSlot::new(
        2,
        result_latch_id,
        result_latch_ty,
        &lowerer.coroutines.slots[result_latch_id],
    );
    let failure_latch = FrameSlot::new(
        3,
        failure_latch_id,
        failure_latch_ty,
        &lowerer.coroutines.slots[failure_latch_id],
    );
    let outer_suspended = lowerer
        .coroutines
        .step_metadata_for_type(outer_step)
        .suspended();
    let adapter = generate_adapter(
        lowerer,
        module,
        frame_class,
        frame_layout,
        site.destination.map(|local| frame_slots[&local].clone()),
        failure_slot.clone(),
        outer_step,
        outer_continuation,
        outer_resume,
        outer_failure,
        source_name,
        driver,
        source,
        source_odr_group,
        site.identity_path.clone(),
        site.state,
        &site.result,
        (result_latch.clone(), failure_latch.clone()),
        !matches!(&kind, SuspendKind::Call(_)),
    );
    let adapter_local = body.locals.alloc(local(
        &format!("$safe_adapter.{}", site.state),
        mir::Type::Class(adapter.class),
    ));
    let current = &mut body.blocks[site.block];
    for local in site
        .live_after
        .iter()
        .copied()
        .filter(|local| Some(*local) != site.destination)
    {
        if let Some(slot) = frame_slots.get(&local) {
            current.statements.push(save_statement(
                mir::Expr::local(frame_local, mir::Type::Class(frame_class)),
                mir::Expr::local(local, body.locals[local].ty.clone()),
                slot,
            ));
        }
    }
    current.statements.push(atomic_field_store(
        mir::Expr::local(frame_local, mir::Type::Class(frame_class)),
        frame_layout.state.field_index(),
        frame_state(suspended_state(site.state)),
    ));
    current.statements.extend(initialized_generated_class(
        adapter_local,
        adapter.class,
        vec![
            mir::Expr::local(frame_local, mir::Type::Class(frame_class)),
            adapter_state(if matches!(&kind, SuspendKind::Yield) {
                ADAPTER_WAITING
            } else {
                ADAPTER_REGISTERING
            }),
            slot_empty(&result_latch),
            slot_empty(&failure_latch),
        ],
    ));
    let registration =
        match kind {
            SuspendKind::Yield => {
                current.statements.push(statement(mir::StatementKind::Call(
                    mir::CallEffect::Unit(mir::Call {
                        target: mir::CallTarget::Direct(mir::Callee::User(adapter.resume)),
                        args: vec![
                            mir::Expr::local(adapter_local, mir::Type::Class(adapter.class)),
                            mir::Expr::unit(),
                        ],
                    }),
                )));
                current.terminator = mir::Terminator::Return {
                    value: suspended_value(outer_step, outer_suspended),
                };
                None
            }
            SuspendKind::Intrinsic { method, receiver } => Some((
                mir::CallEffect::Unit(mir::Call {
                    target: method.call(*receiver),
                    args: vec![adapter.reference(adapter_local)],
                }),
                None,
            )),
            SuspendKind::Call(mut call) => {
                let ty = callee_return_type(lowerer, &call.target);
                let result = body
                    .locals
                    .alloc(local(&format!("$call_step.{}", site.state), ty.clone()));
                call.args.push(adapter.reference(adapter_local));
                Some((
                    mir::CallEffect::Value {
                        destination: result,
                        call,
                    },
                    Some((result, ty)),
                ))
            }
        };
    if let Some((call, returned)) = registration {
        registration::rewrite_registration(
            lowerer,
            module,
            body,
            frame_local,
            frame_class,
            frame_layout,
            &site,
            &adapter,
            adapter_local,
            &result_latch,
            &failure_latch,
            outer_step,
            post,
            unwind,
            call,
            returned,
        );
    }

    let mut resume_statements = Vec::new();
    for local in &site.live_after {
        if Some(*local) == site.destination {
            continue;
        }
        if let Some(slot) = frame_slots.get(local) {
            resume_statements.push(restore_statement(
                mir::Expr::local(frame_local, mir::Type::Class(frame_class)),
                *local,
                slot,
            ));
        }
    }
    if let Some(destination) = site.destination {
        resume_statements.push(restore_statement(
            mir::Expr::local(frame_local, mir::Type::Class(frame_class)),
            destination,
            &frame_slots[&destination],
        ));
    }
    let resume_block = body.blocks.alloc(mir::BasicBlock {
        terminator_origin: Default::default(),
        name: format!("coroutine.resume.{}", site.state),
        statements: resume_statements,
        terminator: mir::Terminator::Goto(post),
        unwind,
    });
    let failure_block =
        failure_resume_block(body, frame_local, frame_slots, failure_slot, &site, unwind);
    let point = register_resume_point(
        lowerer,
        frame,
        site.state,
        site.result,
        &adapter,
        parents,
        post,
        resume_block,
        failure_block,
        failure_value,
        unwind,
    );
    GeneratedSite { point }
}
