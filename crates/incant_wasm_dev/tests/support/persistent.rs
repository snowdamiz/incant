use incant_game_runtime::{GameError, HostedOfflineGameSession, SceneHost};
use incant_runtime::*;
use incant_types::{TextWrap, UiKind, UiLength, UiNode, UiStyle, UiText};
use incant_ui::{MeasureSpace, TextMeasure, UiError, UiEvent, UiRuntime, Widget};
use std::{path::Path, sync::Arc};

pub fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}
pub fn payload() -> CheckpointPayloadBudget {
    CheckpointPayloadBudget::new(64 * 1024 * 1024).unwrap()
}
pub fn planar() -> PlanarCheckpointWorkBudget {
    PlanarCheckpointWorkBudget::new(100_000_000)
}
pub fn spatial() -> SpatialCheckpointWorkBudget {
    SpatialCheckpointWorkBudget::new(100_000_000)
}
pub fn allocations() -> CheckpointAllocationBudget {
    CheckpointAllocationBudget::new(64 * 1024 * 1024, 100_000)
}
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
fn error(error: impl std::error::Error + Send + Sync + 'static) -> std::io::Error {
    std::io::Error::other(error)
}
pub struct Host {
    pub ui: UiRuntime,
    reject: bool,
}
impl SceneHost for Host {
    const SUPPORTS_UI: bool = true;
    type Settings = bool;
    type Error = std::io::Error;
    fn ui(&self) -> Option<&UiRuntime> {
        Some(&self.ui)
    }
    fn prepare(world: &NativeWorld, reject: bool) -> Result<Self, Self::Error> {
        let mut ui = UiRuntime::for_world(world).map_err(error)?;
        for events in [vec![], vec![UiEvent::Focus(id(2))]] {
            let next = ui
                .prepare(&events, [400.; 2], &mut Measure)
                .map_err(error)?;
            ui.commit_for_world(next, world).map_err(error)?;
        }
        let next = ui
            .prepare(
                &[UiEvent::Commit {
                    target: ui.focus().unwrap(),
                    text: "日本語".into(),
                }],
                [400.; 2],
                &mut Measure,
            )
            .map_err(error)?;
        ui.commit_for_world(next, world).map_err(error)?;
        Ok(Self { ui, reject })
    }
    fn prepare_checkpoint(
        world: &NativeWorld,
        reject: bool,
        state: Option<&incant_ui::PendingUiCheckpoint>,
    ) -> incant_game_runtime::Result<Self, Self::Error> {
        let ui = state
            .ok_or(GameError::UiHost)?
            .restore(world, [400.; 2], &mut Measure)
            .map_err(|e| GameError::Host(error(e)))?;
        if reject {
            return Err(GameError::Host(std::io::Error::other(
                "candidate host refusal after UI reconstruction",
            )));
        }
        Ok(Self { ui, reject })
    }
    fn sync(&mut self, world: &NativeWorld) -> Result<(), Self::Error> {
        if self.reject {
            return Err(std::io::Error::other("late composed consumer refusal"));
        }
        let next = self
            .ui
            .prepare_refresh([400.; 2], &mut Measure)
            .map_err(error)?;
        self.ui.commit_for_world(next, world).map_err(error)
    }
}
pub fn audible(root: &Path) -> (CookedScene, Vec<incant_audio_runtime::ClipBinding>) {
    let key = super::audio_support::cooked(root, &vec![[0.25, -0.125]; 4096]);
    let clip = incant_audio_runtime::ClipBinding::load(id(90), root, &key).unwrap();
    let scene = CookedScene::new(
        id(99),
        64,
        (1..=3)
            .map(|n| CookedEntity {
                id: id(n),
                parent: (n == 2).then(|| id(1)),
                velocity: None,
                transform: Default::default(),
            })
            .collect(),
    )
    .unwrap()
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
            entity: id(1),
            node: Arc::new(UiNode {
                kind: UiKind::Root,
                order: 0,
                disabled: false,
                style: Default::default(),
            }),
        },
        CookedUi {
            entity: id(2),
            node: Arc::new(UiNode {
                kind: UiKind::TextInput {
                    text: UiText {
                        content: "seed".into(),
                        fonts: vec![id(101)],
                        font_size: 16.,
                        line_height: 20.,
                        wrap: TextWrap::Word,
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
    .with_audio(vec![CookedAudio {
        entity: id(3),
        bus: None,
        listener: false,
        source: Some(NativeAudioSource {
            transport_epoch: 0,
            clip: id(90),
            bus: None,
            gain_db: 0.,
            pan: 0.,
            rate: 1.,
            start_seconds: 0.,
            playing: true,
            looping: true,
            streaming: false,
            spatial: None,
        }),
    }])
    .unwrap();
    (scene, vec![clip])
}
pub fn tick(game: &mut HostedOfflineGameSession<Host>) -> Vec<f32> {
    let mut pcm = vec![9.; game.next_output_frames().unwrap() * 2];
    game.tick(&[], &mut pcm).unwrap();
    pcm
}
