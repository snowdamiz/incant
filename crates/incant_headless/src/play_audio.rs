//! Bounded offline audio export; no device, credentials or pixel capture.
use crate::play::PlayError;
use incant_assets::AssetStore;
use incant_audio::{OfflineBackend, SceneAudio};
use incant_core::RuntimeSnapshot;
use incant_doc::Project;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

const MAX_PCM_BYTES: u64 = 256 * 1024 * 1024;
#[derive(Serialize)]
pub struct Report {
    file: PathBuf,
    format: &'static str,
    sample_rate: u32,
    channels: u32,
    frames: u64,
    peak: f32,
    samples_at_full_scale: u64,
    pcm_sha256: String,
}
pub struct Output {
    scene: SceneAudio<OfflineBackend>,
    destination: PathBuf,
    staged: tempfile::NamedTempFile,
    sample_rate: u32,
    tick_rate: u32,
    total_frames: u64,
    frames: u64,
    peak: f32,
    full_scale: u64,
    hash: Sha256,
}
impl Output {
    pub fn new(
        path: &Path,
        project: &Project,
        snapshot: &RuntimeSnapshot,
        assets: &AssetStore,
        sample_rate: u32,
        ticks: u64,
        outputs: [Option<&Path>; 3],
    ) -> Result<Self, PlayError> {
        let frames = ticks
            .checked_mul(u64::from(sample_rate))
            .ok_or(PlayError::AudioBudget)?
            / u64::from(project.settings.tick_rate);
        let bytes = frames
            .checked_mul(8)
            .filter(|b| *b <= MAX_PCM_BYTES)
            .ok_or(PlayError::AudioBudget)?;
        let mut scene = SceneAudio::offline(sample_rate, project)?;
        scene.sync(project, snapshot, assets)?;
        let parent = parent(path);
        fs::create_dir_all(parent)?;
        if fs::symlink_metadata(path).is_ok() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "audio output already exists",
            )
            .into());
        }
        let directory = parent.canonicalize()?;
        let filename = name(path);
        let [frames_dir, save, logs] = outputs;
        if frames_dir.is_some_and(|p| {
            p.canonicalize().is_ok_and(|d| d == directory)
                && (filename == "report.json"
                    || (filename.starts_with("frame-") && filename.ends_with(".png")))
        }) {
            return Err(PlayError::AudioOutputConflict);
        }
        for other in [save, logs].into_iter().flatten() {
            fs::create_dir_all(self::parent(other))?;
            if directory == self::parent(other).canonicalize()? && filename == name(other) {
                return Err(PlayError::AudioOutputConflict);
            }
        }
        let mut staged = tempfile::NamedTempFile::new_in(parent)?;
        // RIFF/WAVE IEEE float with a fact chunk. Kira's final [-1,1] output is preserved.
        staged.write_all(b"RIFF")?;
        staged.write_all(&(bytes as u32 + 48).to_le_bytes())?;
        staged.write_all(b"WAVEfmt ")?;
        staged.write_all(&16_u32.to_le_bytes())?;
        staged.write_all(&3_u16.to_le_bytes())?;
        staged.write_all(&2_u16.to_le_bytes())?;
        staged.write_all(&sample_rate.to_le_bytes())?;
        staged.write_all(&(sample_rate * 8).to_le_bytes())?;
        staged.write_all(&8_u16.to_le_bytes())?;
        staged.write_all(&32_u16.to_le_bytes())?;
        staged.write_all(b"fact")?;
        staged.write_all(&4_u32.to_le_bytes())?;
        staged.write_all(&(frames as u32).to_le_bytes())?;
        staged.write_all(b"data")?;
        staged.write_all(&(bytes as u32).to_le_bytes())?;
        Ok(Self {
            scene,
            destination: path.to_path_buf(),
            staged,
            sample_rate,
            tick_rate: project.settings.tick_rate,
            total_frames: frames,
            frames: 0,
            peak: 0.,
            full_scale: 0,
            hash: Sha256::new(),
        })
    }
    pub fn tick(
        &mut self,
        relative_tick: u64,
        project: &Project,
        snapshot: &RuntimeSnapshot,
        assets: &AssetStore,
    ) -> Result<(), PlayError> {
        if project.settings.tick_rate != self.tick_rate {
            return Err(incant_audio::AudioError::Unsupported(
                "changing the audio tick rate requires restarting playback",
            )
            .into());
        }
        self.scene.sync(project, snapshot, assets)?;
        let target = relative_tick * u64::from(self.sample_rate) / u64::from(self.tick_rate);
        if target < self.frames || target > self.total_frames {
            return Err(PlayError::AudioBudget);
        }
        let mut samples = [0_f32; 2048];
        let mut bytes = [0_u8; 8192];
        while self.frames < target {
            let count = (target - self.frames).min(1024) as usize;
            self.scene.render(&mut samples[..count * 2])?;
            for (sample, bytes) in samples[..count * 2]
                .iter()
                .zip(bytes.as_chunks_mut::<4>().0.iter_mut())
            {
                if !sample.is_finite() {
                    return Err(
                        incant_audio::AudioError::Invalid("mixed output is nonfinite").into(),
                    );
                }
                self.peak = self.peak.max(sample.abs());
                self.full_scale += u64::from(sample.abs() >= 1.);
                bytes.copy_from_slice(&sample.to_le_bytes());
            }
            self.staged.write_all(&bytes[..count * 8])?;
            self.hash.update(&bytes[..count * 8]);
            self.frames += count as u64;
        }
        Ok(())
    }
    pub fn finish(self) -> Result<Report, PlayError> {
        if self.frames != self.total_frames {
            return Err(PlayError::AudioBudget);
        }
        self.staged.as_file().sync_all()?;
        self.staged
            .persist_noclobber(&self.destination)
            .map_err(|e| e.error)?;
        Ok(Report {
            file: self.destination,
            format: "wav-f32-le",
            sample_rate: self.sample_rate,
            channels: 2,
            frames: self.frames,
            peak: self.peak,
            samples_at_full_scale: self.full_scale,
            pcm_sha256: format!("{:x}", self.hash.finalize()),
        })
    }
}
fn parent(path: &Path) -> &Path {
    path.parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
}
fn name(path: &Path) -> String {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_ascii_lowercase()
}
