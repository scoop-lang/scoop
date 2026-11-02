//! Closed HIR products for one current Cone.

use std::fmt;
use std::ops::Deref;

use scoop_identity::{
    CallableMaterializationContext, ExecutableEntryIdentity, NativeAppEntryIdentity,
};

use crate::{
    ConeOutputKind, CoreProtocols, ExportHir, HirExportBindingSurfaceValidationError,
    HirNativeBoundaryTypeDefinitions, LocalConcreteHir, LocalExecutableEntry,
    LocalExecutableEntryError, LocalNativeAppEntry, LocalNativeAppEntryError, concrete,
};

mod dependencies;
pub use dependencies::*;
mod source;
pub use source::DependencyExportHirOutput;

/// Export HIR paired with its validated library/executable contract.
#[derive(Debug, Clone)]
pub struct ExportHirOutput {
    module: ExportHir,
    output_kind: ConeOutputKind,
    shared_source: std::sync::Arc<crate::production::ExportSharedSource>,
}

impl ExportHirOutput {
    pub fn try_new(
        module: ExportHir,
        output_kind: ConeOutputKind,
    ) -> Result<Self, ExportHirOutputError> {
        Self::with_dependencies(module, output_kind, None)
    }

    pub fn try_new_with_dependencies(
        module: ExportHir,
        output_kind: ConeOutputKind,
        dependencies: &crate::SelectedImportedDependencySet,
    ) -> Result<Self, ExportHirOutputError> {
        Self::with_dependencies(module, output_kind, Some(dependencies))
    }

    fn with_dependencies(
        module: ExportHir,
        output_kind: ConeOutputKind,
        dependencies: Option<&crate::SelectedImportedDependencySet>,
    ) -> Result<Self, ExportHirOutputError> {
        module
            .export_binding_identities
            .validate_public_bindings(&module.public_export_bindings)
            .map_err(ExportHirOutputError::PublicBindings)?;
        if let ConeOutputKind::Executable { local_entry } = &output_kind {
            let expected =
                LocalExecutableEntry::try_new(&module, local_entry.local_function().function())
                    .map_err(ExportHirOutputError::InvalidEntry)?;
            if &expected != local_entry.as_ref() {
                return Err(ExportHirOutputError::EntryMismatch);
            }
        }
        if let ConeOutputKind::NativeApp { local_entry } = &output_kind {
            let expected =
                LocalNativeAppEntry::try_new(&module, local_entry.local_function().function())
                    .map_err(ExportHirOutputError::InvalidNativeAppEntry)?;
            if &expected != local_entry.as_ref() {
                return Err(ExportHirOutputError::EntryMismatch);
            }
        }
        let shared_source =
            crate::production::ExportSharedSource::from_export(&module, dependencies)
                .map_err(|error| ExportHirOutputError::Templates(Box::new(error)))?;
        Ok(Self {
            module,
            output_kind,
            shared_source: std::sync::Arc::new(shared_source),
        })
    }

    pub fn shared_source(&self) -> &std::sync::Arc<crate::ExportSharedSource> {
        &self.shared_source
    }

    pub fn set_shared_source(&mut self, source: std::sync::Arc<crate::ExportSharedSource>) {
        self.shared_source = source;
    }

    pub const fn module(&self) -> &ExportHir {
        &self.module
    }

    pub const fn output_kind(&self) -> &ConeOutputKind {
        &self.output_kind
    }

    pub fn into_module(self) -> ExportHir {
        self.module
    }
}

impl Deref for ExportHirOutput {
    type Target = ExportHir;

    fn deref(&self) -> &Self::Target {
        self.module()
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CurrentConcreteFunctionId(concrete::FunctionId);

impl CurrentConcreteFunctionId {
    pub const fn function(self) -> concrete::FunctionId {
        self.0
    }
}

/// Local-concrete projection of the validated executable entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConcreteExecutableEntry {
    identity: ExecutableEntryIdentity,
    local_function: CurrentConcreteFunctionId,
}

impl ConcreteExecutableEntry {
    pub fn try_new(
        module: &LocalConcreteHir,
        source: &LocalExecutableEntry,
        function_id: concrete::FunctionId,
    ) -> Result<Self, ConcreteEntryError> {
        if source.identity().root_cone() != module.cone {
            return Err(ConcreteEntryError::NotCurrentCone);
        }
        validate_concrete_entry(
            module,
            source.declaration().template(),
            source.source_signature(),
            function_id,
        )?;
        Ok(Self {
            identity: source.identity().clone(),
            local_function: CurrentConcreteFunctionId(function_id),
        })
    }

    fn validate(&self, module: &LocalConcreteHir) -> Result<(), ConcreteEntryError> {
        if self.identity.root_cone() != module.cone {
            return Err(ConcreteEntryError::NotCurrentCone);
        }
        validate_concrete_entry(
            module,
            self.declaration().template(),
            self.identity.source_signature(),
            self.local_function.function(),
        )
    }

    pub const fn identity(&self) -> &ExecutableEntryIdentity {
        &self.identity
    }

    pub const fn declaration(&self) -> scoop_identity::ExecutableEntryDeclaration {
        self.identity.declaration()
    }

    pub const fn local_function(&self) -> CurrentConcreteFunctionId {
        self.local_function
    }
}

/// Local-concrete projection of the validated native-app entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConcreteNativeAppEntry {
    identity: NativeAppEntryIdentity,
    local_function: CurrentConcreteFunctionId,
}

impl ConcreteNativeAppEntry {
    pub fn try_new(
        module: &LocalConcreteHir,
        source: &LocalNativeAppEntry,
        function_id: concrete::FunctionId,
    ) -> Result<Self, ConcreteEntryError> {
        if source.identity().root_cone() != module.cone {
            return Err(ConcreteEntryError::NotCurrentCone);
        }
        validate_concrete_entry(
            module,
            scoop_identity::CallableTemplateOwner::Function(source.declaration()),
            source.source_signature(),
            function_id,
        )?;
        Ok(Self {
            identity: source.identity().clone(),
            local_function: CurrentConcreteFunctionId(function_id),
        })
    }

    fn validate(&self, module: &LocalConcreteHir) -> Result<(), ConcreteEntryError> {
        if self.identity.root_cone() != module.cone {
            return Err(ConcreteEntryError::NotCurrentCone);
        }
        validate_concrete_entry(
            module,
            scoop_identity::CallableTemplateOwner::Function(self.declaration()),
            self.identity.source_signature(),
            self.local_function.function(),
        )
    }

    pub const fn identity(&self) -> &NativeAppEntryIdentity {
        &self.identity
    }

    pub const fn declaration(&self) -> scoop_identity::PersistentFunctionId {
        self.identity.declaration()
    }

    pub const fn local_function(&self) -> CurrentConcreteFunctionId {
        self.local_function
    }
}

/// Output branch translated into the LocalConcrete HIR id domain.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LocalConeOutputKind {
    Library,
    Executable {
        local_entry: Box<ConcreteExecutableEntry>,
    },
    NativeApp {
        local_entry: Box<ConcreteNativeAppEntry>,
    },
}

mod shape_support;
pub use shape_support::*;

mod materialized_types;
pub use materialized_types::MaterializedTypeClosureError;

/// LocalConcrete HIR paired with its closed output branch.
#[derive(Debug, Clone)]
pub struct LocalConcreteHirOutput {
    module: LocalConcreteHir,
    output_kind: LocalConeOutputKind,
    materialization: LocalShapeSupportPlan,
}

impl LocalConcreteHirOutput {
    pub fn try_new(
        module: LocalConcreteHir,
        output_kind: LocalConeOutputKind,
        materialization: LocalShapeSupportPlan,
    ) -> Result<Self, LocalConcreteHirOutputError> {
        if let LocalConeOutputKind::Executable { local_entry } = &output_kind {
            local_entry
                .validate(&module)
                .map_err(LocalConcreteHirOutputError::InvalidEntry)?;
        }
        if let LocalConeOutputKind::NativeApp { local_entry } = &output_kind {
            local_entry
                .validate(&module)
                .map_err(LocalConcreteHirOutputError::InvalidEntry)?;
        }
        Ok(Self {
            module,
            output_kind,
            materialization,
        })
    }

    pub const fn module(&self) -> &LocalConcreteHir {
        &self.module
    }

    pub const fn output_kind(&self) -> &LocalConeOutputKind {
        &self.output_kind
    }

    pub const fn materialization(&self) -> &LocalShapeSupportPlan {
        &self.materialization
    }

    pub fn into_module(self) -> LocalConcreteHir {
        self.module
    }
}

impl Deref for LocalConcreteHirOutput {
    type Target = LocalConcreteHir;

    fn deref(&self) -> &Self::Target {
        self.module()
    }
}

impl crate::Output {
    pub fn try_new(
        export: ExportHirOutput,
        local: LocalConcreteHirOutput,
        native_boundary_types: HirNativeBoundaryTypeDefinitions,
        warnings: Vec<scoop_ast::Diagnostic>,
    ) -> Result<Self, HirOutputError> {
        if !matches!(
            (
                &export.module().core_protocols,
                &local.module().core_protocols
            ),
            (
                CoreProtocols::Defined(_),
                concrete::ConcreteCoreProtocols::Defined(_)
            ) | (
                CoreProtocols::Imported(_),
                concrete::ConcreteCoreProtocols::Imported(_)
            )
        ) {
            return Err(HirOutputError::CoreProtocolBranchMismatch);
        }
        match (export.output_kind(), local.output_kind()) {
            (ConeOutputKind::Library, LocalConeOutputKind::Library) => {}
            (
                ConeOutputKind::Executable {
                    local_entry: export_entry,
                },
                LocalConeOutputKind::Executable {
                    local_entry: concrete_entry,
                },
            ) if export_entry.identity() == concrete_entry.identity() => {}
            (
                ConeOutputKind::NativeApp {
                    local_entry: export_entry,
                },
                LocalConeOutputKind::NativeApp {
                    local_entry: concrete_entry,
                },
            ) if export_entry.identity() == concrete_entry.identity() => {}
            (ConeOutputKind::NativeApp { .. }, LocalConeOutputKind::NativeApp { .. }) => {
                return Err(HirOutputError::EntryMismatch);
            }
            (ConeOutputKind::Executable { .. }, LocalConeOutputKind::Executable { .. }) => {
                return Err(HirOutputError::EntryMismatch);
            }
            _ => return Err(HirOutputError::OutputKindMismatch),
        }
        Ok(Self {
            export,
            local,
            native_boundary_types,
            warnings,
            documentation: None,
        })
    }

    pub const fn output_kind(&self) -> &ConeOutputKind {
        self.export.output_kind()
    }
}

fn validate_concrete_entry(
    module: &LocalConcreteHir,
    declaration: scoop_identity::CallableTemplateOwner,
    source_signature: &scoop_identity::ExactCallableSignature,
    function_id: concrete::FunctionId,
) -> Result<(), ConcreteEntryError> {
    let function = module
        .functions
        .iter()
        .find_map(|(id, function)| (id == function_id).then_some(function))
        .ok_or(ConcreteEntryError::MissingFunction)?;
    if !module.top_level.contains(&function_id) {
        return Err(ConcreteEntryError::NotTopLevel);
    }
    if function.receiver.value_type().is_some() {
        return Err(ConcreteEntryError::HasReceiver);
    }
    if function.materialization.template() != declaration {
        return Err(ConcreteEntryError::DeclarationMismatch);
    }
    if function.materialization.context() != CallableMaterializationContext::NoSubstitution {
        return Err(ConcreteEntryError::Generic);
    }
    if function.is_suspend {
        return Err(ConcreteEntryError::Suspend);
    }
    let exact_type = |ty| {
        module
            .exact_type_identities
            .get(ty)
            .map(|identity| identity.id())
            .ok_or(ConcreteEntryError::MissingExactType)
    };
    let signature = scoop_identity::ExactCallableSignature::new(
        scoop_identity::Effect::Ordinary,
        None,
        function
            .params
            .iter()
            .map(|parameter| exact_type(parameter.ty))
            .collect::<Result<_, _>>()?,
        exact_type(function.return_ty)?,
    );
    if &signature != source_signature {
        return Err(ConcreteEntryError::SignatureMismatch);
    }
    if !matches!(function.kind, concrete::FunctionKind::User(_)) {
        return Err(ConcreteEntryError::NotScoopDefined);
    }
    Ok(())
}

#[derive(Debug)]
pub enum ExportHirOutputError {
    PublicBindings(HirExportBindingSurfaceValidationError),
    InvalidEntry(LocalExecutableEntryError),
    InvalidNativeAppEntry(LocalNativeAppEntryError),
    EntryMismatch,
    Templates(Box<crate::GenericTemplateProductionError>),
}

impl fmt::Display for ExportHirOutputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PublicBindings(error) => error.fmt(formatter),
            Self::InvalidNativeAppEntry(error) => error.fmt(formatter),
            Self::InvalidEntry(error) => error.fmt(formatter),
            Self::Templates(error) => error.fmt(formatter),
            Self::EntryMismatch => formatter
                .write_str("the executable entry identity does not match its Export HIR function"),
        }
    }
}

impl std::error::Error for ExportHirOutputError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConcreteEntryError {
    MissingFunction,
    NotTopLevel,
    HasReceiver,
    NotCurrentCone,
    DeclarationMismatch,
    Generic,
    Suspend,
    NotScoopDefined,
    MissingExactType,
    SignatureMismatch,
}

impl fmt::Display for ConcreteEntryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::MissingFunction => "the executable entry does not belong to LocalConcrete HIR",
            Self::NotTopLevel => "the executable entry is not a top-level concrete function",
            Self::HasReceiver => "the executable entry has a concrete receiver",
            Self::NotCurrentCone => "the executable entry does not belong to the current Cone",
            Self::DeclarationMismatch => {
                "the concrete entry does not materialize the selected source declaration"
            }
            Self::Generic => "the concrete entry has a substitution context",
            Self::Suspend => "the concrete entry is suspend",
            Self::NotScoopDefined => "the concrete entry has no ordinary Scoop body",
            Self::MissingExactType => "a concrete entry signature type has no exact identity",
            Self::SignatureMismatch => {
                "the concrete entry signature differs from the executable source signature"
            }
        })
    }
}

impl std::error::Error for ConcreteEntryError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalConcreteHirOutputError {
    InvalidEntry(ConcreteEntryError),
}

impl fmt::Display for LocalConcreteHirOutputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidEntry(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for LocalConcreteHirOutputError {}

#[derive(Debug)]
pub enum HirOutputError {
    OutputKindMismatch,
    EntryMismatch,
    CoreProtocolBranchMismatch,
}

impl fmt::Display for HirOutputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::OutputKindMismatch => "Export HIR and LocalConcrete HIR output kinds disagree",
            Self::EntryMismatch => {
                "Export HIR and LocalConcrete HIR executable entry identities disagree"
            }
            Self::CoreProtocolBranchMismatch => {
                "Export HIR and LocalConcrete HIR compiler-protocol branches disagree"
            }
        })
    }
}

impl std::error::Error for HirOutputError {}
