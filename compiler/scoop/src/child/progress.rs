//! Human progress for an actual compiler invocation, independent of its wire output.

use std::io::{self, Write};
use std::time::Instant;

use scoop_protocol::{CurrentConeRequestV1, ScoopcOutputRequestV1, ScoopcRequestEnvelopeV1};

pub(super) struct CompilationProgress {
    coordinate: String,
    started: Instant,
}

impl CompilationProgress {
    pub fn start(request: &ScoopcRequestEnvelopeV1) -> io::Result<Self> {
        let coordinate = match request.build().current() {
            CurrentConeRequestV1::ExpandedCone { config } => {
                config.manifest.semantic().coordinate().to_string()
            }
            CurrentConeRequestV1::SingleFile { .. } => "scoop:single-file:0.0.0".to_owned(),
        };
        let action = match request.build().output() {
            ScoopcOutputRequestV1::Artifact(_)
            | ScoopcOutputRequestV1::ArtifactWithDocumentation { .. } => "Compiling",
            ScoopcOutputRequestV1::Check => "Checking",
            ScoopcOutputRequestV1::Documentation(_) => "Documenting",
        };
        write_line(format!("{action:>12} {coordinate}\n"))?;
        Ok(Self {
            coordinate,
            started: Instant::now(),
        })
    }

    pub fn finish(self, success: bool) -> io::Result<()> {
        let status = if success { "Finished" } else { "Failed" };
        write_line(format!(
            "{status:>12} {} in {:.2}s\n",
            self.coordinate,
            self.started.elapsed().as_secs_f64()
        ))
    }
}

fn write_line(line: String) -> io::Result<()> {
    io::stderr().lock().write_all(line.as_bytes())
}
