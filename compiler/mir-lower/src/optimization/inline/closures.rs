//! Expose known synchronous closure bodies within the ordinary growth limits.

use std::collections::HashSet;

use super::{Target, flow, mir, policy};
use crate::optimization::allocation::{defined_local, local_alias, statement_alias};

pub(super) fn exposes_known_invoke(
    module: &mir::Module,
    target: Target,
    callee: &mir::Function,
    blocks: &[(mir::BlockId, mir::BasicBlock)],
    arguments: &[flow::ValueFacts],
) -> bool {
    callee
        .params
        .iter()
        .zip(arguments)
        .enumerate()
        .any(|(index, (parameter, argument))| {
            let flow::ActualType::Closure(class) = argument.actual else {
                return false;
            };
            let closure = &module.closure_classes[class];
            if module.function_types[closure.function_type].is_suspend {
                return false;
            }
            let invoke = module.closure_invoke_functions[closure.invoke].function;
            // The generated environment parameter belongs to this exact invoke.
            if index == 0 && target == Target::Local(invoke) {
                return true;
            }
            only_invokes(module, callee, blocks, parameter.local, invoke)
        })
}

fn only_invokes(
    module: &mir::Module,
    callee: &mir::Function,
    blocks: &[(mir::BlockId, mir::BasicBlock)],
    parameter: mir::LocalId,
    invoke: mir::FunctionId,
) -> bool {
    let mut aliases = HashSet::from([parameter]);
    loop {
        let mut changed = false;
        for (_, block) in blocks {
            for statement in &block.statements {
                if statement_alias(statement).is_some_and(|source| aliases.contains(&source))
                    && let Some(destination) = defined_local(statement)
                {
                    changed |= aliases.insert(destination);
                }
            }
        }
        if !changed {
            break;
        }
    }
    if callee
        .params
        .iter()
        .any(|p| p.local != parameter && aliases.contains(&p.local))
    {
        return false;
    }
    let owns = |expr: &mir::Expr| local_alias(expr).is_some_and(|local| aliases.contains(&local));
    let (mut reads, mut allowed, mut calls, mut addressed) = (0, 0, 0, false);
    for (_, block) in blocks {
        for statement in &block.statements {
            let alias = statement_alias(statement).is_some_and(|local| aliases.contains(&local));
            if defined_local(statement).is_some_and(|local| aliases.contains(&local)) && !alias {
                return false;
            }
            allowed += usize::from(alias);
            if let mir::StatementKind::Call(effect) = &statement.kind {
                let call = crate::optimization::call(effect);
                let receiver = match &call.target {
                    mir::CallTarget::Closure {
                        function_type,
                        receiver,
                    } if !module.function_types[*function_type].is_suspend => Some(&**receiver),
                    _ if policy::local_target(module, call) == Some(invoke) => call.args.first(),
                    _ => None,
                };
                if receiver.is_some_and(owns) {
                    allowed += 1;
                    calls += 1;
                }
            }
        }
        mir::visit_block_exprs(block, &mut |expr| match expr.kind {
            mir::ExprKind::Local(local) => reads += usize::from(aliases.contains(&local)),
            mir::ExprKind::AddressOf { local, .. } => addressed |= aliases.contains(&local),
            _ => {}
        });
    }
    !addressed && calls != 0 && reads == allowed
}
