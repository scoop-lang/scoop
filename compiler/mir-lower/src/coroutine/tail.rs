//! Delegate tail suspensions to the caller's existing registration handshake.

use super::*;

mod returns;

pub(super) fn rewrite(
    lowerer: &mut Lowerer,
    coroutine: mir::CoroutineFunctionId,
    sites: &[SuspendSite],
) -> bool {
    let metadata = &lowerer.coroutines.functions[coroutine];
    let function = &lowerer.functions[metadata.function];
    let completed = lowerer.coroutines.steps[metadata.step].completed();
    let mut addressed = HashSet::new();
    for (_, block) in function.body.blocks.iter() {
        mir::visit_block_exprs(block, &mut |expression| {
            if let mir::ExprKind::AddressOf { local, .. } = expression.kind {
                addressed.insert(local);
            }
        });
    }
    if function.body.blocks.iter().any(|(_, block)| {
        block.unwind.is_some()
            || block
                .statements
                .iter()
                .any(|statement| matches!(statement.kind, mir::StatementKind::Eh(_)))
    }) || !sites.iter().all(|site| {
        site.resume.result == metadata.source_return
            && site.resume.pending.0.is_empty()
            // The checked suspend call already selects the same completion /
            // step ABI for this exact result, including dynamic and external
            // targets. Yield must retain its unconditional scheduling boundary.
            && !matches!(site.kind, SuspendKind::Yield)
            && returns::unchanged(&function.body, site, sites, completed, &addressed)
    }) {
        return false;
    }

    let function_id = metadata.function;
    let owner = metadata.source;
    let suspended = lowerer.coroutines.steps[metadata.step].suspended();
    let function = &mut lowerer.functions[function_id];
    let completion = function
        .params
        .last()
        .expect("suspend ABI has a completion");
    let completion = mir::Expr::local(completion.local, completion.ty.clone());
    for site in sites {
        let (call, result) = match site.kind.clone() {
            SuspendKind::Call(mut call) => {
                call.args.push(completion.clone());
                let result = function.body.locals.alloc(mir::Local {
                    debug_variable: Default::default(),
                    name: format!("$tail_step.{}", site.resume.state),
                    ty: function.return_ty.clone(),
                    mutable: false,
                });
                lowerer.local_values.record_generated_local(
                    function_id,
                    result,
                    owner,
                    hir::StructuralDefinitionSiteRole::SyntheticValue,
                    hir::SyntheticLocalRole::Temporary,
                );
                (
                    mir::CallEffect::Value {
                        destination: result,
                        call,
                    },
                    mir::Expr::local(result, function.return_ty.clone()),
                )
            }
            SuspendKind::Intrinsic { method, receiver } => (
                mir::CallEffect::Unit(mir::Call {
                    target: method.call(*receiver),
                    args: vec![completion.clone()],
                }),
                suspended_value(&function.return_ty, suspended),
            ),
            SuspendKind::Yield => unreachable!("yield cannot delegate its scheduling boundary"),
        };
        let block = &mut function.body.blocks[site.resume.block];
        let mut statement = statement(mir::StatementKind::Call(call));
        if matches!(site.kind, SuspendKind::Call(_)) {
            statement.span = block.statements[site.resume.statement].span;
            statement.debug_origin = block.statements[site.resume.statement].debug_origin.clone();
        }
        block.statements.truncate(site.resume.statement);
        block.statements.push(statement);
        block.terminator = mir::Terminator::Return { value: result };
    }
    lowerer.coroutines.functions[coroutine].lowering = mir::CoroutineLowering::TailForwarding;
    true
}
