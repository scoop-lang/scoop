use super::*;

impl WireEncode for LinuxFinalLinkProfile {
    fn encode(&self, e: &mut Encoder) -> Result<(), scoop_wire::cbor::EncodeError> {
        e.map(7)?;
        e.field(1)?;
        self.startup.profile().contract().target().encode(e)?;
        e.field(2)?;
        self.startup.profile().contract().encode(e)?;
        e.field(3)?;
        let linker_args = self.linker_args();
        e.array((linker_args.len() + usize::from(!self.cxx())) as u64)?;
        if !self.cxx() {
            e.text("-nodefaultlibs")?;
        }
        for option in linker_args {
            e.text(option)?;
        }
        e.field(4)?;
        sha256(self.elf_plan().metadata_script().as_bytes()).encode(e)?;
        e.field(5)?;
        e.unsigned(1)?;
        e.field(6)?;
        match &self.data.runtime {
            RuntimeLibraries::C { .. } => e.array(0),
            RuntimeLibraries::Cxx(cxx) => {
                e.array(1)?;
                cxx.fingerprint().encode(e)
            }
        }?;
        e.field(7)?;
        self.data.output.encode(e)
    }
}
