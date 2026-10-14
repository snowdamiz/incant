//! Observation only: no allocator, GC threshold, hard limit or guest change.
use incant_profile::{HeapMode, HeapSample};
pub(crate) fn observe(runtime: &rquickjs::Runtime) {
    let Some(request) = incant_profile::quickjs_observation() else {
        return;
    };
    let sample = (|| -> Option<HeapSample> {
        match request.mode() {
            HeapMode::Maintained => {
                let h = runtime.malloc_state();
                Some(HeapSample {
                    allocated_bytes: h.allocated_bytes.try_into().ok()?,
                    allocation_count: h.allocation_count.try_into().ok()?,
                    limit_bytes: h.limit_bytes.try_into().ok()?,
                    estimated_used_bytes: None,
                })
            }
            HeapMode::Detailed => {
                let h = runtime.memory_usage();
                Some(HeapSample {
                    allocated_bytes: h.malloc_size.try_into().ok()?,
                    allocation_count: h.malloc_count.try_into().ok()?,
                    limit_bytes: h.malloc_limit.try_into().ok()?,
                    estimated_used_bytes: Some(h.memory_used_size.try_into().ok()?),
                })
            }
        }
    })();
    request.record(sample);
}
