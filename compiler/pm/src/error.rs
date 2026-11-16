use std::{
    fmt,
    ops::Range,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManifestOrigin {
    pub manifest: PathBuf,
    pub span: Range<usize>,
}

#[derive(Debug)]
pub struct PmError {
    pub notes: Vec<scoop_protocol::DiagnosticNoteV1>,
    pub code: scoop_source::DiagnosticCode,
    pub message: String,
    pub origins: Vec<ManifestOrigin>,
    pub(crate) resolution_conflicts: Vec<scoop_manifest::DependencyCoordinateKey>,
}

pub type PmResult<T> = Result<T, PmError>;

impl PmError {
    pub fn new(code: scoop_source::DiagnosticCode, message: impl Into<String>) -> Self {
        Self {
            notes: Vec::new(),
            code,
            message: message.into(),
            origins: Vec::new(),
            resolution_conflicts: Vec::new(),
        }
    }

    pub fn with_help(self, code: scoop_source::DiagnosticCode, message: impl Into<String>) -> Self {
        self.with_note(code, message)
    }

    pub fn with_note(
        mut self,
        code: scoop_source::DiagnosticCode,
        message: impl Into<String>,
    ) -> Self {
        self.notes.push(
            scoop_protocol::DiagnosticNoteV1::new(
                code,
                message.into(),
                scoop_protocol::DiagnosticOriginV1::None,
            )
            .expect("a compiler attachment has a nonempty message"),
        );
        self
    }

    pub fn at(mut self, origin: ManifestOrigin) -> Self {
        self.origins.push(origin);
        self
    }

    pub(crate) fn io(path: &Path, error: std::io::Error) -> Self {
        Self::new(
            scoop_source::DiagnosticCode::E2201,
            format!("{}: {error}", path.display()),
        )
    }

    pub(crate) fn manifest_read(path: &Path, error: std::io::Error) -> Self {
        let code = if error.kind() == std::io::ErrorKind::InvalidData {
            scoop_source::DiagnosticCode::E2003
        } else {
            scoop_source::DiagnosticCode::E2201
        };
        Self::new(code, format!("{}: {error}", path.display()))
    }

    pub fn machine_diagnostic(
        &self,
    ) -> Result<scoop_protocol::package::PackageDiagnosticV1, String> {
        use scoop_protocol::{
            DiagnosticNoteV1, DiagnosticOriginV1, DiagnosticSeverityV1, HostPathCarrier,
            ProtocolByteSpan, StructuredDiagnosticV1,
        };
        let mut origins = self.origins.iter().map(|origin| -> Result<_, String> {
            Ok(DiagnosticOriginV1::HostPathSpan {
                path: HostPathCarrier::from_path(&origin.manifest).map_err(|e| e.to_string())?,
                span: ProtocolByteSpan::new(
                    origin
                        .span
                        .start
                        .try_into()
                        .map_err(|e: std::num::TryFromIntError| e.to_string())?,
                    origin
                        .span
                        .end
                        .try_into()
                        .map_err(|e: std::num::TryFromIntError| e.to_string())?,
                )
                .map_err(|e| e.to_string())?,
            })
        });
        let origin = origins
            .next()
            .transpose()?
            .unwrap_or(DiagnosticOriginV1::None);
        let mut notes: Vec<_> = origins
            .map(|origin| {
                DiagnosticNoteV1::new(
                    scoop_source::DiagnosticCode::N0001,
                    "related declaration".into(),
                    origin?,
                )
                .map_err(|e| e.to_string())
            })
            .collect::<Result<_, String>>()?;
        notes.extend(self.notes.iter().cloned());
        Ok(scoop_protocol::package::PackageDiagnosticV1 {
            version: scoop_protocol::package::PACKAGE_PROTOCOL_VERSION,
            diagnostic: StructuredDiagnosticV1::new(
                DiagnosticSeverityV1::Error,
                self.code.to_string(),
                self.message.clone(),
                origin,
                notes,
            )
            .map_err(|e| e.to_string())?,
        })
    }
}

impl fmt::Display for PmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)?;
        for origin in &self.origins {
            write!(
                f,
                "\n  at {} (bytes {}..{})",
                origin.manifest.display(),
                origin.span.start,
                origin.span.end
            )?;
        }
        for note in &self.notes {
            write!(
                f,
                "\n{}[{}]: {}",
                note.code().role().as_str(),
                note.code(),
                note.message()
            )?;
        }
        Ok(())
    }
}

impl std::error::Error for PmError {}
