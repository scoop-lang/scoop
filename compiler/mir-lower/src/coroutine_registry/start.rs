use super::*;

mod entry;
use entry::{create_entry, declare_work_class, read_field};

impl CoroutineRegistry {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn start_helper(
        &mut self,
        exact_types: &SourceExactTypeRegistry,
        core: mir::ConeIdentity,
        result: &mir::Type,
        task_interface: mir::InterfaceId,
        continuation_interface: mir::InterfaceId,
        work: crate::coroutine_registry::ProtocolMethod,
        execution_start: crate::coroutine_registry::ProtocolMethod,
        work_signature: hir::ExactCallableSignature,
        run: crate::coroutine_registry::ProtocolMethod,
        resume: crate::coroutine_registry::ProtocolMethod,
        resume_with_exception: crate::coroutine_registry::ProtocolMethod,
        step_ty: &mir::Type,
        throwable: mir::Type,
        functions: &mut Arena<mir::Function>,
        top_level: &mut Vec<mir::FunctionId>,
        classes: &mut Arena<mir::ClassDef>,
        shell: &mut mir::Module,
    ) -> mir::FunctionId {
        let source = Self::source_type(exact_types, result);
        let exact = source.identity_record();
        let nominal_group = source.nominal_specialization();
        let step_id = self
            .steps_by_result
            .iter()
            .find(|(found, _)| *found == exact.id())
            .map(|(_, id)| *id)
            .expect("start helpers are created after their CoroutineStep metadata");
        let step_metadata = &self.steps[step_id];
        assert_eq!(
            step_ty,
            &mir::Type::Enum(step_metadata.enum_id(), Vec::new()),
            "the start helper result and CoroutineStep type must identify the same metadata",
        );
        assert_eq!(
            self.step_metadata_for_type(step_ty).result(),
            result,
            "the start helper CoroutineStep metadata must carry its result type",
        );
        if let Some(found) = self
            .start_helpers
            .iter()
            .find(|found| found.identity().result_record().id() == exact.id())
        {
            return found.function();
        }
        let completed_variant = step_metadata.completed();
        let completed_payload = step_metadata.completed_payload();

        let work_class = declare_work_class(
            core,
            task_interface,
            continuation_interface,
            work.interface,
            classes,
            shell,
        );
        let mut locals = Arena::new();
        let receiver = locals.alloc(mir::Local {
            debug_variable: Default::default(),
            name: "this".into(),
            ty: mir::Type::Class(work_class),
            mutable: false,
        });
        let task = locals.alloc(mir::Local {
            debug_variable: Default::default(),
            name: "task".to_string(),
            ty: mir::Type::Interface(task_interface),
            mutable: false,
        });
        let completion = locals.alloc(mir::Local {
            debug_variable: Default::default(),
            name: "completion".to_string(),
            ty: mir::Type::Interface(continuation_interface),
            mutable: false,
        });
        let step = locals.alloc(mir::Local {
            debug_variable: Default::default(),
            name: "$step".to_string(),
            ty: step_ty.clone(),
            mutable: false,
        });
        let exception = locals.alloc(mir::Local {
            debug_variable: Default::default(),
            name: "$exception".to_string(),
            ty: throwable.clone(),
            mutable: false,
        });
        let span = mir::SourceSpan::new(0, 0).expect("synthetic span is ordered");
        let statement = |kind| mir::Statement {
            debug_origin: Default::default(),
            kind,
            span,
        };
        let mut blocks = Arena::new();
        let mut exception_states = Arena::new();
        let state = exception_states.alloc(mir::ExceptionState);
        let switch = crate::task_context::TaskSwitch::new(
            core,
            &mut locals,
            &mut blocks,
            &mut exception_states,
        );
        let suspended = blocks.alloc(mir::BasicBlock {
            terminator_origin: Default::default(),
            name: "suspended".to_string(),
            statements: vec![switch.leave()],
            terminator: mir::Terminator::Return {
                value: mir::Expr::unit(),
            },
            unwind: None,
        });
        let completed = blocks.alloc(mir::BasicBlock {
            terminator_origin: Default::default(),
            name: "completed".to_string(),
            statements: vec![statement(mir::StatementKind::Call(mir::CallEffect::Unit(
                mir::Call {
                    target: resume.call(mir::Expr::local(
                        completion,
                        mir::Type::Interface(continuation_interface),
                    )),
                    args: vec![mir::Expr::new(
                        result.clone(),
                        mir::ExprKind::EnumField {
                            operand: Box::new(mir::Expr::local(step, step_ty.clone())),
                            variant: completed_payload.variant().variant_index(),
                            index: completed_payload.field_index(),
                        },
                    )],
                },
            )))],
            terminator: mir::Terminator::Return {
                value: mir::Expr::unit(),
            },
            unwind: Some(switch.unwind),
        });
        let failed = blocks.alloc(mir::BasicBlock {
            terminator_origin: Default::default(),
            name: "failed".to_string(),
            statements: vec![statement(mir::StatementKind::Call(mir::CallEffect::Unit(
                mir::Call {
                    target: resume_with_exception.call(mir::Expr::local(
                        completion,
                        mir::Type::Interface(continuation_interface),
                    )),
                    args: vec![mir::Expr::local(exception, throwable.clone())],
                },
            )))],
            terminator: mir::Terminator::Return {
                value: mir::Expr::unit(),
            },
            unwind: Some(switch.unwind),
        });
        let catch_pad = blocks.alloc(mir::BasicBlock {
            terminator_origin: Default::default(),
            name: "body_failure".to_string(),
            statements: vec![
                statement(mir::StatementKind::Eh(mir::EhStatement::LandingPad {
                    cleanup: false,
                    state,
                })),
                statement(mir::StatementKind::Eh(mir::EhStatement::BeginCatch {
                    state,
                })),
                statement(mir::StatementKind::ValDecl {
                    local: exception,
                    init: mir::Expr::new(
                        throwable.clone(),
                        mir::ExprKind::CaughtException { state },
                    ),
                }),
                statement(mir::StatementKind::Eh(mir::EhStatement::EndCatch)),
            ],
            terminator: mir::Terminator::Goto(failed),
            unwind: Some(switch.catch_unwind),
        });
        let run_body = blocks.alloc(mir::BasicBlock {
            terminator_origin: Default::default(),
            name: "run".to_string(),
            statements: vec![statement(mir::StatementKind::Call(
                mir::CallEffect::Value {
                    destination: step,
                    call: mir::Call {
                        target: run
                            .call(mir::Expr::local(task, mir::Type::Interface(task_interface))),
                        args: vec![mir::Expr::local(
                            completion,
                            mir::Type::Interface(continuation_interface),
                        )],
                    },
                },
            ))],
            terminator: mir::Terminator::Branch {
                cond: mir::Expr::machine_eq(
                    mir::Expr::enum_tag(mir::Expr::local(step, step_ty.clone())),
                    mir::MachineScalarValue::EnumTag(completed_variant.variant_index()),
                ),
                then_block: completed,
                else_block: suspended,
            },
            unwind: Some(catch_pad),
        });
        blocks[completed].statements.push(switch.leave());
        blocks[failed].statements.push(switch.leave());
        let context_ty = mir::Type::Context(mir::ContextStorageType::new(
            core,
            mir::ContextStorageRole::Task,
        ));
        let receiver_value = mir::Expr::local(receiver, mir::Type::Class(work_class));
        let entry = blocks.alloc(mir::BasicBlock {
            terminator_origin: Default::default(),
            name: "entry".to_string(),
            statements: vec![
                statement(mir::StatementKind::ValDecl {
                    local: task,
                    init: read_field(
                        receiver_value.clone(),
                        1,
                        mir::Type::Interface(task_interface),
                    ),
                }),
                statement(mir::StatementKind::ValDecl {
                    local: completion,
                    init: read_field(
                        receiver_value.clone(),
                        2,
                        mir::Type::Interface(continuation_interface),
                    ),
                }),
                switch.enter(read_field(receiver_value, 0, context_ty)),
            ],
            terminator: mir::Terminator::Goto(run_body),
            unwind: None,
        });
        let result_name = mir::type_name(shell, result);
        let work_run = functions.alloc(mir::Function {
            debug_origin: Default::default(),
            gc_effect: mir::GcEffect::Managed,
            name: format!("CoroutineStartWork<{result_name}>.run"),
            params: vec![mir::Param {
                name: "this".into(),
                ty: mir::Type::Class(work_class),
                local: receiver,
            }],
            return_ty: mir::Type::Unit,
            body: mir::Body {
                exception_states,
                locals,
                blocks,
                entry,
                loop_header_polls: Vec::new(),
            },
        });
        top_level.push(work_run);
        let mir::ClassRepresentation::Declared(_) = &classes[work_class].representation else {
            unreachable!("start work is a declared generated class")
        };
        classes[work_class].itables = vec![mir::ItableRecord {
            interface: work.interface,
            slots: vec![mir::TableSlot::Function(work_run)],
        }];
        assert_eq!(
            work.slot, 0,
            "CoroutineWork has exactly its checked run method"
        );
        let function = create_entry(
            core,
            &result_name,
            work_class,
            work.interface,
            execution_start,
            task_interface,
            continuation_interface,
            functions,
        );
        top_level.push(function);
        let signature = hir::ExactCallableSignature::new(
            hir::Effect::Ordinary,
            None,
            [
                mir::Type::Interface(execution_start.interface),
                mir::Type::Boolean,
                mir::Type::Interface(task_interface),
                mir::Type::Interface(continuation_interface),
            ]
            .map(|ty| {
                exact_types
                    .get(&ty)
                    .expect("a concrete coroutine protocol interface has one exact identity")
                    .identity_record()
                    .id()
            })
            .into(),
            exact_types
                .get(&mir::Type::Unit)
                .expect("the core Unit type has one exact identity")
                .identity_record()
                .id(),
        );
        let identity = mir::CoroutineStartIdentity::new(exact, nominal_group, signature)
            .expect("local-concrete coroutine protocols have one coroutine-start identity");
        let work_identity =
            mir::CoroutineStartWorkIdentity::new(exact, nominal_group, work_signature)
                .expect("start work has an exact owner and a checked CoroutineWork signature");
        let metadata = mir::CoroutineStart::checked(
            functions,
            result.clone(),
            function,
            identity,
            work_class,
            work_run,
            work_identity,
        )
        .expect("generated coroutine start helper has the exact erased signature");
        self.start_helpers.push(metadata);
        function
    }
}
