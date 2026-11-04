//! Persistent identities for continuation adapters generated at suspension sites.

use std::fmt;

use scoop_identity::{
    CallableMaterialization, CallableMaterializationContext, CallableOdrMemberId, CallableOwner,
    CallableTemplateOwner, CborIdentityRecord, CoroutineAdapterRole, Effect,
    ExactCallableSignature, FieldIdentityError, FieldIdentityKey, GeneratedCallableIdentityError,
    GeneratedCallableKey, GeneratedNominalIdentityError, GeneratedNominalKey, OdrGroupId,
    OdrMemberDiscriminator, OdrMemberId, OdrMemberIdentityError, OdrMemberKey, OdrMemberRole,
    PersistentFieldId, PersistentGeneratedCallableId, PersistentTypeId, StructuralDefinitionPath,
};
use scoop_wire::HashError;

use crate::{CallableSignatureRecord, CallableSignatureSubject};

type GeneratedCallableRecord =
    CborIdentityRecord<PersistentGeneratedCallableId, GeneratedCallableKey>;
type GeneratedTypeRecord = CborIdentityRecord<PersistentTypeId, GeneratedNominalKey>;
type FieldRecord = CborIdentityRecord<PersistentFieldId, FieldIdentityKey>;
type OdrMemberRecord = CborIdentityRecord<OdrMemberId, OdrMemberKey>;

/// Persistent field identities whose presence is fixed by the adapter's
/// concrete continuation protocol.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContinuationAdapterStorageIdentity {
    result: FieldRecord,
    failure: FieldRecord,
}

impl ContinuationAdapterStorageIdentity {
    pub const fn result_field_record(&self) -> &FieldRecord {
        &self.result
    }
    pub const fn failure_field_record(&self) -> &FieldRecord {
        &self.failure
    }
}

/// One generated success or failure callback at a suspension site.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoroutineAdapterCallableIdentity {
    callable: GeneratedCallableRecord,
    odr_member: Option<OdrMemberRecord>,
    signature: CallableSignatureRecord,
}

impl CoroutineAdapterCallableIdentity {
    pub const fn callable_record(&self) -> &GeneratedCallableRecord {
        &self.callable
    }

    pub const fn odr_member_record(&self) -> Option<&OdrMemberRecord> {
        self.odr_member.as_ref()
    }

    pub const fn signature_record(&self) -> &CallableSignatureRecord {
        &self.signature
    }

    pub const fn materialization(&self) -> CallableMaterialization {
        CallableMaterialization::new(
            CallableTemplateOwner::Generated(self.callable.id()),
            CallableMaterializationContext::NoSubstitution,
        )
    }
}

/// Complete persistent identity projection of one continuation adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContinuationAdapterIdentity {
    source: CallableMaterialization,
    suspension_site: StructuralDefinitionPath,
    generated_type: GeneratedTypeRecord,
    frame: FieldRecord,
    state: FieldRecord,
    storage: ContinuationAdapterStorageIdentity,
    success: CoroutineAdapterCallableIdentity,
    failure: CoroutineAdapterCallableIdentity,
    work: CoroutineAdapterCallableIdentity,
    odr_member: Option<OdrMemberRecord>,
}

impl ContinuationAdapterIdentity {
    pub fn new(
        source: CallableMaterialization,
        suspension_site: StructuralDefinitionPath,
        success_signature: ExactCallableSignature,
        failure_signature: ExactCallableSignature,
        work_signature: ExactCallableSignature,
        odr_group: Option<OdrGroupId>,
    ) -> Result<Self, ContinuationAdapterIdentityError> {
        if success_signature.effect() != Effect::Ordinary
            || !success_signature.receiver().is_present()
            || success_signature.parameters().len() != 1
        {
            return Err(ContinuationAdapterIdentityError::InvalidSuccessSignature);
        }
        if failure_signature.effect() != Effect::Ordinary
            || failure_signature.receiver() != success_signature.receiver()
            || failure_signature.parameters().len() != 1
            || failure_signature.result() != success_signature.result()
        {
            return Err(ContinuationAdapterIdentityError::InvalidFailureSignature);
        }
        if work_signature.effect() != Effect::Ordinary
            || !work_signature.receiver().is_present()
            || !work_signature.parameters().is_empty()
            || work_signature.result() != success_signature.result()
        {
            return Err(ContinuationAdapterIdentityError::InvalidWorkSignature);
        }
        if source.context() != CallableMaterializationContext::NoSubstitution && odr_group.is_none()
        {
            return Err(ContinuationAdapterIdentityError::MissingOdrGroup);
        }
        let generated_type =
            CborIdentityRecord::from_key(GeneratedNominalKey::ContinuationAdapterEnvironment {
                source_callable: source,
                suspension_site: suspension_site.clone(),
            })
            .map_err(ContinuationAdapterIdentityError::GeneratedType)?;
        let frame = field(FieldIdentityKey::coroutine_adapter_frame(
            generated_type.key(),
        ))?;
        let state = field(FieldIdentityKey::coroutine_adapter_state(
            generated_type.key(),
        ))?;
        let storage = ContinuationAdapterStorageIdentity {
            result: field(FieldIdentityKey::coroutine_adapter_result(
                generated_type.key(),
            ))?,
            failure: field(FieldIdentityKey::coroutine_adapter_failure(
                generated_type.key(),
            ))?,
        };
        let success = callable(
            source,
            suspension_site.clone(),
            CoroutineAdapterRole::Success,
            success_signature,
            odr_group,
        )?;
        let failure = callable(
            source,
            suspension_site.clone(),
            CoroutineAdapterRole::Failure,
            failure_signature,
            odr_group,
        )?;
        let work = callable(
            source,
            suspension_site.clone(),
            CoroutineAdapterRole::Run,
            work_signature,
            odr_group,
        )?;
        let odr_member = odr_group
            .map(|group| {
                member(
                    group,
                    OdrMemberRole::GeneratedNominal,
                    OdrMemberDiscriminator::GeneratedNominal(generated_type.id()),
                )
            })
            .transpose()?;
        Ok(Self {
            source,
            suspension_site,
            generated_type,
            frame,
            state,
            storage,
            success,
            failure,
            work,
            odr_member,
        })
    }

    pub const fn source(&self) -> CallableMaterialization {
        self.source
    }

    pub const fn suspension_site(&self) -> &StructuralDefinitionPath {
        &self.suspension_site
    }

    pub const fn generated_type_record(&self) -> &GeneratedTypeRecord {
        &self.generated_type
    }

    pub const fn frame_field_record(&self) -> &FieldRecord {
        &self.frame
    }

    pub const fn state_field_record(&self) -> &FieldRecord {
        &self.state
    }

    pub const fn storage(&self) -> &ContinuationAdapterStorageIdentity {
        &self.storage
    }

    pub const fn success(&self) -> &CoroutineAdapterCallableIdentity {
        &self.success
    }

    pub const fn failure(&self) -> &CoroutineAdapterCallableIdentity {
        &self.failure
    }

    pub const fn work(&self) -> &CoroutineAdapterCallableIdentity {
        &self.work
    }

    pub const fn odr_member_record(&self) -> Option<&OdrMemberRecord> {
        self.odr_member.as_ref()
    }
}

fn field(
    key: Result<FieldIdentityKey, FieldIdentityError>,
) -> Result<FieldRecord, ContinuationAdapterIdentityError> {
    CborIdentityRecord::from_key(key.map_err(ContinuationAdapterIdentityError::Field)?)
        .map_err(ContinuationAdapterIdentityError::Field)
}

fn callable(
    source: CallableMaterialization,
    suspension_site: StructuralDefinitionPath,
    role: CoroutineAdapterRole,
    signature: ExactCallableSignature,
    odr_group: Option<OdrGroupId>,
) -> Result<CoroutineAdapterCallableIdentity, ContinuationAdapterIdentityError> {
    let callable = CborIdentityRecord::from_key(GeneratedCallableKey::CoroutineAdapter {
        source_callable: source,
        suspension_site,
        role,
    })
    .map_err(ContinuationAdapterIdentityError::GeneratedCallable)?;
    let odr_member = odr_group
        .map(|group| {
            member(
                group,
                OdrMemberRole::CallableBody,
                OdrMemberDiscriminator::GeneratedCallable(callable.id()),
            )
        })
        .transpose()?;
    let subject = match &odr_member {
        Some(member) => CallableSignatureSubject::odr(
            CallableOdrMemberId::from_key(member.key())
                .map_err(ContinuationAdapterIdentityError::OdrMember)?,
        ),
        None => CallableSignatureSubject::strong(CallableOwner::Generated(callable.id())),
    };
    Ok(CoroutineAdapterCallableIdentity {
        callable,
        odr_member,
        signature: CallableSignatureRecord::new(subject, signature),
    })
}

fn member(
    group: OdrGroupId,
    role: OdrMemberRole,
    discriminator: OdrMemberDiscriminator,
) -> Result<OdrMemberRecord, ContinuationAdapterIdentityError> {
    let key = OdrMemberKey::new(group, role, discriminator)
        .map_err(ContinuationAdapterIdentityError::OdrMember)?;
    CborIdentityRecord::from_key(key).map_err(ContinuationAdapterIdentityError::OdrMemberRecord)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContinuationAdapterIdentityError {
    InvalidSuccessSignature,
    InvalidFailureSignature,
    InvalidWorkSignature,
    MissingOdrGroup,
    GeneratedType(GeneratedNominalIdentityError),
    GeneratedCallable(GeneratedCallableIdentityError),
    Field(FieldIdentityError),
    OdrMember(OdrMemberIdentityError),
    OdrMemberRecord(HashError),
}

impl fmt::Display for ContinuationAdapterIdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSuccessSignature => formatter.write_str(
                "continuation adapter success must be an ordinary one-parameter method",
            ),
            Self::InvalidWorkSignature => formatter.write_str("adapter work must be an ordinary receiver method with no value parameters"),
            Self::InvalidFailureSignature => formatter.write_str(
                "continuation adapter failure must share the success receiver and result with one parameter",
            ),
            Self::MissingOdrGroup => formatter.write_str(
                "a specialized coroutine source requires an ODR group for its continuation adapter",
            ),
            Self::GeneratedType(error) => error.fmt(formatter),
            Self::GeneratedCallable(error) => error.fmt(formatter),
            Self::Field(error) => error.fmt(formatter),
            Self::OdrMember(error) => error.fmt(formatter),
            Self::OdrMemberRecord(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for ContinuationAdapterIdentityError {}

#[cfg(test)]
mod tests {
    use scoop_identity::{
        CallableApplicationKey, CallableInstantiationOwner, CanonicalIdentifier, ConeIdentity,
        CoreBuiltinNominal, DeclarationScope, DefinitionOwnerChain, ExactTypeKey, PackagePath,
        PersistentFunctionId, SourceDeclarationKey, SourceDeclarationSite, SpecializationKey,
        StructuralDefinitionSiteRole, StructuralPathSegment,
    };

    use super::*;

    fn source_function() -> PersistentFunctionId {
        let site = SourceDeclarationSite::new(
            ConeIdentity::SINGLE_FILE,
            PackagePath::root(),
            DefinitionOwnerChain::top_level(),
            DeclarationScope::ConeWide,
        )
        .unwrap();
        PersistentFunctionId::from_source_declaration(&SourceDeclarationKey::function(
            site,
            CanonicalIdentifier::new("suspendSource").unwrap(),
            0,
            None,
            Vec::new(),
        ))
        .unwrap()
    }

    fn source() -> CallableMaterialization {
        CallableMaterialization::new(
            CallableTemplateOwner::Function(source_function()),
            CallableMaterializationContext::NoSubstitution,
        )
    }

    fn suspension_site(ordinal: u32) -> StructuralDefinitionPath {
        StructuralDefinitionPath::from_first(
            StructuralPathSegment::new(StructuralDefinitionSiteRole::CoroutineTransform, ordinal),
            [],
        )
    }

    fn signatures() -> (
        ExactCallableSignature,
        ExactCallableSignature,
        ExactCallableSignature,
    ) {
        let exact = |nominal| {
            CborIdentityRecord::from_key(ExactTypeKey::Nominal(nominal))
                .unwrap()
                .id()
        };
        let receiver = exact(CoreBuiltinNominal::Any.identity_record().id());
        let unit = exact(CoreBuiltinNominal::Unit.identity_record().id());
        (
            ExactCallableSignature::new(Effect::Ordinary, Some(receiver), vec![unit], unit),
            ExactCallableSignature::new(Effect::Ordinary, Some(receiver), vec![receiver], unit),
            ExactCallableSignature::new(Effect::Ordinary, Some(receiver), Vec::new(), unit),
        )
    }

    #[test]
    fn adapter_has_one_environment_and_three_callable_identities() {
        let source = source();
        let site = suspension_site(2);
        let (success, failure, work) = signatures();
        let identity = ContinuationAdapterIdentity::new(
            source,
            site.clone(),
            success.clone(),
            failure.clone(),
            work.clone(),
            None,
        )
        .unwrap();

        assert_eq!(
            identity.generated_type_record().key(),
            &GeneratedNominalKey::ContinuationAdapterEnvironment {
                source_callable: source,
                suspension_site: site.clone(),
            }
        );
        assert_eq!(
            identity.success().callable_record().key(),
            &GeneratedCallableKey::CoroutineAdapter {
                source_callable: source,
                suspension_site: site.clone(),
                role: CoroutineAdapterRole::Success,
            }
        );
        assert_eq!(
            identity.failure().callable_record().key(),
            &GeneratedCallableKey::CoroutineAdapter {
                source_callable: source,
                suspension_site: site.clone(),
                role: CoroutineAdapterRole::Failure,
            }
        );
        assert_eq!(
            identity.work().callable_record().key(),
            &GeneratedCallableKey::CoroutineAdapter {
                source_callable: source,
                suspension_site: site,
                role: CoroutineAdapterRole::Run,
            }
        );
        assert_eq!(identity.odr_member_record(), None);
        assert_eq!(identity.success().signature_record().signature(), &success);
        assert_eq!(identity.failure().signature_record().signature(), &failure);
        assert_eq!(identity.work().signature_record().signature(), &work);
        assert!(matches!(
            identity.success().signature_record().subject(),
            CallableSignatureSubject::Strong(CallableOwner::Generated(id))
                if id == identity.success().callable_record().id()
        ));
    }

    #[test]
    fn adapter_has_both_latch_field_identities() {
        let (success, failure, work) = signatures();
        let identity = ContinuationAdapterIdentity::new(
            source(),
            suspension_site(0),
            success,
            failure,
            work,
            None,
        )
        .unwrap();

        let result = identity.storage().result_field_record();
        let failure = identity.storage().failure_field_record();
        assert_eq!(
            result.key(),
            &FieldIdentityKey::coroutine_adapter_result(identity.generated_type_record().key())
                .unwrap()
        );
        assert_eq!(
            failure.key(),
            &FieldIdentityKey::coroutine_adapter_failure(identity.generated_type_record().key())
                .unwrap()
        );
    }

    #[test]
    fn adapter_rejects_non_protocol_callback_signatures() {
        let (success, failure, work) = signatures();
        assert_eq!(
            ContinuationAdapterIdentity::new(
                source(),
                suspension_site(0),
                ExactCallableSignature::new(
                    Effect::Suspend,
                    success.receiver().into_option(),
                    success.parameters().to_vec(),
                    success.result(),
                ),
                failure.clone(),
                work.clone(),
                None,
            ),
            Err(ContinuationAdapterIdentityError::InvalidSuccessSignature)
        );
        assert_eq!(
            ContinuationAdapterIdentity::new(
                source(),
                suspension_site(0),
                success,
                ExactCallableSignature::new(
                    Effect::Ordinary,
                    None,
                    failure.parameters().to_vec(),
                    failure.result(),
                ),
                work,
                None,
            ),
            Err(ContinuationAdapterIdentityError::InvalidFailureSignature)
        );
    }

    #[test]
    fn specialized_adapter_source_links_all_four_generated_members_to_its_group() {
        let application_key = CallableApplicationKey::for_function(
            source_function(),
            CallableInstantiationOwner::NoOwner,
        );
        let application = CborIdentityRecord::from_key(application_key.clone())
            .unwrap()
            .id();
        let group = CborIdentityRecord::from_key(SpecializationKey::Callable {
            application: application_key,
        })
        .unwrap()
        .id();
        let source = CallableMaterialization::new(
            CallableTemplateOwner::Function(source_function()),
            CallableMaterializationContext::Application(application),
        );
        let (success, failure, work) = signatures();
        let identity = ContinuationAdapterIdentity::new(
            source,
            suspension_site(0),
            success,
            failure,
            work,
            Some(group),
        )
        .unwrap();
        let members = [
            identity.odr_member_record().unwrap(),
            identity.success().odr_member_record().unwrap(),
            identity.failure().odr_member_record().unwrap(),
            identity.work().odr_member_record().unwrap(),
        ];

        assert!(members.iter().all(|member| member.key().group() == group));
        assert_eq!(members[0].key().role(), OdrMemberRole::GeneratedNominal);
        assert_eq!(members[1].key().role(), OdrMemberRole::CallableBody);
        assert_eq!(members[2].key().role(), OdrMemberRole::CallableBody);
        assert_ne!(members[0].id(), members[1].id());
        assert_ne!(members[1].id(), members[2].id());
        assert_eq!(members[3].key().role(), OdrMemberRole::CallableBody);
        assert_ne!(members[2].id(), members[3].id());
        assert!(matches!(
            identity.success().signature_record().subject(),
            CallableSignatureSubject::Odr(subject) if subject.member() == members[1].id()
        ));
    }
}
