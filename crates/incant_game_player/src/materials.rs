//! Explicit, data-only offline recording and load admission. No native cache or
//! executable shader bytes can be supplied through these options.
use crate::session::{Result, Settings};
#[cfg(feature = "render-input")]
pub(crate) type Manifest = incant_render_data::prewarm::MaterialManifest;
#[cfg(not(feature = "render-input"))]
pub(crate) enum Manifest {}
#[derive(Default, Debug)]
pub(crate) struct Options {
    pub record: Option<String>,
    load: Option<String>,
    program: Option<String>,
    samples: Option<String>,
}
impl Options {
    pub fn split(args: Vec<String>) -> Result<(Vec<String>, Self)> {
        let mut result = Vec::new();
        let mut value = Self::default();
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            let slot = match arg.as_str() {
                "--record-materials" => &mut value.record,
                "--prewarm-materials" => &mut value.load,
                "--material-program" => &mut value.program,
                "--material-samples" => &mut value.samples,
                _ => {
                    result.push(arg);
                    continue;
                }
            };
            if slot.is_some() {
                return Err("duplicate material option".into());
            }
            let text = args.next().ok_or("material option needs a value")?;
            if text.is_empty() || text.starts_with("--") || text.chars().any(char::is_control) {
                return Err("invalid material option value".into());
            }
            *slot = Some(text);
        }
        if value.record.is_some() && value.load.is_some() {
            return Err("recording and prewarm are separate material runs".into());
        }
        if value.record.is_none() && (value.program.is_some() || value.samples.is_some()) {
            return Err("material profile options require --record-materials".into());
        }
        if value.enabled()
            && result.iter().any(|arg| {
                matches!(
                    arg.as_str(),
                    "--window" | "--build-info" | "--load-checkpoint" | "--save-checkpoint"
                )
            })
        {
            return Err("recorded materials require a fresh offline run".into());
        }
        if value
            .program
            .as_ref()
            .is_some_and(|p| !matches!(p.as_str(), "wgsl" | "cooked"))
            || value
                .samples
                .as_ref()
                .is_some_and(|p| !matches!(p.as_str(), "1" | "4"))
        {
            return Err("material profile must select wgsl|cooked and samples 1|4".into());
        }
        if let Some(path) = &value.record {
            match std::fs::symlink_metadata(path) {
                Ok(_) => return Err("material recording output already exists".into()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.into()),
            }
        }
        #[cfg(not(feature = "render-input"))]
        if value.enabled() {
            return Err("material recording requires render-input".into());
        }
        Ok((result, value))
    }
    fn enabled(&self) -> bool {
        self.record.is_some() || self.load.is_some()
    }
    #[cfg(feature = "render-input")]
    fn recording_program(&self) -> incant_render_data::prewarm::MaterialProgram {
        use incant_render_data::prewarm::MaterialProgram;
        match self.program.as_deref() {
            Some("cooked") => MaterialProgram::Cooked,
            Some("wgsl") => MaterialProgram::Wgsl,
            _ => {
                #[cfg(feature = "render-gpu")]
                return incant_render::Renderer::material_program();
                #[cfg(not(feature = "render-gpu"))]
                MaterialProgram::Wgsl
            }
        }
    }
    pub fn configure(
        &self,
        settings: Option<Settings>,
        scene: &incant_runtime::CookedScene,
        script: &incant_script::SessionProgram,
        input: &[u8],
    ) -> Result<Option<Settings>> {
        #[cfg(feature = "render-input")]
        if self.enabled() {
            use incant_render_data::{material_keys::MaterialSamples, prewarm::*};
            use sha2::{Digest, Sha256};
            if self.record.is_some() {
                let Some(Settings::Cpu(settings)) = settings else {
                    return Err("material recording requires the CPU render-input host".into());
                };
                let context = RecordingContext {
                    scene: scene.id().0,
                    cooked_scene_sha256: Sha256::digest(scene.to_bytes()).into(),
                    profile: MaterialProfile {
                        samples: if self.samples.as_deref() == Some("4") { MaterialSamples::Four } else { MaterialSamples::One },
                        program: self.recording_program(),
                    },
                    script_sha256: script.artifact_sha256().ok_or("material recording requires an identified bytecode or development module artifact")?,
                    input_sha256: Sha256::digest(input).into(),
                    tick_rate: scene.tick_rate(),
                };
                return Ok(Some(Settings::Recording(settings, context)));
            }
            #[cfg(feature = "render-gpu")]
            {
                // --prewarm-materials is the explicit GPU opt-in, also for the
                // package convenience form whose default spatial host is CPU.
                let Some(Settings::Cpu(settings) | Settings::Gpu(settings)) = settings else {
                    return Err("material prewarm requires an explicit spatial host".into());
                };
                if settings.ui.is_some() {
                    return Err("recorded offscreen UI is unsupported".into());
                }
                let bytes = crate::bounded(self.load.as_ref().unwrap(), MAX_MANIFEST_BYTES)?;
                let manifest = MaterialManifest::from_bytes(&bytes)?;
                return Ok(Some(Settings::RecordedGpu(settings, manifest)));
            }
            #[cfg(not(feature = "render-gpu"))]
            return Err("material prewarm requires the render-gpu build feature".into());
        }
        #[cfg(not(feature = "render-input"))]
        let _ = (scene, script, input);
        Ok(settings)
    }
    pub fn publish(&self, manifest: Option<&Manifest>) -> Result<Option<serde_json::Value>> {
        #[cfg(feature = "render-input")]
        if let Some(path) = &self.record {
            use std::io::Write;
            let manifest = manifest.ok_or("material recording owner is absent")?;
            let bytes = manifest.to_bytes()?;
            let path = std::path::Path::new(path);
            let parent = path
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(std::path::Path::new("."));
            let mut file = tempfile::Builder::new()
                .prefix(".incant-materials-")
                .tempfile_in(parent)?;
            file.write_all(&bytes)?;
            file.as_file().sync_all()?;
            file.persist_noclobber(path).map_err(|e| e.error)?;
            return Ok(Some(
                serde_json::json!({"sha256": incant_assets_runtime::sha256(&bytes),
                "keys":manifest.observed.len(),"first_tick":manifest.recording.first_tick,
                "last_tick":manifest.recording.last_tick,"gpu_submitted":false}),
            ));
        }
        #[cfg(not(feature = "render-input"))]
        let _ = manifest;
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|s| (*s).into()).collect()
    }
    #[test]
    fn material_options_are_explicit_and_conflicting_routes_fail() {
        let ordinary = args(&["scene", "cache", "clips"]);
        assert_eq!(Options::split(ordinary.clone()).unwrap().0, ordinary);
        for values in [
            vec!["--record-materials", "a", "--prewarm-materials", "b"],
            vec!["--material-samples", "4"],
            vec!["--prewarm-materials", "a", "--window"],
            vec!["--record-materials", "a", "--save-checkpoint", "b"],
            vec!["--record-materials", "a", "--material-program", "unknown"],
            vec!["--record-materials", "a", "--material-samples", "2"],
            vec!["--prewarm-materials", "a", "--prewarm-materials", "b"],
        ] {
            assert!(Options::split(args(&values)).is_err(), "{values:?}");
        }
    }
}
