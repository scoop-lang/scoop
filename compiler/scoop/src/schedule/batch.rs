//! One worker limit for independent closures. Cache-key locks share actual work.

use super::execution::{CompilerResponse, Execution, PreparedNode};
use super::*;
use crate::ChildTransportError;
use scoop_toolchain::swift::{PreparedSwiftBuild, SwiftProduct};

enum Response {
    Compiler(CompilerResponse),
    Swift(Result<Box<SwiftProduct>, scoop_toolchain::ToolchainError>),
}

fn receive(
    execution: &mut Execution,
    graph: &PreparedBuildGraph,
    position: usize,
    response: Response,
) {
    match response {
        Response::Compiler(response) => execution.receive(graph, (position, response)),
        Response::Swift(Ok(product)) => {
            execution.swift.completed.insert(position, *product);
        }
        Response::Swift(Err(source)) => {
            execution.failures.insert(
                execution.order.len() + position,
                execution.swift.error(position, source),
            );
        }
    }
}

pub(super) fn execute(
    graphs: &[&PreparedBuildGraph],
    runner: &impl SingleConeCompilerRunner,
    include_root: bool,
) -> Result<Vec<Execution>, BuildGraphExecutionFailure> {
    let mut executions = graphs
        .iter()
        .map(|graph| Execution::new(graph, include_root))
        .collect::<Vec<_>>();
    let jobs = graphs
        .iter()
        .map(|graph| graph.jobs().get())
        .min()
        .unwrap_or(1);
    let mut next_ordinal = 0usize;
    let offsets = executions
        .iter()
        .map(|execution| {
            let offset = next_ordinal;
            next_ordinal = next_ordinal
                .checked_add(execution.order.len())
                .ok_or_else(|| {
                    BuildGraphExecutionFailure::new(
                        BuildGraphExecutionError::RequestIdOverflow,
                        &[],
                    )
                })?;
            Ok(offset)
        })
        .collect::<Result<Vec<_>, BuildGraphExecutionFailure>>()?;
    std::thread::scope(|scope| {
        let (sender, receiver) =
            std::sync::mpsc::channel::<(usize, usize, std::num::NonZeroUsize, Response)>();
        let mut active_jobs = 0;
        let mut wait_for_lock = false;
        loop {
            while let Ok((graph, position, worker_jobs, response)) = receiver.try_recv() {
                receive(&mut executions[graph], graphs[graph], position, response);
                active_jobs -= worker_jobs.get();
            }
            let mut failed = executions
                .iter()
                .any(|execution| !execution.failures.is_empty());
            for (index, graph) in graphs.iter().enumerate() {
                let mut waiting = Vec::new();
                while !failed && active_jobs < jobs {
                    let ready = executions
                        .iter()
                        .map(|execution| execution.ready.len() + execution.swift.ready.len())
                        .sum::<usize>()
                        .max(1);
                    let worker_jobs =
                        std::num::NonZeroUsize::new(((jobs - active_jobs) / ready).max(1)).unwrap();
                    let execution = &mut executions[index];
                    let Some(position) = execution.ready.pop() else {
                        break;
                    };
                    let block = std::mem::take(&mut wait_for_lock) && active_jobs == 0;
                    match execution.prepare(graph, position, offsets[index] + position, block) {
                        Ok(PreparedNode::WaitingForLock(key)) => {
                            execution.waiting_keys.insert(position, key);
                            waiting.push(position);
                        }
                        Ok(PreparedNode::Complete(key, node)) => {
                            execution.complete(position, key, *node, false);
                        }
                        Ok(PreparedNode::Pending(key, pending)) => {
                            let request = pending.invocation.request().clone();
                            let mut io = pending.invocation.io().clone();
                            io.jobs = worker_jobs;
                            let sender = sender.clone();
                            let compiler = graph.compiler();
                            let worker = std::thread::Builder::new()
                                .name(format!("scoop-compile-{index}-{position}"))
                                .spawn_scoped(scope, move || {
                                    let response =
                                        std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                                            || runner.invoke(compiler, &request, &io),
                                        ))
                                        .unwrap_or(Err(ChildTransportError::WorkerPanicked));
                                    let _ = sender.send((
                                        index,
                                        position,
                                        worker_jobs,
                                        Response::Compiler(response),
                                    ));
                                });
                            match worker {
                                Ok(_) => {
                                    execution.pending.insert(position, (key, pending));
                                    active_jobs += worker_jobs.get();
                                }
                                Err(source) => {
                                    execution.failures.insert(
                                        position,
                                        BuildGraphExecutionError::Ordinary(
                                            execution.order[position],
                                            Box::new(OrdinarySourceExecutionError::ChildTransport(
                                                ChildTransportError::WorkerSpawn(source),
                                            )),
                                        ),
                                    );
                                    failed = true;
                                }
                            }
                        }
                        Err(source) => {
                            execution.failures.insert(position, source);
                            failed = true;
                        }
                    }
                }
                executions[index].ready.defer(waiting);
                let mut waiting = Vec::new();
                while !failed && active_jobs < jobs {
                    let ready = executions
                        .iter()
                        .map(|execution| execution.ready.len() + execution.swift.ready.len())
                        .sum::<usize>()
                        .max(1);
                    let worker_jobs =
                        std::num::NonZeroUsize::new(((jobs - active_jobs) / ready).max(1)).unwrap();
                    let execution = &mut executions[index];
                    let Some(position) = execution.swift.ready.pop_front() else {
                        break;
                    };
                    let block = std::mem::take(&mut wait_for_lock) && active_jobs == 0;
                    match execution.swift.requests[position].1.prepare(block) {
                        Ok(PreparedSwiftBuild::Complete(product)) => {
                            execution.swift.completed.insert(position, *product);
                        }
                        Ok(PreparedSwiftBuild::WaitingForLock) => waiting.push(position),
                        Ok(PreparedSwiftBuild::Pending(pending)) => {
                            let sender = sender.clone();
                            let target = graph.native_target().clone();
                            let worker = std::thread::Builder::new()
                                .name(format!("scoop-swift-{index}-{position}"))
                                .spawn_scoped(scope, move || {
                                    let result = std::panic::catch_unwind(
                                        std::panic::AssertUnwindSafe(|| {
                                            target.production().and_then(|target| {
                                                pending.build(target, worker_jobs)
                                            })
                                        }),
                                    )
                                    .unwrap_or_else(|_| {
                                        Err(scoop_toolchain::ToolchainError::new(
                                            scoop_source::DiagnosticCode::ICE2201,
                                            "Swift build worker panicked".to_owned(),
                                        ))
                                    });
                                    let _ = sender.send((
                                        index,
                                        position,
                                        worker_jobs,
                                        Response::Swift(result.map(Box::new)),
                                    ));
                                });
                            match worker {
                                Ok(_) => active_jobs += worker_jobs.get(),
                                Err(source) => {
                                    execution.failures.insert(
                                        execution.order.len() + position,
                                        execution.swift.error(
                                            position,
                                            scoop_toolchain::ToolchainError::new(
                                                scoop_source::DiagnosticCode::E2201,
                                                format!(
                                                    "cannot start Swift build worker: {source}"
                                                ),
                                            ),
                                        ),
                                    );
                                    failed = true;
                                }
                            }
                        }
                        Err(source) => {
                            execution.failures.insert(
                                execution.order.len() + position,
                                execution.swift.error(position, source),
                            );
                            failed = true;
                        }
                    }
                }
                executions[index].swift.ready.extend(waiting);
            }
            if active_jobs == 0 {
                if failed
                    || executions.iter().all(|execution| {
                        execution.ready.len() == 0 && execution.swift.ready.is_empty()
                    })
                {
                    break;
                }
                // Only lock waiters remain. Block on one after every runnable
                // compiler and Swift node has had a chance to acquire a slot.
                wait_for_lock = true;
                continue;
            }
            wait_for_lock = false;
            // Drain started workers after failure before releasing the graph.
            let (graph, position, worker_jobs, response) = receiver
                .recv()
                .expect("an active compiler worker returns a response");
            receive(&mut executions[graph], graphs[graph], position, response);
            active_jobs -= worker_jobs.get();
        }
    });
    if let Some((_, cause)) = executions
        .iter_mut()
        .find_map(|execution| execution.failures.pop_first())
    {
        return Err(BuildGraphExecutionFailure::new(
            cause,
            &executions
                .iter()
                .flat_map(Execution::completed_refs)
                .collect::<Vec<_>>(),
        ));
    }
    debug_assert!(
        executions
            .iter()
            .all(
                |execution| execution.completed_refs().len() == execution.order.len()
                    && execution.swift.completed.len() == execution.swift.requests.len()
            )
    );
    Ok(executions)
}
