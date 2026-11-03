use super::*;

mod completion;
mod drive;
mod exits;

use completion::generate_completion_method;
use drive::generate_work_method;
use exits::drive_exit_blocks;

pub(super) struct GeneratedAdapter {
    pub(super) class: mir::ClassId,
    continuation: mir::InterfaceId,
    pub(super) resume: mir::FunctionId,
    pub(super) resume_with_exception: mir::FunctionId,
    pub(super) work_run: mir::FunctionId,
    pub(super) identity: mir::ContinuationAdapterIdentity,
}

impl GeneratedAdapter {
    pub(super) fn reference(&self, local: mir::LocalId) -> mir::Expr {
        let ty = mir::Type::Interface(self.continuation);
        mir::Expr::new(
            ty.clone(),
            mir::ExprKind::Retype {
                operand: Box::new(mir::Expr::local(local, mir::Type::Class(self.class))),
                ty: Box::new(ty),
            },
        )
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn generate_adapter(
    lowerer: &mut Lowerer,
    module: &hir::Module,
    frame_class: mir::ClassId,
    frame_layout: FrameLayout,
    destination: Option<FrameSlot>,
    failure_slot: FrameSlot,
    outer_step: &mir::Type,
    outer_continuation: mir::InterfaceId,
    outer_resume: &crate::coroutine_registry::ProtocolMethod,
    outer_failure: &crate::coroutine_registry::ProtocolMethod,
    source_name: &str,
    driver: mir::FunctionId,
    source: hir::CallableMaterialization,
    source_odr_group: Option<hir::OdrGroupId>,
    suspension_site: hir::StructuralDefinitionPath,
    state: mir::CoroutineSuspendStateId,
    result: &mir::Type,
    latches: (FrameSlot, FrameSlot),
    external: bool,
) -> GeneratedAdapter {
    let protocol = lowerer.coroutine_protocol(module, result);
    let continuation = lowerer.interfaces.mir_id(protocol.continuation);
    let work = lowerer.interfaces.mir_id(protocol.work);
    let execution_resume = crate::coroutine_registry::drive_protocol_call(
        module,
        &lowerer.instances,
        &lowerer.function_map,
        &lowerer.interfaces,
        &lowerer.external_callables,
        protocol.execution_resume,
        protocol.execution,
    );
    let name = format!("CoroutineAdapter<{source_name}>${state}");
    let fields = vec![
        mir::Field {
            name: "frame".to_string(),
            ty: mir::Type::Class(frame_class),
        },
        mir::Field {
            name: "status".to_string(),
            ty: mir::Type::MachineScalar(mir::MachineScalarKind::CoroutineAdapterState),
        },
        mir::Field {
            name: "result".to_string(),
            ty: latches.0.slot_ty.clone(),
        },
        mir::Field {
            name: "failure".to_string(),
            ty: latches.1.slot_ty.clone(),
        },
    ];
    let success_signature =
        crate::source_callables::exact_function_signature(module, protocol.continuation_resume);
    let failure_signature = crate::source_callables::exact_function_signature(
        module,
        protocol.continuation_resume_with_exception,
    );
    let work_signature = crate::coroutine_registry::drive_protocol_signature(
        module,
        protocol.work_run,
        protocol.work,
    );
    let identity = mir::ContinuationAdapterIdentity::new(
        source,
        suspension_site,
        success_signature,
        failure_signature,
        work_signature,
        source_odr_group,
    )
    .expect("a suspension site has one persistent continuation-adapter identity");
    let class = generated_class(lowerer, name, fields, vec![continuation, work], Vec::new());
    let resume = generate_completion_method(
        lowerer,
        module,
        class,
        frame_class,
        frame_layout,
        state,
        &latches.0,
        false,
        external,
        work,
        execution_resume,
    );
    let failure = generate_completion_method(
        lowerer,
        module,
        class,
        frame_class,
        frame_layout,
        state,
        &latches.1,
        true,
        external,
        work,
        execution_resume,
    );
    let work_run = generate_work_method(
        lowerer,
        module,
        class,
        frame_class,
        frame_layout,
        destination,
        &failure_slot,
        &latches,
        outer_step,
        outer_continuation,
        outer_resume,
        outer_failure,
        driver,
        state,
    );
    let mut slots = [
        (protocol.continuation_resume, resume),
        (protocol.continuation_resume_with_exception, failure),
    ]
    .map(|(source, function)| {
        let method = module.functions[source]
            .receiver
            .method()
            .expect("a continuation member has a receiver");
        let hir::MethodDispatch::Interface { slot, .. } = method.dispatch else {
            unreachable!("continuation methods retain interface slots")
        };
        (slot, mir::TableSlot::Function(function))
    });
    slots.sort_by_key(|(slot, _)| *slot);
    lowerer.classes[class].itables = vec![
        mir::ItableRecord {
            interface: continuation,
            slots: slots.into_iter().map(|(_, target)| target).collect(),
        },
        mir::ItableRecord {
            interface: work,
            slots: vec![mir::TableSlot::Function(work_run)],
        },
    ];
    GeneratedAdapter {
        class,
        continuation,
        resume,
        resume_with_exception: failure,
        work_run,
        identity,
    }
}
