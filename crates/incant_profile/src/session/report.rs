use super::*;
use serde::Serialize;

#[derive(Serialize)]
pub struct Report {
    pub format: &'static str,
    pub version: u32,
    pub scope: &'static str,
    pub capacity: usize,
    pub attempted_frames: u64,
    pub evicted_frames: u64,
    pub selected_frames: usize,
    pub failed_frames: usize,
    pub invalid_frames: usize,
    pub dropped_source_samples: u64,
    pub dropped_source_count_overflow: bool,
    pub frames: Vec<Frame>,
    pub zones: Vec<Zone>,
    pub sources: Vec<Source>,
}
#[derive(Serialize)]
pub struct Frame {
    pub attempt: u64,
    pub tick: u64,
    pub success: bool,
    pub invalid_phases: usize,
    pub wall_ns: Option<u64>,
    pub thread_cpu_ns: Option<u64>,
    pub script_wall_ns: Option<u64>,
}
#[derive(Serialize)]
pub struct Zone {
    pub name: &'static str,
    pub totals: Totals,
    pub valid_frames: usize,
    pub missing_frames: usize,
    pub invalid_frames: usize,
    pub p95_wall_ns: Option<u64>,
}
#[derive(Serialize)]
pub struct Source {
    pub source: Option<[u8; 16]>,
    pub totals: Totals,
    pub invalid_frames: usize,
}
fn total(metrics: impl Iterator<Item = Metric>) -> (Totals, usize) {
    let mut out = Totals::default();
    let mut invalid_frames = 0;
    for metric in metrics {
        let mut invalid = metric.invalid;
        sum(&mut out.calls, metric.totals.calls, &mut invalid);
        sum(&mut out.wall_ns, metric.totals.wall_ns, &mut invalid);
        sum(
            &mut out.thread_cpu_ns,
            metric.totals.thread_cpu_ns,
            &mut invalid,
        );
        sum(
            &mut out.unavailable_cpu_samples,
            metric.totals.unavailable_cpu_samples,
            &mut invalid,
        );
        sum(&mut out.unwound, metric.totals.unwound, &mut invalid);
        out.max_wall_ns = out.max_wall_ns.max(metric.totals.max_wall_ns);
        invalid_frames += usize::from(invalid);
    }
    (out, invalid_frames)
}
fn wall(metric: Metric) -> Option<u64> {
    (metric.totals.calls > 0 && !metric.invalid).then_some(metric.totals.wall_ns)
}
impl Recording {
    pub fn read(&self, frames: usize) -> Result<Report, Error> {
        let s = &self.0;
        let retained = s.attempted.min(MAX_FRAMES as u64) as usize;
        if frames == 0 || frames > retained {
            return Err(Error::FrameRange);
        }
        let first = s.attempted - frames as u64 + 1;
        let rows: Vec<_> = (first..=s.attempted)
            .map(|attempt| &s.rows[((attempt - 1) % MAX_FRAMES as u64) as usize])
            .collect();
        let failed = rows.iter().filter(|r| !r.finished || !r.success).count();
        let invalid = rows
            .iter()
            .filter(|r| r.phases.iter().chain(r.sources.iter()).any(|m| m.invalid))
            .count();
        let mut dropped_source_samples = 0;
        let mut dropped_source_count_overflow = false;
        for row in &rows {
            sum(
                &mut dropped_source_samples,
                row.dropped_sources,
                &mut dropped_source_count_overflow,
            );
        }
        let zones = Phase::ALL
            .into_iter()
            .enumerate()
            .map(|(i, phase)| {
                let (totals, invalid_frames) = total(rows.iter().map(|r| r.phases[i]));
                let missing_frames = rows
                    .iter()
                    .filter(|r| r.phases[i].totals.calls == 0)
                    .count();
                let mut values: Vec<_> = rows.iter().filter_map(|r| wall(r.phases[i])).collect();
                let valid_frames = values.len();
                let p95_wall_ns = if valid_frames == frames && invalid_frames == 0 && failed == 0 {
                    values.sort_unstable();
                    Some(values[(frames * 95).div_ceil(100) - 1])
                } else {
                    None
                };
                Zone {
                    name: phase.name(),
                    totals,
                    valid_frames,
                    missing_frames,
                    invalid_frames,
                    p95_wall_ns,
                }
            })
            .collect();
        let sources = s
            .sources
            .iter()
            .enumerate()
            .filter_map(|(i, id)| {
                let source = (*id)?;
                let (totals, invalid_frames) = total(rows.iter().map(|r| r.sources[i]));
                (totals.calls > 0 || invalid_frames > 0).then_some(Source {
                    source,
                    totals,
                    invalid_frames,
                })
            })
            .collect();
        Ok(Report {
            format: "incant-session-profile",
            version: 1,
            scope: "owner-thread synchronous native ticks; inclusive phase CPU/wall observations, not GPU/display or a shipping gate; allocations/heap and worker threads unavailable",
            capacity: MAX_FRAMES,
            attempted_frames: s.attempted,
            evicted_frames: s.attempted.saturating_sub(MAX_FRAMES as u64),
            selected_frames: frames,
            failed_frames: failed,
            invalid_frames: invalid,
            dropped_source_samples,
            dropped_source_count_overflow,
            frames: rows
                .iter()
                .map(|r| {
                    let frame = r.phases[Phase::Frame as usize];
                    Frame {
                        attempt: r.attempt,
                        tick: r.tick,
                        success: r.finished && r.success,
                        invalid_phases: r.phases.iter().filter(|m| m.invalid).count(),
                        wall_ns: wall(frame),
                        thread_cpu_ns: wall(frame)
                            .filter(|_| frame.totals.unavailable_cpu_samples == 0)
                            .map(|_| frame.totals.thread_cpu_ns),
                        script_wall_ns: wall(r.phases[Phase::ScriptTick as usize]),
                    }
                })
                .collect(),
            zones,
            sources,
        })
    }
}
