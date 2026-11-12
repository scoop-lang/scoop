//! Merge suspend call bodies before constructing their frames and completion ABI.

use std::collections::HashMap;

use super::{CallerState, Choice, clone, exception_edges, graph, index, mir, policy, summary};
use crate::Lowerer;
use crate::cfg::{CallSite, CallSiteKind};

mod sites;

struct Template {
    function: mir::Function,
    calls: Vec<CallSite>,
    summary: summary::Summary,
}

pub(in crate::optimization) fn run(lowerer: &mut Lowerer) {
    for source in &lowerer.suspend_sources {
        let function = &mut lowerer.functions[source.function];
        let mut ancestry =
            vec![vec![super::Target::Local(source.function)]; function.body.blocks.len()];
        let moved = exception_edges::separate_entries(function, &mut ancestry);
        sites::move_pad_calls(
            lowerer
                .coroutines
                .pre_coroutine_call_sites
                .get_mut(&source.function)
                .expect("a suspend source has its call order"),
            &moved,
        );
    }
    let templates = lowerer
        .suspend_sources
        .iter()
        .map(|source| {
            let function = lowerer.functions[source.function].clone();
            let calls = lowerer.coroutines.pre_coroutine_call_sites[&source.function].clone();
            let summary = summary::Summary::from_blocks(
                &lowerer.structs.defs,
                &function,
                function
                    .body
                    .blocks
                    .iter()
                    .map(|(id, block)| (id, block.clone()))
                    .collect(),
            );
            (
                source.function,
                Template {
                    function,
                    calls,
                    summary,
                },
            )
        })
        .collect::<HashMap<_, _>>();
    let eligible = eligible_sources(lowerer, &templates);
    let callers = lowerer
        .suspend_sources
        .iter()
        .map(|source| (source.function, source.materialization))
        .collect::<Vec<_>>();
    for (id, owner) in callers {
        let source = &templates[&id];
        let mut function = source.function.clone();
        let mut calls = source.calls.clone();
        let mut state = CallerState {
            growth: policy::Growth::new(source.summary.original.size, 96),
            ancestry: vec![vec![super::Target::Local(id)]; function.body.blocks.len()],
        };
        while let Some((order, callee)) = choose(
            lowerer, &templates, &eligible, &function, &calls, &mut state,
        ) {
            let site = calls[order].clone();
            let template = &templates[&callee];
            let mir::StatementKind::Call(effect) =
                &function.body.blocks[site.block].statements[site.statement].kind
            else {
                unreachable!("a selected call-site record identifies its CFG call");
            };
            let choice = Choice {
                block: site.block,
                statement: site.statement,
                callee: super::Target::Local(callee),
                call: super::super::call(effect).clone(),
                blocks: template
                    .function
                    .body
                    .blocks
                    .iter()
                    .map(|(id, block)| (id, block.clone()))
                    .collect(),
            };
            let expansion = clone::apply(
                &mut function,
                &template.function,
                choice,
                &mut lowerer.local_values,
                &mut lowerer.debug_records,
                owner,
                id,
                &mut state.ancestry,
                &sites::auxiliary_locals(&template.calls),
            );
            sites::splice(&mut calls, order, &template.calls, &expansion);
        }
        lowerer.functions[id] = function;
        lowerer
            .coroutines
            .pre_coroutine_call_sites
            .insert(id, calls);
    }
}

fn eligible_sources(
    lowerer: &Lowerer,
    templates: &HashMap<mir::FunctionId, Template>,
) -> Vec<bool> {
    let mut allowed = vec![false; lowerer.functions.len()];
    let mut dependencies = vec![Vec::new(); lowerer.functions.len()];
    for (&id, template) in templates {
        allowed[index(id)] = policy::executable(&template.function);
        for site in &template.calls {
            if matches!(site.kind, CallSiteKind::Suspend(_)) {
                continue;
            }
            let body = &template.function.body;
            let statement = &body.blocks[site.block].statements[site.statement];
            if let Some((call, _, _)) = crate::coroutine::suspend_effect(lowerer, body, statement)
                && let Some(callee) =
                    direct_target(lowerer, &call).filter(|id| templates.contains_key(id))
            {
                dependencies[index(id)].push(index(callee));
            }
        }
    }
    let recursive = graph::cyclic_nodes(&dependencies);
    allowed
        .iter()
        .zip(recursive)
        .map(|(allowed, recursive)| *allowed && !recursive)
        .collect()
}

fn choose(
    lowerer: &Lowerer,
    templates: &HashMap<mir::FunctionId, Template>,
    eligible: &[bool],
    function: &mir::Function,
    calls: &[CallSite],
    state: &mut CallerState,
) -> Option<(usize, mir::FunctionId)> {
    let heats = policy::heats(&function.body);
    let mut order = calls.iter().enumerate().collect::<Vec<_>>();
    order.sort_by_key(|(_, site)| {
        let heat = heats[index(site.block)];
        std::cmp::Reverse(heat.numerator * 8 / heat.denominator)
    });
    for (order, site) in order {
        if !matches!(site.kind, CallSiteKind::Call) || state.ancestry[index(site.block)].len() > 6 {
            continue;
        }
        let mir::StatementKind::Call(effect) =
            &function.body.blocks[site.block].statements[site.statement].kind
        else {
            unreachable!("a call-site record identifies its CFG call");
        };
        let call = super::super::call(effect);
        let Some(callee) = direct_target(lowerer, call) else {
            continue;
        };
        if !eligible[index(callee)]
            || state.ancestry[index(site.block)].contains(&super::Target::Local(callee))
        {
            continue;
        }
        let source = &templates[&callee];
        let heat = heats[index(site.block)];
        let growth = source.summary.original.size.saturating_sub(6).max(1);
        // These boundaries otherwise materialize separate coroutine frames and
        // completion adapters. Bound code growth without requiring a proof of
        // synchronous completion or using the ordinary-call benefit estimate.
        if growth <= state.growth.remaining(heat) {
            state.growth.charge(growth, heat);
            return Some((order, callee));
        }
    }
    None
}

fn direct_target(lowerer: &Lowerer, call: &mir::Call) -> Option<mir::FunctionId> {
    match call.target {
        mir::CallTarget::Direct(mir::Callee::User(id)) => Some(id),
        mir::CallTarget::Direct(mir::Callee::Monomorphized(id)) => {
            Some(lowerer.instances.meta[id].function)
        }
        _ => None,
    }
}
