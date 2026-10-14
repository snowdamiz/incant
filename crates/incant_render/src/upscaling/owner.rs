use super::{
    Plan, UpscaleConfig, UpscaleError,
    targets::{Output, Overlay},
};
use crate::motion::MotionFrame;
use std::sync::Arc;
#[derive(Clone, Debug)]
pub struct UpscaleFrame {
    motion: MotionFrame,
    output: [u32; 2],
    slot: usize,
}
impl UpscaleFrame {
    pub fn motion(&self) -> &MotionFrame {
        &self.motion
    }
    pub fn output_size(&self) -> [u32; 2] {
        self.output
    }
}
pub(crate) struct Input {
    pub frame: MotionFrame,
    pub rect: [f32; 4],
    pub jitter: [f32; 2],
    pub previous_jitter: [f32; 2],
    pub surfaces: Option<(wgpu::TextureView, wgpu::TextureView)>,
    pub owner: Option<(wgpu::TextureView, wgpu::Buffer)>,
}
pub(crate) struct State {
    config: UpscaleConfig,
    output: Option<Arc<Output>>,
    overlay: Option<Arc<Overlay>>,
    last: Option<UpscaleFrame>,
    /// Expected admitted-history evidence after the accepted frame, in frames of
    /// unit evidence; the accepted internal source width it was gathered at.
    mass: f32,
    source_width: f32,
    /// The accepted slot's RG16F evidence was written by the ordinary resolve on
    /// the accepted output allocation, and the source-rect area it was taken at.
    evidence: bool,
    source_area: f32,
    /// Output-viewport light grid of accepted output-resolution BLEND frames. Its
    /// only owner: released with this state, charged by every upscaling preflight.
    grid: Option<Arc<crate::lighting::GridBuffers>>,
}
pub(crate) struct Pending {
    pub(super) config: UpscaleConfig,
    pub(super) input: Input,
    pub(super) plan: Option<Plan>,
    pub(super) frame: UpscaleFrame,
    pub(super) output: Option<Arc<Output>>,
    pub(super) overlay: Option<Arc<Overlay>>,
    /// This frame's history evidence for the resolve uniform (reactive.y).
    pub(super) history_mass: f32,
    source_width: f32,
    /// Stored-evidence density (old / new source-rect area) and whether the
    /// previous slot's evidence may be read; this frame writes evidence only if a
    /// portable resolve (ordinary or owner) encodes it, never a native frame.
    pub(super) density: f32,
    pub(super) evidence_valid: bool,
    writes_evidence: bool,
    source_area: f32,
    retained_grid: Option<Arc<crate::lighting::GridBuffers>>,
    /// Candidate output grid, offered once while encoding; accepted with the frame.
    grid: std::sync::OnceLock<Arc<crate::lighting::GridBuffers>>,
}
/// Expected admitted-history evidence entering a frame, in frames of unit
/// evidence: 0 after a cut, the accepted evidence otherwise, counted at the old
/// density, (old / new width)^2, after an internal-scale increase. It never
/// exceeds h / (1 - h), so the resolve's history fraction
/// min(h_r, W_r / (W_r + 1)) equals today's h_r in steady state and is lower
/// only after a cut or a scale increase.
pub(super) fn frame_mass(h: f32, mass: f32, has_previous: bool, old_width: f32, width: f32) -> f32 {
    let cap = h / (1. - h);
    if !has_previous || !cap.is_finite() || cap <= 0. {
        return 0.;
    }
    let mut w = if mass.is_finite() {
        mass.clamp(0., cap)
    } else {
        0.
    };
    if old_width > 0. && width > old_width {
        w *= (old_width / width) * (old_width / width);
    }
    w
}
impl State {
    pub fn new(config: UpscaleConfig) -> Result<Self, crate::temporal::TemporalError> {
        config.validate()?;
        Ok(Self {
            config,
            output: None,
            overlay: None,
            last: None,
            mass: 0.,
            source_width: 0.,
            evidence: false,
            source_area: 0.,
            grid: None,
        })
    }
    fn grid_bytes(&self) -> u64 {
        self.grid.as_ref().map_or(0, |g| g.bytes())
    }
    pub fn frame(&self) -> Option<UpscaleFrame> {
        self.last.clone()
    }
    pub fn preflight(
        &self,
        device: &wgpu::Device,
        plan: Plan,
        old_motion: u64,
    ) -> Result<(), UpscaleError> {
        let total = plan.preflight(
            device.limits().max_texture_dimension_2d,
            self.output.as_ref().map_or(0, |o| o.bytes()),
            self.overlay.as_ref().map_or(0, |o| o.bytes()),
            old_motion,
        )?;
        plan.with_output_grid(total, self.grid_bytes())
            .map(|_| ())
            .ok_or(UpscaleError::Capacity)
    }
    pub fn preflight_owner(
        &self,
        device: &wgpu::Device,
        plan: Plan,
        old_motion: Option<crate::motion::attachment::Cache>,
    ) -> Result<(), crate::motion::MotionError> {
        let total = crate::motion::attachment::admit_records(
            plan.source,
            old_motion,
            plan.output,
            self.output
                .as_ref()
                .map(|o| crate::motion::attachment::Cache {
                    size: o.size,
                    owner: o.record.is_some(),
                }),
            self.overlay.as_ref().map(|o| o.size),
            device.limits().max_texture_dimension_2d,
        )?;
        plan.with_output_grid(total, self.grid_bytes())
            .map(|_| ())
            .ok_or(crate::motion::MotionError::Capacity)
    }
    pub fn prepare(
        &self,
        d: &wgpu::Device,
        input: Input,
        plan: Option<Plan>,
        size: [u32; 2],
    ) -> Result<Pending, UpscaleError> {
        if input.frame.bypassed() != plan.is_none()
            || input.frame.bypassed() != input.surfaces.is_none()
        {
            return Err(UpscaleError::History);
        }
        if let Some(p) = plan
            && (input.rect != p.source_rect || input.frame.internal_size() != p.source)
        {
            return Err(UpscaleError::History);
        }
        if input.frame.has_previous()
            && self.last.as_ref().is_none_or(|last| {
                !last.motion.same_owner(&input.frame)
                    || last.motion.epoch() != input.frame.epoch()
                    || last.motion.sequence().checked_add(1) != Some(input.frame.sequence())
                    || last.output != size
            })
        {
            return Err(UpscaleError::History);
        }
        let owner = input.owner.is_some();
        if input.frame.has_previous()
            && self
                .output
                .as_ref()
                .is_some_and(|o| o.record.is_some() != owner)
        {
            return Err(UpscaleError::History);
        }
        let slot = self.last.as_ref().map_or(0, |f| 1 - f.slot);
        let frame = UpscaleFrame {
            motion: input.frame.clone(),
            output: size,
            slot,
        };
        let source_width = plan.map_or(0., |p| p.source_rect[2]);
        let history_mass = frame_mass(
            self.config.history_weight,
            self.mass,
            input.frame.has_previous(),
            self.source_width,
            source_width,
        );
        let output = plan.map(|p| Output::prepare(d, p.output, self.output.as_ref(), owner));
        // Evidence is readable only from this program's own accepted slot on the
        // same allocation: never after a cut, a new allocation (resize or owner
        // toggle), an owner or native frame, or a failed (unaccepted) submission.
        let reused = output
            .as_ref()
            .zip(self.output.as_ref())
            .is_some_and(|(a, b)| Arc::ptr_eq(a, b));
        let source_area = plan.map_or(0., |p| p.source_rect[2] * p.source_rect[3]);
        let density = if self.source_area > 0. && source_area > 0. {
            self.source_area / source_area
        } else {
            1.
        };
        // Both programs write evidence; an owner toggle reallocates Output (record
        // presence differs), so `reused` already separates the two histories.
        let evidence_valid = self.evidence && reused && input.frame.has_previous();
        let writes_evidence = plan.is_some();
        Ok(Pending {
            config: self.config,
            input,
            plan,
            frame,
            output,
            // Output extent: current-only BLEND models render here at output resolution.
            overlay: plan.map(|p| Overlay::prepare(d, p.output, self.overlay.as_ref())),
            history_mass,
            source_width,
            density,
            evidence_valid,
            writes_evidence,
            source_area,
            retained_grid: self.grid.clone(),
            grid: std::sync::OnceLock::new(),
        })
    }
    pub fn accept(&mut self, p: Pending) {
        // Published only with the accepted submission, with colour/depth/records.
        self.evidence = p.writes_evidence;
        self.source_area = p.source_area;
        let h = self.config.history_weight;
        // Each accepted frame adds one unit of expected evidence (the kernel's
        // cycle-mean mass), never above the h / (1 - h) cap.
        self.mass = (p.history_mass + 1.).min(h / (1. - h));
        self.source_width = p.source_width;
        self.output = p.output;
        self.overlay = p.overlay;
        self.last = Some(p.frame);
        // A frame without BLEND draws keeps the retained grid (still charged).
        if let Some(grid) = p.grid.into_inner() {
            self.grid = Some(grid);
        }
    }
}
impl Pending {
    pub fn overlay(&self) -> Option<&wgpu::TextureView> {
        self.overlay.as_ref().map(|o| &o.view)
    }
    pub fn composite(&self) -> Option<&wgpu::TextureView> {
        self.output.as_ref().map(|o| &o.composite)
    }
    pub fn retained_grid(&self) -> Option<&Arc<crate::lighting::GridBuffers>> {
        self.retained_grid.as_ref()
    }
    /// Records this frame's output grid; only the first offer is kept.
    pub fn offer_grid(&self, grid: Arc<crate::lighting::GridBuffers>) {
        let _ = self.grid.set(grid);
    }
    /// Output-resolution target of current-only sprites and world text: the
    /// composited output, its unjittered opaque depth and the output viewport.
    pub fn screen_overlay(&self) -> Option<(&wgpu::TextureView, &wgpu::TextureView, [f32; 4])> {
        let output = self.output.as_ref()?;
        Some((&output.composite, &output.overlay_depth, self.plan?.rect))
    }
    #[cfg(feature = "native-metalfx")]
    pub(crate) fn native_input(
        &self,
    ) -> Option<(&Input, Plan, &wgpu::TextureView, &wgpu::TextureView)> {
        let o = self.output.as_ref()?;
        Some((
            &self.input,
            self.plan?,
            &o.color[self.slot()],
            &o.depth[self.slot()],
        ))
    }
    /// Texture LOD bias for internal opaque/MASK draws: log2(source / output),
    /// negative at reduced scale, so the internal texture footprint matches the
    /// output grid that history accumulates, floored at `jitter::MIN_LOD_BIAS`.
    /// 0 without a portable plan (default and fixed-TAA paths) and never positive.
    pub(crate) fn lod_bias(&self) -> f32 {
        let bias = self.plan.map_or(0., |p| {
            let b = (p.source_rect[2] / p.rect[2]).log2();
            if b.is_finite() {
                b.clamp(crate::temporal::jitter::MIN_LOD_BIAS, 0.)
            } else {
                0.
            }
        });
        #[cfg(test)]
        let bias = crate::temporal::jitter::diagnostic_lod_bias(bias);
        bias
    }
    /// The physical output rect of the portable plan (the DRS-stable guard's base).
    pub(crate) fn output_rect(&self) -> Option<[f32; 4]> {
        self.plan.map(|p| p.rect)
    }
    #[cfg(feature = "native-metalfx")]
    pub(crate) fn native_config(&self) -> UpscaleConfig {
        self.config
    }
    #[cfg(feature = "native-metalfx")]
    pub(crate) fn discard_color_history(&mut self) {
        self.config.history_weight = 0.;
    }
    /// Native MetalFX wrote this frame's colour and depth; its evidence slot is
    /// unwritten, so the next portable frame must not read it.
    #[cfg(feature = "native-metalfx")]
    pub(crate) fn mark_native(&mut self) {
        self.writes_evidence = false;
    }
    pub(super) fn slot(&self) -> usize {
        self.frame.slot
    }
}
#[cfg(test)]
impl State {
    pub(crate) fn diagnostic_surfaces(&self) -> Option<[wgpu::TextureView; 3]> {
        let output = self.output.as_ref()?;
        let slot = self.last.as_ref()?.slot;
        Some([
            output.color[slot].clone(),
            output.depth[slot].clone(),
            output.evidence[slot].clone(),
        ])
    }
    pub(crate) fn accepted_owner_surfaces(&self) -> Option<[wgpu::TextureView; 3]> {
        let output = self.output.as_ref()?;
        let slot = self.last.as_ref()?.slot;
        Some([
            output.color[slot].clone(),
            output.depth[slot].clone(),
            output.record.as_ref()?[slot].clone(),
        ])
    }
    pub(crate) fn accepted_color(&self) -> Option<wgpu::TextureView> {
        self.output
            .as_ref()
            .zip(self.last.as_ref())
            .map(|(o, l)| o.color[l.slot].clone())
    }
}
