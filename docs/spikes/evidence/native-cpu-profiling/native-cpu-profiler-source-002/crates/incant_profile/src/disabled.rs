use crate::{Error, HeapSample, Phase, Report};
#[derive(Default)]
pub struct Scope;
#[inline(always)]
pub fn scope(_: Phase) -> Scope {
    Scope
}
#[inline(always)]
pub fn script_scope(_: Option<[u8; 16]>) -> Scope {
    Scope
}
#[inline(always)]
pub fn enabled() -> bool {
    false
}
#[inline(always)]
pub fn quickjs_heap(_: HeapSample) {}
#[inline(always)]
pub fn frame_outcome(_: bool) {}
pub struct Capture;
impl Capture {
    pub fn start() -> Result<Self, Error> {
        Err(Error::Disabled)
    }
    pub fn start_tracy_process() -> Result<Self, Error> {
        Err(Error::TracyUnavailable)
    }
    pub fn report(&self) -> Report {
        unreachable!("disabled capture cannot be constructed")
    }
}
