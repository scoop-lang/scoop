//! Record locations are separate from the actual coroutine protocol identities.

use super::*;

pub(super) struct Declarations {
    pub work: export::SourceNominalId,
    pub work_run: DriveSource,
    pub execution: export::SourceNominalId,
    pub execution_start: DriveSource,
    pub execution_resume: DriveSource,

    pub continuation: export::SourceNominalId,
    pub suspend_task: export::SourceNominalId,
    pub suspend_registration: export::SourceNominalId,
    pub start_coroutine: FunctionSource,
    pub suspend_coroutine: FunctionSource,
    pub continuation_resume: FunctionSource,
    pub continuation_resume_with_exception: FunctionSource,
    pub suspend_task_run: FunctionSource,
    pub suspend_registration_register: FunctionSource,
}

#[derive(Clone, Copy)]
pub(super) enum DriveSource {
    Local(export::FunctionId),
    Imported(export::ImportedGenericCallableTemplateId),
}

impl Concretizer<'_> {
    pub(super) fn coroutine_declarations(&self) -> Declarations {
        match self.core {
            export::CoreProtocols::Defined(core) => {
                let core = core.coroutines;
                Declarations {
                    work: self.source.nominal_identities[core.work].declaration_id(),
                    work_run: DriveSource::Local(core.work_run),
                    execution: self.source.nominal_identities[core.execution].declaration_id(),
                    execution_start: DriveSource::Local(core.execution_start),
                    execution_resume: DriveSource::Local(core.execution_resume),
                    continuation: self.source.nominal_identities[core.continuation]
                        .declaration_id(),
                    suspend_task: self.source.nominal_identities[core.suspend_task]
                        .declaration_id(),
                    suspend_registration: self.source.nominal_identities[core.suspend_registration]
                        .declaration_id(),
                    start_coroutine: FunctionSource::Local(core.start_coroutine),
                    suspend_coroutine: FunctionSource::Local(core.suspend_coroutine),
                    continuation_resume: FunctionSource::Local(core.continuation_resume),
                    continuation_resume_with_exception: FunctionSource::Local(
                        core.continuation_resume_with_exception,
                    ),
                    suspend_task_run: FunctionSource::Local(core.suspend_task_run),
                    suspend_registration_register: FunctionSource::Local(
                        core.suspend_registration_register,
                    ),
                }
            }
            export::CoreProtocols::Imported(core) => {
                let core = core.coroutines();
                Declarations {
                    work: export::SourceNominalId::Concrete(core.work().persistent()),
                    work_run: self.imported_drive_method(core.work_run()),
                    execution: export::SourceNominalId::Concrete(core.execution().persistent()),
                    execution_start: self.imported_drive_method(core.execution_start()),
                    execution_resume: self.imported_drive_method(core.execution_resume()),
                    continuation: export::SourceNominalId::GenericTemplate(
                        core.continuation().persistent(),
                    ),
                    suspend_task: export::SourceNominalId::GenericTemplate(
                        core.suspend_task().persistent(),
                    ),
                    suspend_registration: export::SourceNominalId::GenericTemplate(
                        core.suspend_registration().persistent(),
                    ),
                    start_coroutine: self.imported_coroutine_function(core.start_coroutine()),
                    suspend_coroutine: self.imported_coroutine_function(core.suspend_coroutine()),
                    continuation_resume: self
                        .imported_coroutine_function(core.continuation_resume()),
                    continuation_resume_with_exception: self
                        .imported_coroutine_function(core.continuation_resume_with_exception()),
                    suspend_task_run: self.imported_coroutine_function(core.suspend_task_run()),
                    suspend_registration_register: self
                        .imported_coroutine_function(core.suspend_registration_register()),
                }
            }
        }
    }

    fn imported_drive_method(
        &self,
        callable: &export::ImportedCoreProtocolCallable,
    ) -> DriveSource {
        let FunctionSource::Imported(template) = self.imported_coroutine_function(callable) else {
            unreachable!("a dependency protocol has an imported declaration")
        };
        DriveSource::Imported(template)
    }

    pub(super) fn request_drive_method(
        &mut self,
        source: DriveSource,
        owner: concrete::InterfaceId,
    ) -> concrete::CoroutineDispatch {
        match source {
            DriveSource::Local(function) => {
                let source = FunctionSource::Local(function);
                let key = self.function_key(
                    source,
                    Some(concrete::MethodOwner::Interface(owner)),
                    Vec::new(),
                );
                concrete::CoroutineDispatch::Local(self.request_function_key(key, source))
            }
            DriveSource::Imported(template) => {
                let source = &self.source.imported_generic_templates[template];
                let export::DefaultCallableDeclarationV1::Function(function) =
                    source.declaration.body_owner()
                else {
                    unreachable!("drive methods are nongeneric source functions")
                };
                let export::ImportedCallableTemplateOrigin::Nominal {
                    dispatch: export::DeclaredMethodDispatch::Interface(slot),
                    ..
                } = source.declaration
                else {
                    unreachable!("a drive method retains its interface slot")
                };
                concrete::CoroutineDispatch::Imported {
                    function,
                    slot: self.interface_slot_by_source[&(owner, slot)],
                }
            }
        }
    }

    fn imported_coroutine_function(
        &self,
        callable: &export::ImportedCoreProtocolCallable,
    ) -> FunctionSource {
        let declaration = match callable.definition() {
            export::ImportedCoreProtocolCallableDefinition::Function(id) => {
                export::DefaultCallableDeclarationV1::Function(id.persistent())
            }
            export::ImportedCoreProtocolCallableDefinition::GenericFunction(id) => {
                export::DefaultCallableDeclarationV1::GenericFunction(id.persistent())
            }
            _ => unreachable!("coroutine roles name source functions"),
        };
        let source = self
            .source
            .imported_generic_templates
            .iter()
            .find_map(|(id, template)| {
                (template.declaration.body_owner() == declaration).then_some(id)
            })
            .expect("Export HIR retains every coroutine protocol signature");
        FunctionSource::Imported(source)
    }

    pub(super) fn request_protocol_function(
        &mut self,
        source: FunctionSource,
        owner: Option<concrete::MethodOwner>,
        result: concrete::TypeId,
    ) -> concrete::FunctionId {
        let key = self.function_key(source, owner, vec![result]);
        self.request_function_key(key, source)
    }
}
