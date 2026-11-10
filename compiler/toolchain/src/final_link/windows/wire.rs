use super::*;

impl WireEncode for WindowsFinalLinkProfile {
    fn encode(&self, e: &mut Encoder) -> Result<(), scoop_wire::cbor::EncodeError> {
        e.array(5)?;
        self.startup.target().wire_id().encode(e)?;
        self.startup.profile().contract().encode(e)?;
        let linker_args = self.linker_args();
        e.array(linker_args.len() as u64)?;
        for option in linker_args {
            e.text(option)?;
        }
        match &self.data.cxx {
            Some(cxx) => {
                e.array(1)?;
                cxx.fingerprint().encode(e)
            }
            None => e.array(0),
        }?;
        self.data.output.encode(e)
    }
}
