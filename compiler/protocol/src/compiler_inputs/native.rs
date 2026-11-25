//! Native dependency observations, including currently absent include candidates.

use serde::{Deserialize, Serialize};

use super::{decode_modified, decode_path, invalid};
use crate::HostPathCarrier;
use scoop_wire::{Decoder, Encoder, WireDecode, WireEncode, WireError};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct NativeInputV1 {
    pub path: HostPathCarrier,
    pub state: Option<NativeInputStateV1>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct NativeInputStateV1 {
    pub resolved: HostPathCarrier,
    pub length: u64,
    pub modified: (bool, u64, u32),
}

impl WireEncode for NativeInputV1 {
    fn encode(&self, e: &mut Encoder) -> Result<(), scoop_wire::cbor::EncodeError> {
        e.array(2)?;
        self.path.encode(e)?;
        e.array(u64::from(self.state.is_some()))?;
        if let Some(state) = &self.state {
            e.array(5)?;
            state.resolved.encode(e)?;
            e.unsigned(state.length)?;
            e.unsigned(u64::from(state.modified.0))?;
            e.unsigned(state.modified.1)?;
            e.unsigned(u64::from(state.modified.2))?;
        }
        Ok(())
    }
}

impl WireDecode for NativeInputV1 {
    fn decode(d: &mut Decoder<'_>) -> Result<Self, WireError> {
        if d.array()? != 2 {
            return Err(invalid(d));
        }
        let path = decode_path(d)?;
        let state = match d.array()? {
            0 => None,
            1 => {
                if d.array()? != 5 {
                    return Err(invalid(d));
                }
                Some(NativeInputStateV1 {
                    resolved: decode_path(d)?,
                    length: d.unsigned()?,
                    modified: decode_modified(d)?,
                })
            }
            _ => return Err(invalid(d)),
        };
        Ok(Self { path, state })
    }
}
