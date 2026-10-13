use super::*;
use incant_game_runtime::{GameError, HostedOfflineGameSession, SceneHost};
use incant_runtime::{CookedUi, FontAssetRef, NativeWorld};
use incant_ui::{
    MeasureSpace, TextMeasure, UiError, UiEvent, UiKind, UiLength, UiNode, UiRuntime, UiStyle,
    UiText, Widget,
};
use std::sync::Arc;

struct Measure;
impl TextMeasure for Measure {
    fn measure(
        &mut self,
        _: &Widget,
        text: &str,
        known: [Option<f32>; 2],
        _: [MeasureSpace; 2],
    ) -> Result<[f32; 2], UiError> {
        Ok([
            known[0].unwrap_or(text.chars().count() as f32 * 8.),
            known[1].unwrap_or(20.),
        ])
    }
}
struct Host(UiRuntime);
fn host_error(error: impl std::error::Error + Send + Sync + 'static) -> std::io::Error {
    std::io::Error::other(error)
}
impl SceneHost for Host {
    const SUPPORTS_UI: bool = true;
    type Settings = bool;
    type Error = std::io::Error;
    fn ui(&self) -> Option<&UiRuntime> {
        Some(&self.0)
    }
    fn prepare(world: &NativeWorld, _: bool) -> Result<Self, Self::Error> {
        let mut ui = UiRuntime::for_world(world).map_err(host_error)?;
        let next = ui
            .prepare(&[], [400.; 2], &mut Measure)
            .map_err(host_error)?;
        ui.commit_for_world(next, world).map_err(host_error)?;
        let next = ui
            .prepare(&[UiEvent::Focus(id(9))], [400.; 2], &mut Measure)
            .map_err(host_error)?;
        ui.commit_for_world(next, world).map_err(host_error)?;
        let next = ui
            .prepare(
                &[UiEvent::Commit {
                    target: ui.focus().unwrap(),
                    text: "日本語".into(),
                }],
                [400.; 2],
                &mut Measure,
            )
            .map_err(host_error)?;
        ui.commit_for_world(next, world).map_err(host_error)?;
        Ok(Self(ui))
    }
    fn prepare_checkpoint(
        world: &NativeWorld,
        reset_incorrectly: bool,
        pending: Option<&incant_ui::PendingUiCheckpoint>,
    ) -> incant_game_runtime::Result<Self, Self::Error> {
        if reset_incorrectly {
            let mut ui = UiRuntime::for_world(world).map_err(|e| GameError::Host(host_error(e)))?;
            let next = ui
                .prepare(&[], [400.; 2], &mut Measure)
                .map_err(|e| GameError::Host(host_error(e)))?;
            ui.commit_for_world(next, world)
                .map_err(|e| GameError::Host(host_error(e)))?;
            return Ok(Self(ui));
        }
        let ui = pending
            .ok_or(GameError::UiHost)?
            .restore(world, [600., 300.], &mut Measure)
            .map_err(|error| GameError::Host(host_error(error)))?;
        Ok(Self(ui))
    }
    fn sync(&mut self, world: &NativeWorld) -> Result<(), Self::Error> {
        let next = self
            .0
            .prepare_refresh([400.; 2], &mut Measure)
            .map_err(host_error)?;
        self.0.commit_for_world(next, world).map_err(host_error)
    }
}
fn initial() -> CookedScene {
    scene(true)
        .with_text(
            vec![FontAssetRef {
                id: id(101),
                fingerprint: [7; 32],
            }],
            vec![],
        )
        .unwrap()
        .with_ui(vec![
            CookedUi {
                entity: id(8),
                node: Arc::new(UiNode {
                    kind: UiKind::Root,
                    order: 0,
                    disabled: false,
                    style: Default::default(),
                }),
            },
            CookedUi {
                entity: id(9),
                node: Arc::new(UiNode {
                    kind: UiKind::TextInput {
                        text: UiText {
                            content: "seed".into(),
                            fonts: vec![id(101)],
                            font_size: 16.,
                            line_height: 20.,
                            wrap: incant_types::TextWrap::Word,
                        },
                    },
                    order: 0,
                    disabled: false,
                    style: UiStyle {
                        size: [UiLength::Pixels(180.), UiLength::Pixels(30.)],
                        ..Default::default()
                    },
                }),
            },
        ])
        .unwrap()
}
fn hosted_tick(game: &mut HostedOfflineGameSession<Host>) -> Vec<f32> {
    let mut pcm = vec![0.; game.next_output_frames().unwrap() * 2];
    game.tick(&[], &mut pcm).unwrap();
    pcm
}

#[test]
fn ui_audio_and_both_physics_owners_resume_as_one_fresh_game() {
    let temp = tempfile::tempdir().unwrap();
    let bindings = clips(temp.path());
    let mut live = HostedOfflineGameSession::<Host>::new(
        initial(),
        code(),
        Default::default(),
        bindings.clone(),
        44100,
        false,
    )
    .unwrap();
    for _ in 0..13 {
        hosted_tick(&mut live);
    }
    let original_token = live.host().unwrap().0.focus().unwrap();
    let saved_ui = live.host().unwrap().0.checkpoint_bytes().unwrap();
    let checkpoint = live
        .checkpoint(&mut payload(), &mut planar(), &mut spatial())
        .unwrap();
    assert!(
        HostedOfflineGameSession::<Host>::from_checkpoint(
            prepare(&checkpoint),
            bindings.clone(),
            true
        )
        .is_err()
    );
    // An incapable host must refuse the retained UI section, rather than reset it.
    assert!(
        HostedOfflineGameSession::<super::native_checkpoint_cases::Host>::from_checkpoint(
            prepare(&checkpoint),
            bindings.clone(),
            (
                Arc::new(std::sync::atomic::AtomicUsize::new(0)),
                super::native_checkpoint_cases::Failure::None
            ),
        )
        .is_err()
    );
    let mut restored =
        HostedOfflineGameSession::<Host>::from_checkpoint(prepare(&checkpoint), bindings, false)
            .unwrap();
    assert_ne!(original_token, restored.host().unwrap().0.focus().unwrap());
    assert_eq!(
        saved_ui,
        restored.host().unwrap().0.checkpoint_bytes().unwrap()
    );
    assert_eq!(
        restored.host().unwrap().0.layout().unwrap().viewport,
        [600., 300.]
    );
    assert!(
        restored
            .host()
            .unwrap()
            .0
            .text(id(9))
            .unwrap()
            .text()
            .contains("日本語")
    );
    for _ in 0..37 {
        assert_eq!(hosted_tick(&mut live), hosted_tick(&mut restored));
        assert_eq!(
            live.world().unwrap().snapshot(),
            restored.world().unwrap().snapshot()
        );
        assert_eq!(
            live.host()
                .unwrap()
                .0
                .checkpoint_for_world(live.world().unwrap())
                .unwrap()
                .bytes(),
            restored
                .host()
                .unwrap()
                .0
                .checkpoint_for_world(restored.world().unwrap())
                .unwrap()
                .bytes()
        );
    }
    assert!(live.checkpoint_bytes().is_err());
}
