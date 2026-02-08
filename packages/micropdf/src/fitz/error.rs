//! Error handling for MicroPDF

use std::io;
use thiserror::Error;

/// The main error type for MicroPDF operations
#[derive(Error, Debug)]
pub enum Error {
    #[error("{0}")]
    Generic(String),
    #[error("System error: {0}")]
    System(#[from] io::Error),
    #[error("Invalid argument: {0}")]
    Argument(String),
    #[error("Limit exceeded: {0}")]
    Limit(String),
    #[error("Unsupported: {0}")]
    Unsupported(String),
    #[error("Format error: {0}")]
    Format(String),
    #[error("Syntax error: {0}")]
    Syntax(String),
    #[error("PDF error: {0}")]
    Pdf(String),
    #[error("Encryption error: {0}")]
    Encryption(String),
    #[error("Font error: {0}")]
    Font(String),
    #[error("Image error: {0}")]
    Image(String),
    #[error("Unexpected end of file")]
    Eof,
    #[error("Operation aborted")]
    Abort,
}

impl Error {
    pub fn generic<S: Into<String>>(msg: S) -> Self {
        Error::Generic(msg.into())
    }
    pub fn argument<S: Into<String>>(msg: S) -> Self {
        Error::Argument(msg.into())
    }
    pub fn limit<S: Into<String>>(msg: S) -> Self {
        Error::Limit(msg.into())
    }
    pub fn unsupported<S: Into<String>>(msg: S) -> Self {
        Error::Unsupported(msg.into())
    }
    pub fn format<S: Into<String>>(msg: S) -> Self {
        Error::Format(msg.into())
    }
    pub fn syntax<S: Into<String>>(msg: S) -> Self {
        Error::Syntax(msg.into())
    }
    pub fn pdf<S: Into<String>>(msg: S) -> Self {
        Error::Pdf(msg.into())
    }
    pub fn encryption<S: Into<String>>(msg: S) -> Self {
        Error::Encryption(msg.into())
    }
    pub fn font<S: Into<String>>(msg: S) -> Self {
        Error::Font(msg.into())
    }
    pub fn image<S: Into<String>>(msg: S) -> Self {
        Error::Image(msg.into())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_generic() {
        let e = Error::generic("test error");
        assert!(matches!(e, Error::Generic(_)));
        assert_eq!(format!("{}", e), "test error");
    }

    #[test]
    fn test_error_argument() {
        let e = Error::argument("bad argument");
        assert!(matches!(e, Error::Argument(_)));
        assert!(format!("{}", e).contains("bad argument"));
    }

    #[test]
    fn test_error_limit() {
        let e = Error::limit("size exceeded");
        assert!(matches!(e, Error::Limit(_)));
        assert!(format!("{}", e).contains("size exceeded"));
    }

    #[test]
    fn test_error_unsupported() {
        let e = Error::unsupported("feature not supported");
        assert!(matches!(e, Error::Unsupported(_)));
        assert!(format!("{}", e).contains("feature not supported"));
    }

    #[test]
    fn test_error_format() {
        let e = Error::format("invalid format");
        assert!(matches!(e, Error::Format(_)));
        assert!(format!("{}", e).contains("invalid format"));
    }

    #[test]
    fn test_error_syntax() {
        let e = Error::syntax("syntax error at line 5");
        assert!(matches!(e, Error::Syntax(_)));
        assert!(format!("{}", e).contains("syntax error"));
    }

    #[test]
    fn test_error_pdf() {
        let e = Error::pdf("invalid PDF structure");
        assert!(matches!(e, Error::Pdf(_)));
        assert!(format!("{}", e).contains("invalid PDF"));
    }

    #[test]
    fn test_error_encryption() {
        let e = Error::encryption("wrong password");
        assert!(matches!(e, Error::Encryption(_)));
        assert!(format!("{}", e).contains("wrong password"));
    }

    #[test]
    fn test_error_font() {
        let e = Error::font("font not found");
        assert!(matches!(e, Error::Font(_)));
        assert!(format!("{}", e).contains("font not found"));
    }

    #[test]
    fn test_error_image() {
        let e = Error::image("corrupted image");
        assert!(matches!(e, Error::Image(_)));
        assert!(format!("{}", e).contains("corrupted image"));
    }

    #[test]
    fn test_error_eof() {
        let e = Error::Eof;
        assert!(matches!(e, Error::Eof));
        assert!(format!("{}", e).contains("end of file"));
    }

    #[test]
    fn test_error_abort() {
        let e = Error::Abort;
        assert!(matches!(e, Error::Abort));
        assert!(format!("{}", e).contains("aborted"));
    }

    #[test]
    fn test_error_from_io_error() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let e: Error = io_err.into();
        assert!(matches!(e, Error::System(_)));
        assert!(format!("{}", e).contains("file not found"));
    }

    #[test]
    fn test_error_debug() {
        let e = Error::generic("test");
        let debug = format!("{:?}", e);
        assert!(debug.contains("Generic"));
    }

    #[test]
    fn test_result_type() {
        fn returns_ok() -> Result<i32> {
            Ok(42)
        }

        fn returns_err() -> Result<i32> {
            Err(Error::generic("error"))
        }

        assert_eq!(returns_ok().unwrap(), 42);
        assert!(returns_err().is_err());
    }
}
