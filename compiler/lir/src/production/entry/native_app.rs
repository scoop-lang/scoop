use super::*;
use scoop_identity::AppInitCallableBodyId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeAppEntryPlanV1 {
    pub(super) root_cone: scoop_identity::ConeIdentity,
    pub(super) declaration: scoop_identity::PersistentFunctionId,
    pub(super) source_signature: ExactCallableSignature,
    pub(super) source_signature_fingerprint: SourceSignatureFingerprint,
    pub(super) app_init: AppInitCallableBodyId,
    pub(super) failure_root: PersistentStaticStorageId,
    pub(super) gateway: PersistentCallableBodyId,
    pub(super) root_descriptor_symbol: PersistentSymbolRequest,
    pub(super) root_descriptor_definition: ObjectDefinitionPlanId,
}

impl NativeAppEntryPlanV1 {
    pub const fn root_cone(&self) -> scoop_identity::ConeIdentity {
        self.root_cone
    }

    pub const fn declaration(&self) -> scoop_identity::PersistentFunctionId {
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

    pub const fn failure_root(&self) -> PersistentStaticStorageId {
        self.failure_root
    }

    pub const fn gateway(&self) -> PersistentCallableBodyId {
        self.gateway
    }

    pub const fn root_descriptor_symbol(&self) -> PersistentSymbolRequest {
        self.root_descriptor_symbol
    }

    pub const fn root_descriptor_definition(&self) -> ObjectDefinitionPlanId {
        self.root_descriptor_definition
    }

    pub fn failure_root_registration_symbol(
        &self,
    ) -> Result<PersistentSymbolRequest, PersistentSymbolError> {
        PersistentSymbolRequest::new(
            PersistentSymbolKey::RootRegistration(self.failure_root),
            scoop_identity::LinkageClass::ConeStrong,
        )
    }

    pub fn gateway_symbol(&self) -> Result<PersistentSymbolRequest, PersistentSymbolError> {
        PersistentSymbolRequest::new(
            PersistentSymbolKey::CallableBody(self.gateway),
            scoop_identity::LinkageClass::ConeStrong,
        )
    }
}

impl WireEncode for NativeAppEntryPlanV1 {
    fn encode(&self, encoder: &mut Encoder) -> Result<(), scoop_wire::cbor::EncodeError> {
        encoder.map(9)?;
        encoder.field(1)?;
        self.root_cone.encode(encoder)?;
        encoder.field(2)?;
        self.declaration.encode(encoder)?;
        encoder.field(3)?;
        self.source_signature.encode(encoder)?;
        encoder.field(4)?;
        self.source_signature_fingerprint.encode(encoder)?;
        encoder.field(5)?;
        self.app_init.encode(encoder)?;
        encoder.field(6)?;
        self.failure_root.encode(encoder)?;
        encoder.field(7)?;
        self.gateway.encode(encoder)?;
        encoder.field(8)?;
        self.root_descriptor_symbol.encode(encoder)?;
        encoder.field(9)?;
        self.root_descriptor_definition.encode(encoder)
    }
}

#[derive(Debug)]
pub struct DecodedNativeAppEntryPlanV1 {
    pub(super) root_cone: DecodedPersistentId<scoop_identity::ConeIdentity>,
    pub(super) declaration: DecodedPersistentId<scoop_identity::PersistentFunctionId>,
    pub(super) source_signature: DecodedExactCallableSignature,
    pub(super) source_signature_fingerprint: DecodedPersistentId<SourceSignatureFingerprint>,
    pub(super) app_init: DecodedPersistentId<PersistentCallableBodyId>,
    pub(super) failure_root: DecodedPersistentId<PersistentStaticStorageId>,
    pub(super) gateway: DecodedPersistentId<PersistentCallableBodyId>,
    pub(super) root_descriptor_symbol: DecodedPersistentSymbolRequest,
    pub(super) root_descriptor_definition: DecodedPersistentId<ObjectDefinitionPlanId>,
}

impl WireEncode for DecodedNativeAppEntryPlanV1 {
    fn encode(&self, encoder: &mut Encoder) -> Result<(), scoop_wire::cbor::EncodeError> {
        encoder.map(9)?;
        encoder.field(1)?;
        self.root_cone.encode(encoder)?;
        encoder.field(2)?;
        self.declaration.encode(encoder)?;
        encoder.field(3)?;
        self.source_signature.encode(encoder)?;
        encoder.field(4)?;
        self.source_signature_fingerprint.encode(encoder)?;
        encoder.field(5)?;
        self.app_init.encode(encoder)?;
        encoder.field(6)?;
        self.failure_root.encode(encoder)?;
        encoder.field(7)?;
        self.gateway.encode(encoder)?;
        encoder.field(8)?;
        self.root_descriptor_symbol.encode(encoder)?;
        encoder.field(9)?;
        self.root_descriptor_definition.encode(encoder)
    }
}

impl WireDecode for DecodedNativeAppEntryPlanV1 {
    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, WireError> {
        decoder.expect_map(9)?;
        Ok(Self {
            root_cone: decoder.field(1, DecodedPersistentId::decode)?,
            declaration: decoder.field(2, DecodedPersistentId::decode)?,
            source_signature: decoder.field(3, DecodedExactCallableSignature::decode)?,
            source_signature_fingerprint: decoder.field(4, DecodedPersistentId::decode)?,
            app_init: decoder.field(5, DecodedPersistentId::decode)?,
            failure_root: decoder.field(6, DecodedPersistentId::decode)?,
            gateway: decoder.field(7, DecodedPersistentId::decode)?,
            root_descriptor_symbol: decoder.field(8, DecodedPersistentSymbolRequest::decode)?,
            root_descriptor_definition: decoder.field(9, DecodedPersistentId::decode)?,
        })
    }
}
