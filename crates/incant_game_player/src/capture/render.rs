use super::{Options, Result, output::Output};
use crate::{pcm_output::PcmOutput, session::Player};
use incant_render::{FrameCapture, Renderer, RetainedScene, UiLayer};
use incant_runtime::WorldIdentity;
use serde::Serialize;

#[derive(Serialize)]
struct Receipt {
    host_step: u64,
    generation: u64,
    /// Run-local epoch whose opaque world identity is checked at each sample.
    world_epoch: u64,
    simulation_tick: u64,
    topology_generation: u64,
    binding_generation: u64,
    text_generation: u64,
    physical_size: [u32; 2],
    logical_viewport: Option<[f32; 2]>,
    ui_widgets: usize,
    ui_glyph_quads: usize,
    pcm_frames: u64,
    pcm_prefix_sha256: String,
    png: String,
    png_bytes: u64,
    png_sha256: String,
    rgba_sha256: String,
}
struct Scene {
    identity: WorldIdentity,
    generation: u64,
    topology: u64,
    binding: u64,
    text: u64,
    gpu: RetainedScene,
    ui: UiLayer,
}
pub struct Capture {
    options: Options,
    output: Output,
    renderer: Renderer,
    target: FrameCapture,
    scene: Option<Scene>,
    receipts: Vec<Receipt>,
    failed: bool,
}
impl Capture {
    pub fn new(options: Options, game: &Player) -> Result<Self> {
        let Player::Spatial(game) = game else {
            return Err("capture requires CPU spatial composition".into());
        };
        let host = game.host()?;
        if game.world()?.ui_view()?.nodes().next().is_some() && host.ui_input().is_none() {
            return Err("capture requires --ui-viewport for screen-space UI".into());
        }
        // Collision/admission precedes GPU setup. A later setup/render error may
        // retain partial.json but can never publish a completed manifest.
        let output = Output::create(&options)?;
        let renderer = Renderer::headless().map_err(|e| -> Box<dyn std::error::Error> { e })?;
        let target = FrameCapture::new(&renderer, options.size[0], options.size[1])
            .map_err(|e| -> Box<dyn std::error::Error> { e })?;
        let receipts = Vec::with_capacity(options.steps.len());
        Ok(Self {
            options,
            output,
            renderer,
            target,
            scene: None,
            receipts,
            failed: false,
        })
    }
    pub fn sample(
        &mut self,
        host_step: u64,
        generation: u64,
        game: &Player,
        pcm: &PcmOutput,
    ) -> Result<()> {
        if self.failed {
            return Err("capture is terminal after an earlier failure".into());
        }
        let result = self.sample_inner(host_step, generation, game, pcm);
        self.failed |= result.is_err();
        result
    }
    fn sample_inner(
        &mut self,
        host_step: u64,
        generation: u64,
        game: &Player,
        pcm: &PcmOutput,
    ) -> Result<()> {
        if self.options.steps.binary_search(&host_step).is_err() {
            return Ok(());
        }
        if self
            .receipts
            .last()
            .is_some_and(|r| r.host_step >= host_step)
        {
            return Err("capture chronology repeated or reversed".into());
        }
        let Player::Spatial(game) = game else {
            return Err("capture lost CPU spatial owner".into());
        };
        let world = game.world()?;
        let view = world.spatial_view()?;
        let host = game.host()?;
        let input = host.render_input();
        if input.tick() != world.tick() {
            return Err("capture input is not the accepted world boundary".into());
        }
        if world.ui_view()?.nodes().next().is_some() && host.ui_input().is_none() {
            return Err("capture cannot omit screen-space UI".into());
        }
        if let Some(old) = &self.scene
            && (generation < old.generation
                || ((generation == old.generation) != (&old.identity == view.identity())))
        {
            return Err("capture world/generation correspondence changed".into());
        }
        let replace = self.scene.as_ref().is_none_or(|s| {
            s.generation != generation
                || s.topology != input.topology_generation()
                || s.binding != input.binding_generation()
                || s.text != input.text_generation()
        });
        if replace {
            let gpu = self
                .renderer
                .prepare_retained(input.input(), input.camera_label())
                .map_err(|e| -> Box<dyn std::error::Error> { e })?;
            self.scene = Some(Scene {
                identity: view.identity().clone(),
                generation,
                topology: input.topology_generation(),
                binding: input.binding_generation(),
                text: input.text_generation(),
                gpu,
                ui: self.renderer.ui_layer(),
            });
        }
        let scene = self.scene.as_mut().ok_or("capture scene invariant")?;
        let rgba = self
            .target
            .render(
                &self.renderer,
                &mut scene.gpu,
                input.input(),
                host.ui_input().map(|ui| (&mut scene.ui, ui)),
            )
            .map_err(|e| -> Box<dyn std::error::Error> { e })?;
        let rgba_sha256 = incant_assets_runtime::sha256(rgba);
        let png = format!("frame-{host_step:04}.png");
        let raw = Options::raw_bytes(self.options.size)?;
        let (png_bytes, png_sha256) = self.output.publish(&png, raw + 1024 * 1024, |writer| {
            self.target
                .write_png(writer)
                .map_err(|e| -> Box<dyn std::error::Error> { e })
        })?;
        let ui_stats = host.ui_input().and_then(|_| scene.ui.stats());
        let receipt = Receipt {
            host_step,
            generation,
            world_epoch: generation,
            simulation_tick: world.tick(),
            topology_generation: input.topology_generation(),
            binding_generation: input.binding_generation(),
            text_generation: input.text_generation(),
            physical_size: self.options.size,
            logical_viewport: host.ui_input().map(|ui| ui.viewport),
            ui_widgets: ui_stats.map_or(0, |s| s.widgets),
            ui_glyph_quads: ui_stats.map_or(0, |s| s.glyph_quads),
            pcm_frames: pcm.frames(),
            pcm_prefix_sha256: pcm.sha256(),
            png,
            png_bytes,
            png_sha256,
            rgba_sha256,
        };
        self.output
            .json(&format!("frame-{host_step:04}.json"), &receipt)?;
        self.receipts.push(receipt);
        Ok(())
    }
    pub fn report(&self) -> serde_json::Value {
        let adapter = self.renderer.adapter.get_info();
        serde_json::json!({"mode":"selected-offscreen-frames","on_screen":false,"physical_size":self.options.size,
            "selected_host_steps":self.options.steps,"captured_frames":self.receipts.len(),
            "adapter":{"name":adapter.name,"backend":format!("{:?}",adapter.backend)},
            "continuous_gpu_submission":false,"temporal_history":false})
    }
    pub fn finish(mut self, report: &serde_json::Value) -> Result<()> {
        if self.failed {
            return Err("capture failed; complete manifest withheld".into());
        }
        self.options
            .complete(self.receipts.iter().map(|r| r.host_step))?;
        self.output.json(
            "manifest.json",
            &serde_json::json!({"format":"incant-player-frame-capture","version":1,
            "complete":true,"capture":self.report(),"frames":self.receipts,"player":report}),
        )?;
        Ok(())
    }
}

#[cfg(all(test, target_os = "macos"))]
#[path = "metal.rs"]
mod tests;
