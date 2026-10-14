//! QuickJS telemetry is sampled per VM after a script tick attempt, not summed
//! across VMs. Logical allocation counters exclude backing-arena/RSS accounting;
//! estimated used size enumerates objects and can include externally owned data.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "summary", derive(serde::Serialize))]
#[cfg_attr(feature = "summary", serde(rename_all = "snake_case"))]
pub enum HeapMode {
    #[default]
    Maintained,
    Detailed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "summary", derive(serde::Serialize))]
pub struct HeapSample {
    pub allocated_bytes: u64,
    pub allocation_count: u64,
    /// Zero means unlimited in the pinned QuickJS allocator API.
    pub limit_bytes: u64,
    /// Missing in maintained mode; never substitute logical or backing bytes.
    pub estimated_used_bytes: Option<u64>,
}
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "summary", derive(serde::Serialize))]
pub struct QuickJsHeap {
    pub version: u32,
    pub mode: HeapMode,
    pub sampling: &'static str,
    pub samples: u64,
    pub invalid_samples: u64,
    pub detailed_samples: u64,
    pub latest: Option<HeapSample>,
    /// Maxima over valid observations only; transient peaks can be missed.
    pub sampled_peak_allocated_bytes: Option<u64>,
    pub sampled_peak_allocation_count: Option<u64>,
    pub sampled_peak_estimated_used_bytes: Option<u64>,
}
impl QuickJsHeap {
    pub(crate) const fn empty(mode: HeapMode) -> Self {
        Self {
            version: 1,
            mode,
            sampling: "script_tick_end_per_vm",
            samples: 0,
            invalid_samples: 0,
            detailed_samples: 0,
            latest: None,
            sampled_peak_allocated_bytes: None,
            sampled_peak_allocation_count: None,
            sampled_peak_estimated_used_bytes: None,
        }
    }
    #[cfg(feature = "summary")]
    pub(crate) fn record(&mut self, sample: Option<HeapSample>) -> Option<HeapSample> {
        let Some(sample) = sample
            .filter(|s| s.estimated_used_bytes.is_some() == (self.mode == HeapMode::Detailed))
        else {
            self.invalid_samples = self.invalid_samples.saturating_add(1);
            return None;
        };
        self.samples = self.samples.saturating_add(1);
        self.latest = Some(sample);
        fn update(peak: &mut Option<u64>, value: u64) {
            *peak = Some(peak.map_or(value, |old| old.max(value)));
        }
        update(
            &mut self.sampled_peak_allocated_bytes,
            sample.allocated_bytes,
        );
        update(
            &mut self.sampled_peak_allocation_count,
            sample.allocation_count,
        );
        if let Some(used) = sample.estimated_used_bytes {
            self.detailed_samples = self.detailed_samples.saturating_add(1);
            update(&mut self.sampled_peak_estimated_used_bytes, used);
        }
        Some(sample)
    }
}
impl Default for QuickJsHeap {
    fn default() -> Self {
        Self::empty(HeapMode::Maintained)
    }
}
