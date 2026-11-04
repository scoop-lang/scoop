//! Untrusted callables registration carriers.

use super::*;

#[derive(Debug)]
pub struct DecodedStrongCallableRegistrationPlanV1 {
    body: DecodedPersistentId<PersistentCallableBodyId>,
    symbol: DecodedPersistentSymbolRequest,
    definition_plan: DecodedPersistentId<ObjectDefinitionPlanId>,
    primary_atom: DecodedPersistentId<ObjectDefinitionAtomId>,
    entry_symbol: DecodedPersistentSymbolRequest,
    body_definition_plan: DecodedPersistentId<ObjectDefinitionPlanId>,
    body_primary_atom: DecodedPersistentId<ObjectDefinitionAtomId>,

    body_definition_node: DecodedPersistentId<DigestNodeId>,
}

impl WireEncode for DecodedStrongCallableRegistrationPlanV1 {
    fn encode(&self, encoder: &mut Encoder) -> Result<(), scoop_wire::cbor::EncodeError> {
        encoder.map(8)?;
        encode_field(encoder, 1, &self.body)?;
        encode_field(encoder, 2, &self.symbol)?;
        encode_field(encoder, 3, &self.definition_plan)?;
        encode_field(encoder, 4, &self.primary_atom)?;
        encode_field(encoder, 5, &self.entry_symbol)?;
        encode_field(encoder, 6, &self.body_definition_plan)?;
        encode_field(encoder, 7, &self.body_primary_atom)?;
        encode_field(encoder, 9, &self.body_definition_node)?;
        Ok(())
    }
}

impl WireDecode for DecodedStrongCallableRegistrationPlanV1 {
    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, WireError> {
        decoder.expect_map(8)?;
        Ok(Self {
            body: decoder.field(1, DecodedPersistentId::decode)?,
            symbol: decoder.field(2, DecodedPersistentSymbolRequest::decode)?,
            definition_plan: decoder.field(3, DecodedPersistentId::decode)?,
            primary_atom: decoder.field(4, DecodedPersistentId::decode)?,
            entry_symbol: decoder.field(5, DecodedPersistentSymbolRequest::decode)?,
            body_definition_plan: decoder.field(6, DecodedPersistentId::decode)?,
            body_primary_atom: decoder.field(7, DecodedPersistentId::decode)?,

            body_definition_node: decoder.field(9, DecodedPersistentId::decode)?,
        })
    }
}
