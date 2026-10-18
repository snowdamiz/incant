//! Copied localization values. Returned text borrows caller-owned result scratch,
//! so it can outlive the callback without retaining a VM or a native pointer.
//! Missing-string requested/resolved locale are the enclosing result's fields.
pub use incant_wasm_abi::localization::report::{
    Entry as ReportEntry, Request as ReportRequest, Snapshot as ReportSnapshot,
};
pub use incant_wasm_abi::localization::{
    Argument, CalendarDate, DateLength, LocaleSettings, LocaleSnapshot, LocalizeRequest,
    LocalizedText, MessageArgument, MissingKind, MissingString,
};
pub use incant_wasm_abi::{
    LOCALIZATION_MAX_REQUEST_BYTES, LOCALIZATION_MAX_RESULT_BYTES, LOCALIZATION_MAX_SETTINGS_BYTES,
};
pub(crate) fn error(error: incant_wasm_abi::localization::Error) -> crate::Error {
    match error {
        incant_wasm_abi::localization::Error::Bounds => crate::Error::Bounds,
        incant_wasm_abi::localization::Error::Malformed => crate::Error::Malformed,
    }
}
