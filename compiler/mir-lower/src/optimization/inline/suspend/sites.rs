use super::*;
use crate::cfg::{CleanupContext, CleanupTransfer, SuspendCall};

pub(super) fn move_pad_calls(calls: &mut [CallSite], moved: &[(mir::BlockId, mir::BlockId)]) {
    for site in calls {
        if let Some((_, destination)) = moved.iter().find(|(source, _)| *source == site.block) {
            site.block = *destination;
            site.statement = site
                .statement
                .checked_sub(1)
                .expect("a call in a landing pad follows the native entry");
        }
    }
}

pub(super) fn auxiliary_locals(calls: &[CallSite]) -> Vec<mir::LocalId> {
    let mut locals = std::collections::BTreeSet::new();
    for site in calls {
        if let CallSiteKind::Suspend(SuspendCall::Registration {
            receiver,
            destination,
            ..
        }) = &site.kind
        {
            locals.extend(*destination);
            mir::visit_expr(receiver, &mut |expression| {
                if let mir::ExprKind::Local(local) | mir::ExprKind::AddressOf { local, .. } =
                    expression.kind
                {
                    locals.insert(local);
                }
            });
        }
        for transfer in &site.pending.0 {
            match transfer {
                CleanupTransfer::ReturnValue(local)
                | CleanupTransfer::ManagedThrow {
                    exception: local, ..
                } => {
                    locals.insert(*local);
                }
                _ => {}
            }
        }
    }
    locals.into_iter().collect()
}

fn remap_kind(kind: &CallSiteKind, expansion: &clone::Expansion) -> CallSiteKind {
    let mut kind = kind.clone();
    if let CallSiteKind::Suspend(SuspendCall::Registration {
        receiver,
        destination,
        ..
    }) = &mut kind
    {
        *destination = destination.map(|local| remap_local(local, expansion));
        mir::visit_expr_mut(receiver, &mut |expression| match &mut expression.kind {
            mir::ExprKind::Local(local) | mir::ExprKind::AddressOf { local, .. } => {
                *local = remap_local(*local, expansion);
            }
            mir::ExprKind::CaughtException { state } => {
                *state = expansion.exception_states[index(*state)]
            }
            _ => {}
        });
    }
    kind
}

fn remap_local(local: mir::LocalId, expansion: &clone::Expansion) -> mir::LocalId {
    expansion.locals[index(local)].expect("a private call-site local is cloned with its body")
}

fn remap_pending(
    caller: &CleanupContext,
    callee: &CleanupContext,
    expansion: &clone::Expansion,
) -> CleanupContext {
    let target =
        |block| expansion.blocks[index(block)].expect("a complete suspend template is cloned");
    let mut pending = caller.0.clone();
    pending.extend(callee.0.iter().map(|transfer| {
        match transfer {
            CleanupTransfer::Fallthrough(block) => CleanupTransfer::Fallthrough(
                mir::CoroutineCleanupFallthroughTarget::new(target(block.block())),
            ),
            // The callee return now reaches the call continuation. Its result local
            // remains live through the actual cloned return/assignment CFG.
            CleanupTransfer::ReturnUnit | CleanupTransfer::ReturnValue(_) => {
                CleanupTransfer::Fallthrough(mir::CoroutineCleanupFallthroughTarget::new(
                    expansion.continuation,
                ))
            }
            CleanupTransfer::Break(block) => {
                CleanupTransfer::Break(mir::CoroutineLoopExitTarget::new(target(block.block())))
            }
            CleanupTransfer::Continue(block) => CleanupTransfer::Continue(
                mir::CoroutineLoopHeaderTarget::new(target(block.block())),
            ),
            CleanupTransfer::ManagedThrow { exception, unwind } => CleanupTransfer::ManagedThrow {
                exception: remap_local(*exception, expansion),
                unwind: unwind
                    .map(|block| target(block.block()))
                    .or(expansion.unwind)
                    .map(mir::CoroutineUnwindTarget::new),
            },
        }
    }));
    CleanupContext(pending)
}

pub(super) fn splice(
    calls: &mut Vec<CallSite>,
    order: usize,
    callee: &[CallSite],
    expansion: &clone::Expansion,
) {
    let original = calls[order].clone();
    for site in calls.iter_mut() {
        if site.block == original.block && site.statement > original.statement {
            site.block = expansion.continuation;
            site.statement = site.statement - original.statement - 1 + expansion.result_prefix;
        }
    }
    // Deferred registrations at the same position precede the selected call.
    // Keep them in the prefix and preserve the source call order of the clones.
    let cloned = callee.iter().map(|site| CallSite {
        block: expansion.blocks[index(site.block)].expect("the complete template was cloned"),
        statement: site.statement,
        pending: remap_pending(&original.pending, &site.pending, expansion),
        kind: remap_kind(&site.kind, expansion),
    });
    calls.splice(order..=order, cloned);
}
