//! Request-local host services for compile-time inputs. No host I/O lives in HIR.

/// A valid environment name that may be queried by a compiler intrinsic.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CompileTimeEnvironmentName(String);

impl CompileTimeEnvironmentName {
    pub fn parse(name: &str) -> Result<Self, &'static str> {
        if !name.starts_with("SCOOP_") {
            return Err("compile-time environment names must start with `SCOOP_`");
        }
        if name.contains(['=', '\0']) {
            return Err("compile-time environment names must not contain `=` or NUL");
        }
        Ok(Self(name.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Supplied by the compiler or editor host; shared by candidate-local probes.
pub trait CompileTimeInputHost: Send + Sync {
    fn environment(
        &self,
        name: &CompileTimeEnvironmentName,
    ) -> Result<Option<String>, CompileTimeInputError>;
    fn file(
        &self,
        source: &scoop_identity::SourceIdentity,
        path: &CompileTimePath,
    ) -> Result<std::sync::Arc<[u8]>, CompileTimeInputError>;
    fn directory(
        &self,
        source: &scoop_identity::SourceIdentity,
        path: &CompileTimePath,
        filter: &CompileTimeDirectoryFilter,
    ) -> Result<std::sync::Arc<[CompileTimeFile]>, CompileTimeDirectoryError>;
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CompileTimeDirectoryFilter {
    pub include: Vec<String>,
    pub exclude: Vec<String>,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CompileTimeFile {
    /// Logical UTF-8 path relative to the selected directory, using slash separators.
    pub path: String,
    pub bytes: std::sync::Arc<[u8]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompileTimeDirectoryError {
    Input(CompileTimeInputError),
    IncludePattern { index: usize, message: String },
    ExcludePattern { index: usize, message: String },
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CompileTimePath(String);

impl CompileTimePath {
    pub fn parse(path: &str) -> Result<Self, &'static str> {
        if path.is_empty() || path.contains('\0') {
            return Err("include path must be nonempty and must not contain NUL");
        }
        Ok(Self(path.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One host-input failure, classified before it crosses the HIR service boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompileTimeInputError {
    pub code: scoop_source::DiagnosticCode,
    pub message: String,
}
impl CompileTimeInputError {
    pub fn new(code: scoop_source::DiagnosticCode, message: impl std::fmt::Display) -> Self {
        Self {
            code,
            message: message.to_string(),
        }
    }
}
impl std::fmt::Display for CompileTimeInputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for CompileTimeInputError {}
