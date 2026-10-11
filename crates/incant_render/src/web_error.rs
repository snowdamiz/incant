//! Detach browser GPU diagnostics before crossing the Send + Sync error boundary.
use std::{error::Error, fmt};

/// The original WebGPU preparation failure category.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreparationErrorKind {
    OutOfMemory,
    Validation,
    Internal,
}

/// Owned diagnostics without a browser handle or a thread-confined error source.
/// Native builds retain `wgpu::Error` itself through the public type alias.
#[derive(Debug)]
pub struct PreparationError {
    kind: PreparationErrorKind,
    description: String,
    source: DiagnosticSource,
}
impl PreparationError {
    pub fn kind(&self) -> PreparationErrorKind {
        self.kind
    }
}
impl From<wgpu::Error> for PreparationError {
    fn from(error: wgpu::Error) -> Self {
        let (kind, description, source) = match error {
            wgpu::Error::OutOfMemory { source } => (
                PreparationErrorKind::OutOfMemory,
                "Out of Memory".into(),
                source,
            ),
            wgpu::Error::Validation {
                description,
                source,
            } => (PreparationErrorKind::Validation, description, source),
            wgpu::Error::Internal {
                description,
                source,
            } => (PreparationErrorKind::Internal, description, source),
        };
        Self {
            kind,
            description,
            source: DiagnosticSource::copy(source.as_ref()),
        }
    }
}
impl fmt::Display for PreparationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.description)
    }
}
impl Error for PreparationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}
#[derive(Debug)]
struct DiagnosticSource {
    description: String,
    source: Option<Box<Self>>,
}
impl DiagnosticSource {
    fn copy(error: &(dyn Error + 'static)) -> Self {
        Self {
            description: error.to_string(),
            source: error.source().map(|source| Box::new(Self::copy(source))),
        }
    }
}
impl fmt::Display for DiagnosticSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.description)
    }
}
impl Error for DiagnosticSource {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source.as_deref().map(|source| source as &dyn Error)
    }
}
#[cfg(test)]
#[path = "../tests/web_error/mod.rs"]
mod tests;
