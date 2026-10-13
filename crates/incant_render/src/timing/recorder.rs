use super::{
    pool::{Lease, Pool},
    *,
};
use std::sync::Arc;
const QUERIES: u32 = 2 * GPU_PASS_COUNT as u32;
const BYTES: u64 = QUERIES as u64 * 8;
struct Buffers {
    queries: wgpu::QuerySet,
    resolve: wgpu::Buffer,
    readback: wgpu::Buffer,
}
pub(crate) struct Profiler {
    buffers: [Arc<Buffers>; GPU_TIMING_CAPACITY],
    pool: Pool,
    period: f32,
    device: wgpu::Device,
    queue: wgpu::Queue,
}
pub(crate) struct Capture {
    buffers: Arc<Buffers>,
    lease: Lease,
    period: f32,
    pairs: [Option<u32>; GPU_PASS_COUNT],
    count: u32,
    device: wgpu::Device,
    queue: wgpu::Queue,
}
impl Profiler {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Result<Self, GpuTimingError> {
        required_features(true, device.features())?;
        let period = queue.get_timestamp_period();
        span(0, 0, period)?;
        let buffers = std::array::from_fn(|_| {
            Arc::new(Buffers {
                queries: device.create_query_set(&wgpu::QuerySetDescriptor {
                    label: Some("Incant bounded GPU timestamps"),
                    ty: wgpu::QueryType::Timestamp,
                    count: QUERIES,
                }),
                resolve: device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Incant timestamp resolve"),
                    size: BYTES,
                    usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                }),
                readback: device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Incant timestamp readback"),
                    size: BYTES,
                    usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }),
            })
        });
        Ok(Self {
            buffers,
            pool: Pool::default(),
            period,
            device: device.clone(),
            queue: queue.clone(),
        })
    }
    pub fn begin(&self) -> Result<Option<Capture>, GpuTimingError> {
        let Some(lease) = self.pool.reserve()? else {
            return Ok(None);
        };
        let buffers = Arc::clone(&self.buffers[lease.index]);
        Ok(Some(Capture {
            buffers,
            lease,
            period: self.period,
            pairs: [None; GPU_PASS_COUNT],
            count: 0,
            device: self.device.clone(),
            queue: self.queue.clone(),
        }))
    }
    pub fn take(&self) -> Result<Option<GpuFrameTiming>, GpuTimingError> {
        self.pool.take()?.transpose()
    }
    pub fn stats(&self) -> Result<GpuTimingStats, GpuTimingError> {
        self.pool.stats()
    }
}
/// One logical group may span several real passes (shadow cascades).
pub(crate) struct PassQueries {
    queries: wgpu::QuerySet,
    start: u32,
    end: u32,
}
impl PassQueries {
    pub fn render(&self) -> wgpu::RenderPassTimestampWrites<'_> {
        self.render_range(0, 1).expect("one complete pass")
    }
    pub fn render_range(
        &self,
        index: usize,
        count: usize,
    ) -> Option<wgpu::RenderPassTimestampWrites<'_>> {
        let (first, last) = range_edges(index, count)?;
        Some(wgpu::RenderPassTimestampWrites {
            query_set: &self.queries,
            beginning_of_pass_write_index: first.then_some(self.start),
            end_of_pass_write_index: last.then_some(self.end),
        })
    }
    pub fn compute(&self) -> wgpu::ComputePassTimestampWrites<'_> {
        wgpu::ComputePassTimestampWrites {
            query_set: &self.queries,
            beginning_of_pass_write_index: Some(self.start),
            end_of_pass_write_index: Some(self.end),
        }
    }
}
// Wgpu rejects Some(timestamp_writes) with neither index. Interior passes in a
// logical multi-pass group must omit the descriptor entirely.
pub(super) fn range_edges(index: usize, count: usize) -> Option<(bool, bool)> {
    if index >= count {
        return None;
    }
    let edges = (index == 0, index + 1 == count);
    (edges.0 || edges.1).then_some(edges)
}
impl Capture {
    pub fn pass(&mut self, pass: GpuPass) -> PassQueries {
        debug_assert!(self.pairs[pass.index()].is_none());
        self.pairs[pass.index()] = Some(self.count);
        let queries = PassQueries {
            queries: self.buffers.queries.clone(),
            start: self.count,
            end: self.count + 1,
        };
        self.count += 2;
        queries
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub fn finish(self, encoder: &mut wgpu::CommandEncoder) {
        // The frame owns this callback. Abandoning it drops the lease; actual
        // completion keeps the exact device/query owner alive for deferred resolve.
        encoder.on_submitted_work_done(move || self.resolve());
    }
    #[cfg(not(target_arch = "wasm32"))]
    fn resolve(self) {
        let Self {
            buffers,
            lease,
            period,
            pairs,
            count,
            device,
            queue,
        } = self;
        // Metal stage counters become visible after the sampled command buffer
        // finishes. Encode and submit the resolve from the completion callback,
        // after query initialization is known to the queue. Never wait in the frame.
        let mut resolve = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Incant completed timestamp resolve"),
        });
        resolve.resolve_query_set(&buffers.queries, 0..count, &buffers.resolve, 0);
        let bytes = u64::from(count) * 8;
        resolve.copy_buffer_to_buffer(&buffers.resolve, 0, &buffers.readback, 0, bytes);
        let readback = buffers.readback.clone();
        resolve.map_buffer_on_submit(
            &buffers.readback,
            wgpu::MapMode::Read,
            0..bytes,
            move |result| {
                let sample = if result.is_ok() {
                    let view = readback.slice(0..bytes).get_mapped_range();
                    let result = decode(&view, lease.sequence, period, pairs, count);
                    drop(view);
                    readback.unmap();
                    result
                } else {
                    Err(GpuTimingError::Readback)
                };
                lease.complete(sample);
            },
        );
        queue.submit([resolve.finish()]);
    }
    #[cfg(target_arch = "wasm32")]
    pub fn finish(self, _encoder: &mut wgpu::CommandEncoder) {
        unreachable!("native GPU timing is refused before construction on WebAssembly");
    }
}
pub(super) fn decode(
    bytes: &[u8],
    sequence: u64,
    period: f32,
    pairs: [Option<u32>; GPU_PASS_COUNT],
    count: u32,
) -> Result<GpuFrameTiming, GpuTimingError> {
    if !(2..=QUERIES).contains(&count)
        || !count.is_multiple_of(2)
        || bytes.len() != count as usize * 8
    {
        return Err(GpuTimingError::Payload);
    }
    let mut ticks = [0u64; QUERIES as usize];
    for (slot, chunk) in ticks.iter_mut().zip(bytes.as_chunks::<8>().0) {
        *slot = u64::from_ne_bytes(*chunk);
    }
    // An entire zero-filled payload is indistinguishable from unwritten query
    // storage. Refuse it rather than publishing a fabricated zero-cost frame.
    // Equal nonzero endpoints remain valid: short intervals can quantize to zero.
    if ticks[..count as usize].iter().all(|&tick| tick == 0) {
        return Err(GpuTimingError::Unwritten);
    }
    // Independent compute/render passes can overlap or begin out of encoding
    // order. Their envelope covers all measured work without counting overlap
    // twice or assuming the first/last encoded endpoints bound execution.
    let mut first = u64::MAX;
    let mut last = 0;
    for pair in ticks[..count as usize].as_chunks::<2>().0 {
        span(pair[0], pair[1], period)?;
        first = first.min(pair[0]);
        last = last.max(pair[1]);
    }
    // Min/max cannot distinguish a wrap between individually ordered pairs
    // from an implausibly wide interval. Refuse ambiguous 64-bit envelopes.
    if last - first > u64::MAX / 2 {
        return Err(GpuTimingError::Reversed);
    }
    let mut passes = [None; GPU_PASS_COUNT];
    for (pass, pair) in passes.iter_mut().zip(pairs) {
        if let Some(index) = pair {
            if !index.is_multiple_of(2) || index >= count {
                return Err(GpuTimingError::Payload);
            }
            *pass = Some(span(
                ticks[index as usize],
                ticks[index as usize + 1],
                period,
            )?);
        }
    }
    Ok(GpuFrameTiming {
        sequence,
        timestamp_period_ns: period,
        frame: span(first, last, period)?,
        passes,
    })
}
