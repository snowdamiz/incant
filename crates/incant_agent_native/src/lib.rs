//! Reusable authoring-side native Play for trusted editor/headless agent hosts.
//!
//! Preparation cooks once through `incant_play`; retained sessions hold only
//! owned native state. This crate is not a shipped-player dependency. Callers
//! select the project root and fallback compiler; neither is agent-controlled.
mod controller;
mod host;
mod perception;
mod session;
pub use controller::{Controller, Error};
pub use host::Host;
pub use incant_play::StartOptions;
pub use perception::GpuPerception;
pub use session::{Session, SessionError};

/// Trusted engine function used only when the selected scene has no authored
/// attachments. The caller supplies the existing embedded behavior or its own
/// engine no-op; a project cannot choose a compiler, command or executable.
pub type FallbackCompiler =
    fn() -> Result<incant_script::CompiledScript, incant_script::ScriptError>;
