use super::CurrentFunctionId;
use crate::{ExportHir, FunctionGenericity, FunctionId, FunctionKind, HirSourceFunctionIdentity};
use scoop_identity::{
    DeclarationName, DefinitionOriginSubject, ExactCallableSignature, NativeAppEntryIdentity,
    NativeAppEntryIdentityError, SourceSignatureFingerprint,
};
use std::fmt;

/// Unique, fully validated local entry of an native-app Cone.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalNativeAppEntry {
    identity: NativeAppEntryIdentity,
    local_function: CurrentFunctionId,
}

impl LocalNativeAppEntry {
    pub fn try_new(
        module: &ExportHir,
        function_id: FunctionId,
    ) -> Result<Self, LocalNativeAppEntryError> {
        let function = module
            .functions
            .iter()
            .find_map(|(id, function)| (id == function_id).then_some(function))
            .ok_or(LocalNativeAppEntryError::MissingFunction)?;
        if !module.top_level.contains(&function_id) {
            return Err(LocalNativeAppEntryError::NotTopLevel);
        }
        if function.method.is_some() {
            return Err(LocalNativeAppEntryError::HasReceiver);
        }

        if !matches!(function.genericity, FunctionGenericity::Plain) {
            return Err(LocalNativeAppEntryError::Generic);
        }
        if function.is_suspend {
            return Err(LocalNativeAppEntryError::Suspend);
        }
        if !matches!(function.kind, FunctionKind::User(_)) {
            return Err(LocalNativeAppEntryError::NotScoopDefined);
        }

        let exact_type = |ty| {
            module
                .type_identities
                .get(ty)
                .and_then(|identity| identity.exact())
                .map(|identity| identity.id())
                .ok_or(LocalNativeAppEntryError::MissingExactType)
        };
        let signature = ExactCallableSignature::new(
            scoop_identity::Effect::Ordinary,
            None,
            function
                .params
                .iter()
                .map(|parameter| exact_type(parameter.ty))
                .collect::<Result<_, _>>()?,
            exact_type(function.return_ty)?,
        );
        let identity = match &module.function_identities[function_id] {
            crate::HirFunctionIdentity::Source(identity) => {
                if identity.declaration().origin() != module.cone {
                    return Err(LocalNativeAppEntryError::NotCurrentCone);
                }
                if !identity.declaration().owners().owners().is_empty() {
                    return Err(LocalNativeAppEntryError::NotTopLevel);
                }
                if !matches!(
                    identity.declaration().name(),
                    DeclarationName::Named(name) if name.as_str() == "appInit"
                ) || function.name != "appInit"
                {
                    return Err(LocalNativeAppEntryError::NotAppInit);
                }
                let HirSourceFunctionIdentity::Plain(record) = identity else {
                    return Err(LocalNativeAppEntryError::Generic);
                };
                let declaration = record.id();
                let origin = module
                    .export_definition_origins
                    .get(DefinitionOriginSubject::Function(declaration))
                    .ok_or(LocalNativeAppEntryError::MissingDefinitionOrigin)?;
                if origin.origin().source().cone() != module.cone {
                    return Err(LocalNativeAppEntryError::DefinitionOriginConeMismatch);
                }

                NativeAppEntryIdentity::try_new(record, signature)
                    .map_err(LocalNativeAppEntryError::Identity)?
            }
            _ => return Err(LocalNativeAppEntryError::NotSourceFunction),
        };
        Ok(Self {
            identity,
            local_function: CurrentFunctionId(function_id),
        })
    }

    pub const fn declaration(&self) -> scoop_identity::PersistentFunctionId {
        self.identity.declaration()
    }

    pub const fn local_function(&self) -> CurrentFunctionId {
        self.local_function
    }

    pub const fn source_signature(&self) -> &ExactCallableSignature {
        self.identity.source_signature()
    }

    pub const fn source_signature_fingerprint(&self) -> SourceSignatureFingerprint {
        self.identity.source_signature_fingerprint()
    }

    pub const fn identity(&self) -> &NativeAppEntryIdentity {
        &self.identity
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalNativeAppEntryError {
    MissingFunction,
    NotTopLevel,
    HasReceiver,
    NotSourceFunction,
    NotCurrentCone,
    NotAppInit,
    Generic,
    Suspend,
    NotScoopDefined,
    MissingExactType,
    MissingDefinitionOrigin,
    DefinitionOriginConeMismatch,
    Identity(NativeAppEntryIdentityError),
}

impl fmt::Display for LocalNativeAppEntryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::MissingFunction => "the native-app entry does not belong to Export HIR",
            Self::NotTopLevel => "the native-app entry is not a top-level function",
            Self::HasReceiver => "the native-app entry has a receiver",
            Self::NotSourceFunction => "the native-app entry is not a source function",
            Self::NotCurrentCone => "the native-app entry does not belong to the current Cone",
            Self::NotAppInit => "the native-app entry is not named `appInit`",
            Self::Generic => "the native-app entry is generic",
            Self::Suspend => "the native-app entry is suspend",
            Self::NotScoopDefined => "the native-app entry has no ordinary Scoop body",
            Self::MissingExactType => "an native-app signature type has no exact identity",
            Self::MissingDefinitionOrigin => "the native-app entry has no definition origin",
            Self::DefinitionOriginConeMismatch => {
                "the native-app entry definition origin belongs to a different Cone"
            }
            Self::Identity(error) => return error.fmt(formatter),
        })
    }
}

impl std::error::Error for LocalNativeAppEntryError {}
