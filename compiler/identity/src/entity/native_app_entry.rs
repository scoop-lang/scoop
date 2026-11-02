//! Typed source entry for a native-app root.

use crate::{
    AppInitCallableBodyId, CallableBodyKey, CborIdentityRecord, ConeIdentity, DeclarationName,
    DecodedPersistentId, DuplicateSignatureKey, ExactCallableSignature, OptionalSignatureType,
    PersistentCallableBodyId, PersistentFunctionId, SourceDeclarationKey, SourceDeclarationKind,
    SourceSignatureFingerprint,
};
use scoop_wire::{Decoder, Encoder, HashError, WireDecode, WireEncode, WireError};
use std::fmt;

/// Persistent declaration, signature and body of the selected native-app entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeAppEntryIdentity {
    root_cone: ConeIdentity,
    declaration: PersistentFunctionId,
    source_signature: ExactCallableSignature,
    source_signature_fingerprint: SourceSignatureFingerprint,
    app_init: AppInitCallableBodyId,
}

impl NativeAppEntryIdentity {
    pub fn try_new(
        declaration: &CborIdentityRecord<PersistentFunctionId, SourceDeclarationKey>,
        source_signature: ExactCallableSignature,
    ) -> Result<Self, NativeAppEntryIdentityError> {
        let key = declaration.key();
        if key.declaration_kind() != SourceDeclarationKind::Function {
            return Err(NativeAppEntryIdentityError::NotFunction);
        }
        if !key.owners().owners().is_empty() {
            return Err(NativeAppEntryIdentityError::NotTopLevel);
        }
        if !matches!(key.name(), DeclarationName::Named(name) if name.as_str() == "appInit") {
            return Err(NativeAppEntryIdentityError::NotAppInit);
        }
        if !matches!(
            key.duplicate_signature(),
            DuplicateSignatureKey::Function {
                type_parameter_count: 0,
                receiver: OptionalSignatureType::Absent,
                parameters,
                conditions,
            } if conditions.is_empty() && parameters.len() == source_signature.parameters().len() && parameters.len() == 2
        ) {
            return Err(NativeAppEntryIdentityError::InvalidDeclarationShape);
        }
        if source_signature.effect() != super::Effect::Ordinary
            || source_signature.receiver().is_present()
        {
            return Err(NativeAppEntryIdentityError::InvalidDeclarationShape);
        }
        let declaration_id = declaration.id();
        let body = PersistentCallableBodyId::from_key(&CallableBodyKey::strong(
            crate::StrongCallableDefinitionOwner::Function(declaration_id),
        ))
        .map_err(NativeAppEntryIdentityError::Hash)?;
        let source_signature_fingerprint =
            SourceSignatureFingerprint::from_signature(&source_signature)
                .map_err(NativeAppEntryIdentityError::Hash)?;
        Ok(Self {
            root_cone: key.origin(),
            declaration: declaration_id,
            source_signature,
            source_signature_fingerprint,
            app_init: AppInitCallableBodyId::from_body(body),
        })
    }

    pub const fn root_cone(&self) -> ConeIdentity {
        self.root_cone
    }

    pub const fn declaration(&self) -> PersistentFunctionId {
        self.declaration
    }

    pub const fn source_signature(&self) -> &ExactCallableSignature {
        &self.source_signature
    }

    pub const fn source_signature_fingerprint(&self) -> SourceSignatureFingerprint {
        self.source_signature_fingerprint
    }

    pub const fn app_init(&self) -> AppInitCallableBodyId {
        self.app_init
    }
}

impl WireEncode for NativeAppEntryIdentity {
    fn encode(&self, encoder: &mut Encoder) -> Result<(), scoop_wire::cbor::EncodeError> {
        encoder.map(5)?;
        encoder.field(1)?;
        self.root_cone.encode(encoder)?;
        encoder.field(2)?;
        self.declaration.encode(encoder)?;
        encoder.field(3)?;
        self.source_signature.encode(encoder)?;
        encoder.field(4)?;
        self.source_signature_fingerprint.encode(encoder)?;
        encoder.field(5)?;
        self.app_init.encode(encoder)
    }
}

/// Untrusted wire form of [`NativeAppEntryIdentity`].
///
/// Readers resolve the source declaration and exact types, then check the
/// stored identities and fingerprint against the resolved signature.
#[derive(Debug)]
pub struct DecodedNativeAppEntryIdentity {
    root_cone: DecodedPersistentId<ConeIdentity>,
    declaration: DecodedPersistentId<PersistentFunctionId>,
    source_signature: crate::DecodedExactCallableSignature,
    source_signature_fingerprint: DecodedPersistentId<SourceSignatureFingerprint>,
    app_init: DecodedPersistentId<PersistentCallableBodyId>,
}

impl DecodedNativeAppEntryIdentity {
    pub const fn source_signature(&self) -> &crate::DecodedExactCallableSignature {
        &self.source_signature
    }

    pub const fn declaration(&self) -> DecodedPersistentId<PersistentFunctionId> {
        self.declaration
    }
}

impl WireEncode for DecodedNativeAppEntryIdentity {
    fn encode(&self, encoder: &mut Encoder) -> Result<(), scoop_wire::cbor::EncodeError> {
        encoder.map(5)?;
        encoder.field(1)?;
        self.root_cone.encode(encoder)?;
        encoder.field(2)?;
        self.declaration.encode(encoder)?;
        encoder.field(3)?;
        self.source_signature.encode(encoder)?;
        encoder.field(4)?;
        self.source_signature_fingerprint.encode(encoder)?;
        encoder.field(5)?;
        self.app_init.encode(encoder)
    }
}

impl WireDecode for DecodedNativeAppEntryIdentity {
    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, WireError> {
        decoder.expect_map(5)?;
        Ok(Self {
            root_cone: decoder.field(1, DecodedPersistentId::decode)?,
            declaration: decoder.field(2, DecodedPersistentId::decode)?,
            source_signature: decoder.field(3, crate::DecodedExactCallableSignature::decode)?,
            source_signature_fingerprint: decoder.field(4, DecodedPersistentId::decode)?,
            app_init: decoder.field(5, DecodedPersistentId::decode)?,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeAppEntryIdentityError {
    NotFunction,
    NotTopLevel,
    NotAppInit,
    InvalidDeclarationShape,
    Hash(HashError),
}

impl fmt::Display for NativeAppEntryIdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid native-app entry identity: {self:?}")
    }
}

impl std::error::Error for NativeAppEntryIdentityError {}
