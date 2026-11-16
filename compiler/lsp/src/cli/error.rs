use scoop_source::DiagnosticCode;
use std::{fmt, io};

#[derive(Debug)]
pub(super) struct QueryError {
    pub code: String,
    pub message: String,
}
pub(super) type QueryResult<T> = Result<T, QueryError>;

impl QueryError {
    pub fn new(code: DiagnosticCode, message: impl fmt::Display) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
        }
    }
    pub fn response(method: &str, error: lsp_server::ResponseError) -> Self {
        let code = error
            .data
            .as_ref()
            .and_then(|data| data.get("code"))
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned)
            .unwrap_or_else(|| DiagnosticCode::E2505.to_string());
        Self {
            code,
            message: format!("{method}: {} ({})", error.message, error.code),
        }
    }
    pub fn transport(error: io::Error) -> Self {
        Self::new(
            if error.kind() == io::ErrorKind::InvalidData {
                DiagnosticCode::ICE2501
            } else {
                DiagnosticCode::E2204
            },
            error,
        )
    }
}
impl fmt::Display for QueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for QueryError {}
impl From<io::Error> for QueryError {
    fn from(error: io::Error) -> Self {
        Self::new(DiagnosticCode::E2201, error)
    }
}
impl From<serde_json::Error> for QueryError {
    fn from(error: serde_json::Error) -> Self {
        Self::new(DiagnosticCode::ICE2501, error)
    }
}
