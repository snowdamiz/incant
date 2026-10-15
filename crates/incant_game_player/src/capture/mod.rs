//! Explicit sampling of the sole CPU-composed player's accepted boundaries.
//! Flags are host authority, never package-manifest or project shader data.
#[cfg(any(feature = "frame-capture", test))]
mod output;
#[cfg(feature = "frame-capture")]
mod render;
use crate::session::Result;
#[cfg(feature = "frame-capture")]
pub use render::Capture;
use std::path::PathBuf;

const MAX_FRAMES: usize = 128;
const MAX_RAW_BYTES: u64 = 256 * 1024 * 1024;
#[derive(Debug)]
pub struct Options {
    #[cfg_attr(not(feature = "frame-capture"), allow(dead_code))]
    pub directory: PathBuf,
    pub steps: Vec<u64>,
    #[cfg_attr(not(feature = "frame-capture"), allow(dead_code))]
    pub size: [u32; 2],
}
impl Options {
    pub fn split(args: Vec<String>) -> Result<(Vec<String>, Option<Self>)> {
        Self::split_enabled(args, cfg!(feature = "frame-capture"))
    }
    fn split_enabled(args: Vec<String>, enabled: bool) -> Result<(Vec<String>, Option<Self>)> {
        let selected = args.iter().any(|a| {
            matches!(
                a.as_str(),
                "--capture-frames" | "--capture-at" | "--capture-size"
            )
        });
        if !selected {
            return Ok((args, None));
        }
        if !enabled {
            return Err("selected-frame capture requires the frame-capture build feature".into());
        }
        if args.iter().any(|a| {
            matches!(
                a.as_str(),
                "--window"
                    | "--gpu-offscreen"
                    | "--load-checkpoint"
                    | "--save-checkpoint"
                    | "--static-module"
                    | "--build-info"
                    | "--record-materials"
                    | "--prewarm-materials"
            )
        }) {
            return Err("selected-frame capture requires the offline bytecode CPU spatial route without checkpoints".into());
        }
        let mut ordinary = Vec::new();
        let mut fields = [None, None, None];
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            let index = match arg.as_str() {
                "--capture-frames" => 0,
                "--capture-at" => 1,
                "--capture-size" => 2,
                _ => {
                    ordinary.push(arg);
                    continue;
                }
            };
            if fields[index].is_some() {
                return Err("duplicate capture option".into());
            }
            let value = args.next().ok_or("capture option requires a value")?;
            if value.is_empty() || value.starts_with("--") || value.chars().any(char::is_control) {
                return Err("invalid capture option value".into());
            }
            fields[index] = Some(value);
        }
        let [directory, selection, size] = fields;
        let directory =
            PathBuf::from(directory.ok_or("capture requires --capture-frames NEW_DIR")?);
        let selection = selection.ok_or("capture requires --capture-at HOST_STEP,...")?;
        if selection.len() > 1024 {
            return Err("capture selection size limit".into());
        }
        let mut steps = Vec::new();
        for part in selection.split(',') {
            if part.is_empty() || !part.bytes().all(|c| c.is_ascii_digit()) {
                return Err("capture steps must be unsigned host-step integers".into());
            }
            let step = part.parse()?;
            if step > 3600 || steps.contains(&step) || steps.len() == MAX_FRAMES {
                return Err("capture steps must be unique, within 0..=3600 and at most 128".into());
            }
            steps.push(step);
        }
        steps.sort_unstable();
        let size = size.ok_or("capture requires --capture-size WIDTHxHEIGHT")?;
        let (width, height) = size
            .split_once('x')
            .ok_or("capture size must be WIDTHxHEIGHT")?;
        if [width, height]
            .iter()
            .any(|s| s.is_empty() || !s.bytes().all(|c| c.is_ascii_digit()))
        {
            return Err("capture size must be integer physical pixels".into());
        }
        let size: [u32; 2] = [width.parse()?, height.parse()?];
        let raw = Self::raw_bytes(size)?;
        if raw
            .checked_mul(steps.len() as u64)
            .is_none_or(|n| n > MAX_RAW_BYTES)
        {
            return Err("selected capture raw bytes exceed 256 MiB".into());
        }
        Ok((
            ordinary,
            Some(Self {
                directory,
                steps,
                size,
            }),
        ))
    }
    fn raw_bytes([width, height]: [u32; 2]) -> Result<u64> {
        if width == 0 || height == 0 || width > 4096 || height > 4096 {
            return Err("capture dimensions must be within 1..=4096".into());
        }
        Ok(u64::from(width) * u64::from(height) * 4)
    }
    pub fn admit_ticks(&self, ticks: u64) -> Result<()> {
        if self.steps.last().is_none_or(|last| *last > ticks) {
            return Err("capture selection exceeds requested host steps".into());
        }
        Ok(())
    }
    pub fn admit_route(
        &self,
        args: &[String],
        checkpoint: &crate::checkpoint::Options,
    ) -> Result<()> {
        if args.get(8).map(String::as_str) != Some("--cpu-render-input")
            || checkpoint.load.is_some()
            || checkpoint.save.is_some()
        {
            return Err("selected-frame capture requires the offline bytecode CPU spatial route without checkpoints".into());
        }
        Ok(())
    }
    #[cfg(any(feature = "frame-capture", test))]
    fn complete(&self, captured: impl Iterator<Item = u64>) -> Result<()> {
        if captured.ne(self.steps.iter().copied()) {
            return Err("capture incomplete; complete manifest withheld".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
