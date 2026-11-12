use super::*;

pub(super) fn declare_work_class(
    core: mir::ConeIdentity,
    task: mir::InterfaceId,
    completion: mir::InterfaceId,
    work: mir::InterfaceId,
    classes: &mut Arena<mir::ClassDef>,
    shell: &mut mir::Module,
) -> mir::ClassId {
    let context = mir::Type::Context(mir::ContextStorageType::new(
        core,
        mir::ContextStorageRole::Task,
    ));
    let definition = |fields| mir::ClassDef {
        release_policy: Default::default(),
        modifier: mir::ClassModifier::Final,
        name: "CoroutineStartWork".into(),
        type_arguments: Vec::new(),
        representation: mir::ClassRepresentation::Declared(mir::DeclaredClass {
            fields,
            base_class: None,
        }),
        interfaces: vec![work],
        vtable: Vec::new(),
        itables: Vec::new(),
    };
    let class = classes.alloc(definition(vec![
        mir::Field {
            name: "context".into(),
            ty: context,
        },
        mir::Field {
            name: "task".into(),
            ty: mir::Type::Interface(task),
        },
        mir::Field {
            name: "completion".into(),
            ty: mir::Type::Interface(completion),
        },
    ]));
    let mirror = shell.classes.alloc(definition(Vec::new()));
    assert_eq!(
        class, mirror,
        "the type context mirrors generated work classes"
    );
    class
}

pub(super) fn read_field(receiver: mir::Expr, index: u32, ty: mir::Type) -> mir::Expr {
    mir::Expr::new(
        ty,
        mir::ExprKind::FieldAccess {
            receiver: Box::new(receiver),
            index,
        },
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn create_entry(
    core: mir::ConeIdentity,
    result_name: &str,
    work_class: mir::ClassId,
    work_interface: mir::InterfaceId,
    start: ProtocolMethod,
    task: mir::InterfaceId,
    completion: mir::InterfaceId,
    functions: &mut Arena<mir::Function>,
) -> mir::FunctionId {
    let mut locals = Arena::new();
    let params = [
        ("execution", mir::Type::Interface(start.interface)),
        ("dispatched", mir::Type::Boolean),
        ("task", mir::Type::Interface(task)),
        ("completion", mir::Type::Interface(completion)),
    ]
    .map(|(name, ty)| {
        let local = locals.alloc(mir::Local {
            debug_variable: Default::default(),
            name: name.into(),
            ty: ty.clone(),
            mutable: false,
        });
        mir::Param {
            name: name.into(),
            ty,
            local,
        }
    });
    let param = |index: usize| mir::Expr::local(params[index].local, params[index].ty.clone());
    let (fork, context) = crate::task_context::fork_current(core, param(0), &mut locals);
    let work = locals.alloc(mir::Local {
        debug_variable: Default::default(),
        name: "$start_work".into(),
        ty: mir::Type::Class(work_class),
        mutable: false,
    });
    let mut statements = vec![fork];
    statements.extend(crate::coroutine::initialized_generated_class(
        work,
        work_class,
        vec![context, param(2), param(3)],
    ));
    let work_ty = mir::Type::Interface(work_interface);
    let work_value = mir::Expr::new(
        work_ty.clone(),
        mir::ExprKind::Retype {
            operand: Box::new(mir::Expr::local(work, mir::Type::Class(work_class))),
            ty: Box::new(work_ty),
        },
    );
    statements.push(mir::Statement {
        debug_origin: Default::default(),
        span: mir::SourceSpan::new(0, 0).expect("synthetic span is ordered"),
        kind: mir::StatementKind::Call(mir::CallEffect::Unit(mir::Call {
            target: start.call(param(0)),
            args: vec![work_value, param(1)],
        })),
    });
    let mut blocks = Arena::new();
    let entry = blocks.alloc(mir::BasicBlock {
        terminator_origin: Default::default(),
        name: "start".into(),
        statements,
        terminator: mir::Terminator::Return {
            value: mir::Expr::unit(),
        },
        unwind: None,
    });
    functions.alloc(mir::Function {
        debug_origin: Default::default(),
        gc_effect: mir::GcEffect::Managed,
        name: format!("startCoroutineExecution<{result_name}>"),
        params: params.into(),
        return_ty: mir::Type::Unit,
        body: mir::Body {
            exception_states: Default::default(),
            locals,
            blocks,
            entry,
            loop_header_polls: Vec::new(),
        },
    })
}
