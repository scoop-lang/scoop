//! Published documentation data and offline queries. Keys are presentation addresses.

pub mod query;
pub mod render;
pub mod schema;

pub use schema::*;

#[derive(Debug)]
pub struct ApiError {
    pub code: scoop_source::DiagnosticCode,
    pub message: String,
    pub help: Option<(scoop_source::DiagnosticCode, String)>,
}
impl ApiError {
    pub fn new(code: scoop_source::DiagnosticCode, message: impl std::fmt::Display) -> Self {
        Self {
            code,
            message: message.to_string(),
            help: None,
        }
    }
    pub fn with_help(
        mut self,
        code: scoop_source::DiagnosticCode,
        message: impl Into<String>,
    ) -> Self {
        self.help = Some((code, message.into()));
        self
    }
}
impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)?;
        if let Some((code, message)) = &self.help {
            write!(f, "\nhelp[{code}]: {message}")?;
        }
        Ok(())
    }
}
impl std::error::Error for ApiError {}
