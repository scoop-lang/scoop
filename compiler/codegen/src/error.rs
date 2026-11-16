//! Distinguish malformed compiler IR from backend, I/O and external-tool failures.

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodegenErrorKind {
    Internal,
    Backend,
    Io,
    Tool,
}

#[derive(Debug)]
pub struct CodegenError {
    kind: CodegenErrorKind,
    message: String,
    cause: Option<Box<dyn Error + Send + Sync>>,
}

impl CodegenError {
    pub fn internal(message: String) -> Self {
        Self {
            kind: CodegenErrorKind::Internal,
            message,
            cause: None,
        }
    }

    pub fn backend_message(message: String) -> Self {
        Self {
            kind: CodegenErrorKind::Backend,
            message,
            cause: None,
        }
    }

    pub fn internal_cause(message: String, cause: impl Into<Box<dyn Error + Send + Sync>>) -> Self {
        Self::caused(CodegenErrorKind::Internal, message, cause)
    }

    pub fn backend(message: String, cause: impl Into<Box<dyn Error + Send + Sync>>) -> Self {
        Self::caused(CodegenErrorKind::Backend, message, cause)
    }

    pub fn io(message: String, cause: std::io::Error) -> Self {
        Self::caused(CodegenErrorKind::Io, message, cause)
    }

    pub fn tool(message: String, cause: impl Into<Box<dyn Error + Send + Sync>>) -> Self {
        Self::caused(CodegenErrorKind::Tool, message, cause)
    }

    fn caused(
        kind: CodegenErrorKind,
        message: String,
        cause: impl Into<Box<dyn Error + Send + Sync>>,
    ) -> Self {
        Self {
            kind,
            message,
            cause: Some(cause.into()),
        }
    }

    pub const fn kind(&self) -> CodegenErrorKind {
        self.kind
    }
}

impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for CodegenError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.cause
            .as_deref()
            .map(|cause| cause as &(dyn Error + 'static))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failures_retain_their_category_and_original_cause() {
        let io = CodegenError::io(
            "write object".into(),
            std::io::Error::from(std::io::ErrorKind::PermissionDenied),
        );
        assert_eq!(io.kind(), CodegenErrorKind::Io);
        assert_eq!(
            io.source()
                .unwrap()
                .downcast_ref::<std::io::Error>()
                .unwrap()
                .kind(),
            std::io::ErrorKind::PermissionDenied
        );
        let llvm = CodegenError::backend(
            "emit call".into(),
            inkwell::builder::BuilderError::UnsetPosition,
        );
        assert_eq!(llvm.kind(), CodegenErrorKind::Backend);
        assert!(
            llvm.source()
                .unwrap()
                .is::<inkwell::builder::BuilderError>()
        );
        let internal = CodegenError::internal("invalid LIR operand".into());
        assert_eq!(internal.kind(), CodegenErrorKind::Internal);
        assert!(internal.source().is_none());
    }
}
