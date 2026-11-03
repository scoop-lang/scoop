use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn rewrite_site(
    lowerer: &mut Lowerer,
    module: &hir::Module,
    body: &mut mir::Body,
    frame_local: mir::LocalId,
    frame_class: mir::ClassId,
    frame: mir::CoroutineFrameId,
    frame_layout: FrameLayout,
    frame_slots: &HashMap<mir::LocalId, FrameSlot>,
    saved_values: &HashMap<mir::LocalId, mir::CoroutineSavedValueId>,
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
    planned: SuspendSite,
) -> GeneratedSite {
    let SuspendSite { resume: site, kind } = planned;
    let parents = freeze_pending_context(site.pending.clone(), saved_values);
    let block = &mut body.blocks[site.block];
    let removed = usize::from(matches!(kind, SuspendKind::Call(_)));
    let suffix = block.statements.split_off(site.statement + removed);
    block.statements.truncate(site.statement);
    let terminator = std::mem::replace(&mut block.terminator, mir::Terminator::Unreachable);
    let unwind = block.unwind;
    let post = body.blocks.alloc(mir::BasicBlock {
        terminator_origin: Default::default(),
        name: format!("coroutine.post.{}", site.state),
        statements: suffix,
        terminator,
        unwind,
    });

    rewrite_suspension_site(
        lowerer,
        module,
        body,
        frame_local,
        frame_class,
        frame,
        frame_layout,
        frame_slots,
        parents,
        failure_slot,
        failure_value,
        outer_step,
        outer_continuation,
        outer_resume,
        outer_failure,
        source_name,
        driver,
        source,
        source_odr_group,
        site,
        kind,
        post,
        unwind,
    )
}

pub(super) fn failure_resume_block(
    body: &mut mir::Body,
    frame: mir::LocalId,
    frame_slots: &HashMap<mir::LocalId, FrameSlot>,
    failure_slot: FrameSlot,
    site: &ResumeSite,
    unwind: Option<mir::BlockId>,
) -> mir::BlockId {
    let mut statements = Vec::new();
    for local in &site.live_after {
        if Some(*local) == site.destination {
            continue;
        }
        if let Some(slot) = frame_slots.get(local) {
            statements.push(restore_statement(
                mir::Expr::local(frame, body.locals[frame].ty.clone()),
                *local,
                slot,
            ));
        }
    }
    body.blocks.alloc(mir::BasicBlock {
        terminator_origin: Default::default(),
        name: format!("coroutine.failure.{}", site.state),
        statements,
        terminator: mir::Terminator::Throw {
            exception: mir::Expr::new(
                failure_slot.value_ty.clone(),
                mir::ExprKind::EnumField {
                    operand: Box::new(frame_field(
                        mir::Expr::local(frame, body.locals[frame].ty.clone()),
                        failure_slot.field,
                        failure_slot.slot_ty.clone(),
                    )),
                    variant: failure_slot.value_payload.variant().variant_index(),
                    index: failure_slot.value_payload.field_index(),
                },
            ),
            unwind,
        },
        unwind,
    })
}
