//! Default-off Core pose diagnostic. No alternate clock or simulation policy.
use super::{Game, SurfaceTargets, session};
use incant_game_runtime::{SceneHost, SceneHostPresentation};
use incant_render_data::UiInput;
use incant_render_game::{
    NativeRenderHost, NativeRenderSettings, RenderHostError,
    pose::{PoseHostError, PoseRenderHost, PoseReply, PoseRequest},
};
use incant_render_gpu_host::{GpuHostError, GpuRenderConsumer, GpuRenderSettings, PoseTargets};
use incant_runtime::NativeWorld;
use std::sync::Arc;

type Ordinary = NativeRenderHost<GpuRenderConsumer<SurfaceTargets>>;
type Interpolated = PoseRenderHost<GpuRenderConsumer<PoseTargets<SurfaceTargets>>>;
type ConsumerError = GpuHostError<super::Error>;
pub struct Settings {
    pub native: NativeRenderSettings<GpuRenderSettings<SurfaceTargets>>,
    pub enabled: bool,
}
pub enum Host {
    Ordinary(Ordinary),
    Interpolated(Interpolated),
}
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Ordinary(#[from] RenderHostError<ConsumerError>),
    #[error(transparent)]
    Pose(#[from] PoseHostError<ConsumerError>),
    #[error(transparent)]
    OrdinaryCheckpoint(incant_game_runtime::GameError<RenderHostError<ConsumerError>>),
    #[error(transparent)]
    PoseCheckpoint(incant_game_runtime::GameError<PoseHostError<ConsumerError>>),
    #[error("pose presentation was not selected")]
    Unsupported,
}
impl Host {
    pub fn ui(&self) -> Option<&incant_ui::UiRuntime> {
        match self {
            Self::Ordinary(h) => h.ui(),
            Self::Interpolated(h) => h.ui(),
        }
    }
    pub fn ui_input(&self) -> Option<&Arc<UiInput>> {
        match self {
            Self::Ordinary(h) => h.ui_input(),
            Self::Interpolated(h) => h.ui_input(),
        }
    }
    pub fn accepted_frames(&self) -> u64 {
        match self {
            Self::Ordinary(h) => h.consumer().accepted_frames(),
            Self::Interpolated(h) => h.consumer().accepted_frames(),
        }
    }
    pub fn ui_stats(&self) -> Option<incant_render::UiStats> {
        match self {
            Self::Ordinary(h) => h.consumer().ui_stats(),
            Self::Interpolated(h) => h.consumer().ui_stats(),
        }
    }
    fn epoch(&self) -> Result<u64, Error> {
        match self {
            Self::Ordinary(_) => Err(Error::Unsupported),
            Self::Interpolated(h) => Ok(h.continuity_epoch()),
        }
    }
}
impl SceneHost for Host {
    const SUPPORTS_UI: bool = true;
    type Settings = Settings;
    type Error = Error;
    fn prepare(world: &NativeWorld, settings: Settings) -> Result<Self, Error> {
        if !settings.enabled {
            return Ok(Self::Ordinary(Ordinary::prepare(world, settings.native)?));
        }
        Ok(Self::Interpolated(Interpolated::prepare(
            world,
            interpolated_settings(settings.native),
        )?))
    }
    fn prepare_checkpoint(
        world: &NativeWorld,
        settings: Settings,
        checkpoint: Option<&incant_ui::PendingUiCheckpoint>,
    ) -> incant_game_runtime::Result<Self, Error> {
        let result = if settings.enabled {
            Interpolated::prepare_checkpoint(
                world,
                interpolated_settings(settings.native),
                checkpoint,
            )
            .map(Self::Interpolated)
            .map_err(Error::PoseCheckpoint)
        } else {
            Ordinary::prepare_checkpoint(world, settings.native, checkpoint)
                .map(Self::Ordinary)
                .map_err(Error::OrdinaryCheckpoint)
        };
        result.map_err(incant_game_runtime::GameError::Host)
    }
    fn ui(&self) -> Option<&incant_ui::UiRuntime> {
        self.ui()
    }
    fn stage_ui(
        &mut self,
        world: &NativeWorld,
        events: &[incant_input::InputEvent],
        receipts: &[incant_ui::UiEvent],
    ) -> Result<Option<incant_game_runtime::RoutedUiInput>, Error> {
        Ok(match self {
            Self::Ordinary(h) => h.stage_ui(world, events, receipts)?,
            Self::Interpolated(h) => h.stage_ui(world, events, receipts)?,
        })
    }
    fn cancel_ui(&mut self) {
        match self {
            Self::Ordinary(h) => h.cancel_ui(),
            Self::Interpolated(h) => h.cancel_ui(),
        }
    }
    fn resize_ui(&mut self, world: &NativeWorld, viewport: [f32; 2]) -> Result<(), Error> {
        match self {
            Self::Ordinary(h) => h.resize_ui(world, viewport)?,
            Self::Interpolated(h) => h.resize_ui(world, viewport)?,
        }
        Ok(())
    }
    fn sync(&mut self, world: &NativeWorld) -> Result<(), Error> {
        match self {
            Self::Ordinary(h) => h.sync(world)?,
            Self::Interpolated(h) => h.sync(world)?,
        }
        Ok(())
    }
}
fn interpolated_settings(
    s: NativeRenderSettings<GpuRenderSettings<SurfaceTargets>>,
) -> NativeRenderSettings<GpuRenderSettings<PoseTargets<SurfaceTargets>>> {
    NativeRenderSettings {
        model_cache: s.model_cache,
        texture_cache: s.texture_cache,
        camera: s.camera,
        text: s.text,
        ui: s.ui,
        consumer: GpuRenderSettings {
            renderer: s.consumer.renderer,
            targets: PoseTargets::new(s.consumer.targets),
        },
    }
}
impl SceneHostPresentation for Host {
    type Presentation = PoseRequest;
    type Presented = PoseReply;
    fn present(&mut self, world: &NativeWorld, request: PoseRequest) -> Result<PoseReply, Error> {
        match self {
            Self::Ordinary(_) => Err(Error::Unsupported),
            Self::Interpolated(h) => Ok(h.present(world, request)?),
        }
    }
}

/// Only bounded counters and the last accepted receipt are retained. Tick
/// history remains in the host, tied to its world identity. No callback owns it.
pub struct Driver {
    enabled: bool,
    reset_pending: bool,
    displays: u64,
    resets: u64,
    last: Option<incant_render_native::interpolation::PoseReceipt>,
}
impl Driver {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            reset_pending: enabled,
            displays: 0,
            resets: 0,
            last: None,
        }
    }
    pub fn enabled(&self) -> bool {
        self.enabled
    }
    pub fn interrupt(&mut self) {
        self.reset_pending |= self.enabled;
    }
    pub fn display(&mut self, game: &mut Game, fraction: f64) -> session::Result<()> {
        if !self.enabled {
            return Ok(());
        }
        let epoch = game.host()?.epoch()?;
        self.present(epoch, fraction, |request| Ok(game.present_host(request)?))
    }
    fn present(
        &mut self,
        epoch: u64,
        fraction: f64,
        mut submit: impl FnMut(PoseRequest) -> session::Result<PoseReply>,
    ) -> session::Result<()> {
        if !self.enabled {
            return Ok(());
        }
        let fraction = if self.reset_pending {
            let epoch = epoch
                .checked_add(1)
                .ok_or("pose continuity epoch exhausted")?;
            let reply = submit(PoseRequest::Reset {
                continuity_epoch: epoch,
            })?;
            if reply
                != (PoseReply::Reset {
                    continuity_epoch: epoch,
                })
            {
                return Err("pose reset returned a different receipt".into());
            }
            self.resets = self
                .resets
                .checked_add(1)
                .ok_or("pose reset counter exhausted")?;
            self.reset_pending = false;
            1.
        } else {
            fraction
        };
        let PoseReply::Displayed(receipt) = submit(PoseRequest::Display { fraction })? else {
            return Err("pose display returned a reset receipt".into());
        };
        self.displays = self
            .displays
            .checked_add(1)
            .ok_or("pose display counter exhausted")?;
        self.last = Some(receipt);
        Ok(())
    }
    pub fn refresh(&mut self, game: &mut Game) -> session::Result<()> {
        game.refresh_host()?;
        self.display(game, 1.)
    }
    pub fn report(&self) -> serde_json::Value {
        serde_json::json!({"enabled":self.enabled,"diagnostic_only":true,"performance_gate_passed":false,
            "policy":"ordinary-reservations-fixed-ticks","pose_displays":self.displays,"resets":self.resets,
            "world_text_graph_IK_root_motion_supported":false,
            "general_teleport_and_same_clip_seek_reset_channel":false,
            "last":self.last.map(|r| serde_json::json!({"previous_tick":r.previous_tick,"current_tick":r.current_tick,
                "continuity_epoch":r.continuity_epoch,"fraction":r.fraction}))})
    }
}
/// Intermediate catch-up draws and terminal transitions flush exact current
/// state. Only the last ordinary draw uses the accepted clock residual; the
/// shared due-batch deadline is never treated as each ordinal's individual time.
pub fn fraction(residual: f64, ordinal: usize, due: usize, terminal: bool) -> f64 {
    if terminal || ordinal + 1 < due {
        1.
    } else {
        residual
    }
}
pub fn validate_selection(
    enabled: bool,
    pacing: bool,
    wake: bool,
    batch: bool,
) -> session::Result<()> {
    if enabled && (pacing || wake || batch) {
        return Err(
            "pose interpolation requires ordinary pacing, wake and per-tick presentation".into(),
        );
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catchup_and_terminal_draws_do_not_present_backwards_in_simulation_time() {
        let values: Vec<_> = (0..4)
            .map(|n| 10. + n as f64 + fraction(0.25, n, 4, false))
            .collect();
        assert_eq!(values, [11., 12., 13., 13.25]);
        assert_eq!(fraction(0.25, 0, 1, true), 1.);
        assert_eq!(fraction(0., 0, 1, false), 0.);
    }
    #[test]
    fn inactive_driver_never_arms_history_or_records_displays() {
        let mut driver = Driver::new(false);
        driver.interrupt();
        assert!(!driver.reset_pending);
        assert_eq!(driver.displays, 0);
        assert_eq!(driver.resets, 0);
        driver
            .present(0, 0., |_| panic!("unselected pose path"))
            .unwrap();
    }
    #[test]
    fn lifecycle_boundaries_snap_once_before_display_and_refusal_is_not_recorded_as_success() {
        use incant_render_native::interpolation::PoseReceipt;
        let mut driver = Driver::new(true);
        let mut calls = Vec::new();
        let mut epoch = 0;
        for boundary in [true, false, true, false] {
            if boundary {
                driver.interrupt();
                driver.interrupt(); // coalesce focus/resume/resize before a single draw
            }
            driver
                .present(epoch, 0.25, |request| match request {
                    PoseRequest::Reset { continuity_epoch } => {
                        assert_eq!(continuity_epoch, epoch + 1);
                        epoch = continuity_epoch;
                        calls.push(("reset", 1.));
                        Ok(PoseReply::Reset { continuity_epoch })
                    }
                    PoseRequest::Display { fraction } => {
                        calls.push(("display", fraction));
                        Ok(PoseReply::Displayed(PoseReceipt {
                            previous_tick: 8,
                            current_tick: 9,
                            continuity_epoch: epoch,
                            fraction,
                        }))
                    }
                })
                .unwrap();
        }
        assert_eq!(
            calls,
            [
                ("reset", 1.),
                ("display", 1.),
                ("display", 0.25),
                ("reset", 1.),
                ("display", 1.),
                ("display", 0.25)
            ]
        );
        assert_eq!((driver.resets, driver.displays), (2, 4));
        let last = driver.last;
        driver.interrupt();
        assert!(
            driver
                .present(epoch, 0.5, |_| Err("reset refused".into()))
                .is_err()
        );
        assert!(driver.reset_pending);
        assert_eq!((driver.resets, driver.displays, driver.last), (2, 4, last));
        assert!(
            driver
                .present(u64::MAX, 0., |_| panic!("exhausted epoch must not submit"))
                .is_err()
        );
        assert_eq!((driver.resets, driver.displays, driver.last), (2, 4, last));
    }
    #[test]
    fn selection_refuses_other_experiments_before_any_native_startup() {
        for pacing in [false, true] {
            for wake in [false, true] {
                for batch in [false, true] {
                    assert!(validate_selection(false, pacing, wake, batch).is_ok());
                    assert_eq!(
                        validate_selection(true, pacing, wake, batch).is_ok(),
                        !(pacing || wake || batch)
                    );
                }
            }
        }
    }
}
