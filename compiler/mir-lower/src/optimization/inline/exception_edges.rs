//! Keep native landing-pad entry separate from normal exception forwarding.

use super::{Target, index, mir};

#[derive(Clone, Copy)]
pub(super) struct Handler {
    pub state: mir::ExceptionStateId,
    pub continuation: mir::BlockId,
    pub cleanup: bool,
}

pub(super) fn separate_entries(
    function: &mut mir::Function,
    ancestry: &mut Vec<Vec<Target>>,
) -> Vec<(mir::BlockId, mir::BlockId)> {
    let pads = function
        .body
        .blocks
        .iter()
        .filter_map(|(id, block)| {
            matches!(
                block.statements.first().map(|statement| &statement.kind),
                Some(mir::StatementKind::Eh(mir::EhStatement::LandingPad { .. }))
            )
            .then_some(id)
        })
        .collect::<Vec<_>>();
    let mut moved = Vec::new();
    for pad in pads {
        let block = &mut function.body.blocks[pad];
        if block.statements.len() == 1 && matches!(block.terminator, mir::Terminator::Goto(_)) {
            continue;
        }
        let body = mir::BasicBlock {
            name: format!("{}.body", block.name),
            statements: block.statements.split_off(1),
            terminator: std::mem::replace(&mut block.terminator, mir::Terminator::Unreachable),
            terminator_origin: block.terminator_origin.clone(),
            unwind: block.unwind,
        };
        let continuation = function.body.blocks.alloc(body);
        ancestry.push(ancestry[index(pad)].clone());
        function.body.blocks[pad].terminator = mir::Terminator::Goto(continuation);
        moved.push((pad, continuation));
    }
    moved
}

pub(super) fn handler(function: &mir::Function, pad: mir::BlockId) -> Handler {
    let block = &function.body.blocks[pad];
    let mir::StatementKind::Eh(mir::EhStatement::LandingPad { state, cleanup }) =
        block.statements[0].kind
    else {
        unreachable!("an unwind edge enters a native landing pad")
    };
    let mir::Terminator::Goto(continuation) = block.terminator else {
        unreachable!("native exception entry is separated before inline selection")
    };
    Handler {
        state,
        continuation,
        cleanup,
    }
}
