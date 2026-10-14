//! Native error identity is observed before the existing error mapping.
use super::compilation::{Call, ErrorDomain, ErrorIdentity, Observer};
use objc2::rc::Retained;
use objc2_foundation::{ns_string, NSError, NSString};
use std::time::Instant;

impl super::Device {
    /// Clone a diagnostic-only handle; it cannot retain this native device.
    pub fn compilation_observer(&self) -> Observer {
        self.compilation_observer.clone()
    }
}
impl Call {
    pub(super) fn finish_native<T>(self, result: &Result<T, Retained<NSError>>) {
        let ended = Instant::now();
        let error = result.as_ref().err().map(|error| {
            let domain = error.domain();
            let domain: &NSString = &domain;
            let domain = if domain == ns_string!("MTLLibraryErrorDomain") {
                ErrorDomain::Library
            } else if domain == ns_string!("MTLRenderPipelineErrorDomain") {
                ErrorDomain::RenderPipeline
            } else if domain == ns_string!("MTLComputePipelineErrorDomain") {
                ErrorDomain::ComputePipeline
            } else {
                ErrorDomain::Other
            };
            ErrorIdentity {
                domain,
                code: error.code() as i64,
            }
        });
        self.complete(ended, error);
    }
}
