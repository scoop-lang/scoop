//! Identity of the managed work item that enters one exact-result coroutine.

use super::*;
use scoop_identity::{FieldIdentityKey, GeneratedNominalKey, PersistentFieldId, PersistentTypeId};

type TypeRecord = CborIdentityRecord<PersistentTypeId, GeneratedNominalKey>;
type FieldRecord = CborIdentityRecord<PersistentFieldId, FieldIdentityKey>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoroutineStartWorkIdentity {
    generated_type: TypeRecord,
    fields: [FieldRecord; 3],
    type_root: ExactOwnerRoot,
    run: ExactCoroutineCallableIdentity,
}

impl CoroutineStartWorkIdentity {
    pub fn new(
        result: &ExactTypeRecord,
        nominal_group: Option<&OdrGroupRecord>,
        signature: ExactCallableSignature,
    ) -> Result<Self, CoroutineSupportIdentityError> {
        if signature.effect() != Effect::Ordinary
            || !signature.receiver().is_present()
            || !signature.parameters().is_empty()
        {
            return Err(CoroutineSupportIdentityError::InvalidWorkSignature);
        }
        let generated_type =
            CborIdentityRecord::from_key(GeneratedNominalKey::CoroutineStartWork {
                result: result.id(),
            })
            .map_err(CoroutineSupportIdentityError::GeneratedType)?;
        let field = |index| {
            FieldIdentityKey::coroutine_start_slot(generated_type.key(), index)
                .and_then(CborIdentityRecord::from_key)
                .map_err(CoroutineSupportIdentityError::Field)
        };
        let fields = [field(0)?, field(1)?, field(2)?];
        let type_root = ExactOwnerRoot::for_member(
            result,
            nominal_group,
            OdrMemberRole::GeneratedNominal,
            OdrMemberDiscriminator::GeneratedNominal(generated_type.id()),
        )
        .map_err(CoroutineSupportIdentityError::Root)?;
        let run = ExactCoroutineCallableIdentity::new(
            result,
            nominal_group,
            GeneratedCallableKey::CoroutineStartWork {
                result: result.id(),
            },
            signature,
        )?;
        Ok(Self {
            generated_type,
            fields,
            type_root,
            run,
        })
    }

    pub const fn generated_type_record(&self) -> &TypeRecord {
        &self.generated_type
    }
    pub const fn fields(&self) -> &[FieldRecord; 3] {
        &self.fields
    }
    pub const fn type_root(&self) -> &ExactOwnerRoot {
        &self.type_root
    }
    pub const fn callable_root(&self) -> &ExactOwnerRoot {
        &self.run.root
    }
    pub const fn callable_record(&self) -> &GeneratedCallableRecord {
        &self.run.callable
    }
    pub const fn signature_record(&self) -> &CallableSignatureRecord {
        &self.run.signature
    }
}
