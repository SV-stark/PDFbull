//! Enhanced error types

use std::fmt;

/// Result type for enhanced operations
pub type Result<T> = std::result::Result<T, EnhancedError>;

/// Enhanced error types
#[derive(Debug)]
pub enum EnhancedError {
    /// Invalid parameter
    InvalidParameter(String),
    /// I/O error
    Io(std::io::Error),
    /// Feature not supported (fundamental limitation)
    Unsupported(String),
    /// Feature not yet implemented
    NotImplemented(String),
    /// Generic error
    Generic(String),
}

impl fmt::Display for EnhancedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EnhancedError::InvalidParameter(msg) => write!(f, "Invalid parameter: {}", msg),
            EnhancedError::Io(err) => write!(f, "I/O error: {}", err),
            EnhancedError::Unsupported(msg) => write!(f, "Unsupported: {}", msg),
            EnhancedError::NotImplemented(msg) => write!(f, "Not implemented: {}", msg),
            EnhancedError::Generic(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for EnhancedError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            EnhancedError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for EnhancedError {
    fn from(err: std::io::Error) -> Self {
        EnhancedError::Io(err)
    }
}

impl From<crate::fitz::error::Error> for EnhancedError {
    fn from(err: crate::fitz::error::Error) -> Self {
        EnhancedError::Generic(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = EnhancedError::InvalidParameter("test".into());
        assert_eq!(err.to_string(), "Invalid parameter: test");

        let err = EnhancedError::Unsupported("feature".into());
        assert_eq!(err.to_string(), "Unsupported: feature");

        let err = EnhancedError::Generic("message".into());
        assert_eq!(err.to_string(), "message");
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let enhanced_err: EnhancedError = io_err.into();

        match enhanced_err {
            EnhancedError::Io(_) => {}
            _ => panic!("Expected Io error"),
        }
    }

    #[test]
    fn test_fitz_error_conversion() {
        let fitz_err = crate::fitz::error::Error::Generic("fitz error".into());
        let enhanced_err: EnhancedError = fitz_err.into();

        match enhanced_err {
            EnhancedError::Generic(msg) => assert!(msg.contains("fitz error")),
            _ => panic!("Expected Generic error"),
        }
    }
}
