//! Data-only string tables and bounded ICU MessageFormat-subset evaluation.
//! No platform, filesystem, network, rendering or JavaScript dependency.
mod catalog;
mod contracts;
mod exchange;
mod format;
pub use exchange::{MAX_XLIFF_BYTES, TranslationUpdate, export_xliff, translations_from_xliff};
mod message;
mod types;
pub use catalog::Catalog;
pub use format::{format_date, format_number};
use thiserror::Error;
pub use types::*;

pub const MAX_TABLES: usize = 64;
pub const MAX_MESSAGES: usize = 4096;
pub const MAX_LOCALES: usize = 32;
pub const MAX_CATALOG_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_MESSAGE_BYTES: usize = 8192;
pub const MAX_OUTPUT_BYTES: usize = 64 * 1024;
pub const MAX_ARGUMENTS: usize = 64;

#[derive(Debug, Error)]
pub enum LocalizationError {
    #[error("localization validation at {path}: {message}")]
    Validation { path: String, message: String },
    #[error("invalid message at byte {offset}: {message}")]
    Pattern { offset: usize, message: String },
    #[error("missing or invalid message argument: {0}")]
    Argument(String),
    #[error("unknown string table: {0}")]
    Table(String),
    #[error("localized output exceeds 64 KiB")]
    OutputLimit,
    #[error("locale formatting failed: {0}")]
    Format(String),
}
fn invalid(path: impl Into<String>, message: impl Into<String>) -> LocalizationError {
    LocalizationError::Validation {
        path: path.into(),
        message: message.into(),
    }
}
