use super::{
    pool::{Lease, Pool},
    *,
};
use std::sync::Arc;
const QUERIES: u32 = 2 + 2 * GPU_PASS_COUNT as u32;
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
}
pub(crate) struct Capture {
    buffers: Arc<Buffers>,
    lease: Lease,
    period: f32,
    pairs: [Option<u32>; GPU_PASS_COUNT],
    count: u32,
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
        })
    }
    pub fn begin(
        &self,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<Option<Capture>, GpuTimingError> {
        let Some(lease) = self.pool.reserve()? else {
            return Ok(None);
        };
        let buffers = Arc::clone(&self.buffers[lease.index]);
        encoder.write_timestamp(&buffers.queries, 0);
        Ok(Some(Capture {
            buffers,
            lease,
            period: self.period,
            pairs: [None; GPU_PASS_COUNT],
            count: 2,
        }))
    }
    pub fn take(&self) -> Result<Option<GpuFrameTiming>, GpuTimingError> {
        self.pool.take()?.transpose()
    }
    pub fn stats(&self) -> Result<GpuTimingStats, GpuTimingError> {
        self.pool.stats()
    }
}
impl Capture {
    pub fn start(&mut self, encoder: &mut wgpu::CommandEncoder, pass: GpuPass) {
        debug_assert!(self.pairs[pass.index()].is_none());
        self.pairs[pass.index()] = Some(self.count);
        encoder.write_timestamp(&self.buffers.queries, self.count);
        self.count += 2;
    }
    pub fn end(&self, encoder: &mut wgpu::CommandEncoder, pass: GpuPass) {
        if let Some(start) = self.pairs[pass.index()] {
            encoder.write_timestamp(&self.buffers.queries, start + 1);
        }
    }
    pub fn finish(self, encoder: &mut wgpu::CommandEncoder) {
        let Self {
            buffers,
            lease,
            period,
            pairs,
            count,
        } = self;
        encoder.write_timestamp(&buffers.queries, 1);
        encoder.resolve_query_set(&buffers.queries, 0..count, &buffers.resolve, 0);
        let bytes = u64::from(count) * 8;
        encoder.copy_buffer_to_buffer(&buffers.resolve, 0, &buffers.readback, 0, bytes);
        let readback = buffers.readback.clone();
        // Deferred mapping is owned by the command buffer. Dropping an unsubmitted
        // command buffer drops the lease and makes its fixed slot reusable.
        encoder.map_buffer_on_submit(
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
    }
}
pub(super) fn decode(
    bytes: &[u8],
    sequence: u64,
    period: f32,
    pairs: [Option<u32>; GPU_PASS_COUNT],
    count: u32,
) -> Result<GpuFrameTiming, GpuTimingError> {
    if !(2..=QUERIES).contains(&count) || bytes.len() != count as usize * 8 {
        return Err(GpuTimingError::Payload);
    }
    let mut ticks = [0u64; QUERIES as usize];
    for (slot, chunk) in ticks.iter_mut().zip(bytes.as_chunks::<8>().0) {
        *slot = u64::from_ne_bytes(*chunk);
    }
    let mut passes = [None; GPU_PASS_COUNT];
    for (pass, pair) in passes.iter_mut().zip(pairs) {
        if let Some(index) = pair {
            if index < 2 || index + 1 >= count {
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
        frame: span(ticks[0], ticks[1], period)?,
        passes,
    })
}
