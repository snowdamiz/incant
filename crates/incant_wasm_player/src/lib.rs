//! Bounded trusted fixture data for the composed static-module player proof.
//! Retained PCM metadata and input recording JSON are startup exchange formats,
//! not project documents or binary-cooked input assets. No JSON is used on ticks.
use incant_assets_runtime::{AUDIO_CHUNK_FRAMES, AudioMetadata, Dependency, audio, sha256};
use incant_audio_runtime::ClipBinding;
use incant_input::{InputActions, InputReplay};
use incant_runtime::{
    CookedAudio, CookedEntity, CookedPhysics, CookedScene, NativeAudioSource, StableId,
};
use incant_types::{AudioSpatial, BodyMotion, Collider, RigidBody, Transform, Velocity};
use std::{error::Error, fs, path::Path};

pub const SAMPLE_RATE: u32 = 44_117;
pub const TICKS: u64 = 64;
pub fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}
pub fn scene(streaming: bool) -> CookedScene {
    CookedScene::new(
        id(99),
        64,
        vec![
            CookedEntity {
                id: id(1),
                parent: None,
                transform: Transform {
                    translation: [-2., 0., 0.],
                    ..Default::default()
                },
                velocity: Some(Velocity {
                    linear: [1., 0., 0.],
                }),
            },
            CookedEntity {
                id: id(2),
                parent: None,
                transform: Transform::default(),
                velocity: None,
            },
        ],
    )
    .unwrap()
    .with_physics(vec![CookedPhysics {
        entity: id(1),
        collider: Collider::default(),
        body: Some(RigidBody {
            motion: BodyMotion::Kinematic,
            ..Default::default()
        }),
        angular_velocity: None,
    }])
    .unwrap()
    .with_audio(vec![
        CookedAudio {
            entity: id(1),
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
                streaming,
                spatial: Some(AudioSpatial {
                    min_distance: 1.,
                    max_distance: 100.,
                }),
            }),
        },
        CookedAudio {
            entity: id(2),
            bus: None,
            listener: true,
            source: None,
        },
    ])
    .unwrap()
}
pub fn actions() -> InputActions {
    serde_json::from_str(r#"{"jump":{"kind":"button","bindings":[{"type":"key","code":"Space"}]}}"#)
        .unwrap()
}
pub fn recording() -> String {
    // A key-down and key-up have an observable accepted input timeline. The
    // current Wasm ABI cannot read that timeline; the native host can observe it.
    r#"{"format":"incant-input","version":1,"tick_rate":64,"start_tick":0,"ticks":64,"frames":[{"tick":1,"events":[{"type":"key","code":"Space","down":true}]},{"tick":3,"events":[{"type":"key","code":"Space","down":false}]}]}"#.into()
}
pub fn replay(text: &str, world: &CookedScene) -> Result<InputReplay, Box<dyn Error>> {
    let replay = InputReplay::from_text(text, 0, world.tick_rate())?;
    if replay.end_tick() != TICKS {
        return Err("fixture recording must cover exactly 64 ticks from zero".into());
    }
    // Nonzero recording starts are already rejected by from_text(text,0,...).
    Ok(replay)
}
pub fn write_fixture(path: &Path, streaming: bool) -> Result<String, Box<dyn Error>> {
    fs::create_dir(path)?;
    fs::write(path.join("fixture.scene"), scene(streaming).to_bytes())?;
    fs::write(path.join("input.json"), recording())?;
    for (name, scene) in ["camera", "manifest"].into_iter().zip(spatial_scenes()) {
        fs::write(path.join(format!("{name}.scene")), scene.to_bytes())?;
    }
    fs::write(path.join("overflow.scene"), overflow_scene().to_bytes())?;
    let dependency = Dependency {
        path: "absent/fixture.wav".into(),
        sha256: sha256(b"trusted static-game source"),
    };
    let fingerprint = audio::key(&dependency)?;
    let pcm: Vec<u8> = [0.2f32; 2]
        .into_iter()
        .flat_map(f32::to_le_bytes)
        .cycle()
        .take(48_000 * 8)
        .collect();
    let metadata = AudioMetadata {
        version: 1,
        fingerprint: fingerprint.clone(),
        dependency,
        sample_rate: 48_000,
        channels: 2,
        frames: 48_000,
        content_sha256: sha256(&pcm),
        chunk_sha256: pcm.chunks(AUDIO_CHUNK_FRAMES * 8).map(sha256).collect(),
    };
    fs::write(path.join(format!("{}.ipcm", metadata.content_sha256)), pcm)?;
    fs::write(
        path.join(format!("{fingerprint}.json")),
        serde_json::to_vec(&metadata)?,
    )?;
    Ok(fingerprint)
}
pub fn load_clip(cache: &Path, key: &str) -> Result<ClipBinding, Box<dyn Error>> {
    Ok(ClipBinding::load(id(90), cache, key)?)
}

/// Real v4 refusal fixtures for players without a visual host.
pub fn spatial_scenes() -> [CookedScene; 2] {
    let entity = scene(false)
        .entities()
        .iter()
        .find(|e| e.id == id(1))
        .unwrap()
        .clone();
    let plain = CookedScene::new(id(99), 64, vec![entity]).unwrap();
    let mut camera = incant_runtime::CookedSpatial::empty(id(1));
    camera.camera = Some(incant_types::Camera {
        fov_degrees: 60.,
        near: 0.1,
        far: 100.,
        projection: incant_types::CameraProjection::Perspective {},
    });
    [
        plain.clone().with_spatial(vec![], vec![camera]).unwrap(),
        plain
            .with_spatial(
                vec![incant_runtime::VisualAssetRef {
                    id: id(9),
                    kind: incant_runtime::VisualAssetKind::Model,
                    fingerprint: [7; 32],
                }],
                vec![],
            )
            .unwrap(),
    ]
}
pub fn overflow_scene() -> CookedScene {
    CookedScene::new(
        id(99),
        64,
        vec![CookedEntity {
            id: id(1),
            parent: None,
            transform: Transform {
                translation: [f64::MAX, 0., 0.],
                ..Default::default()
            },
            velocity: Some(Velocity {
                linear: [f64::MAX, 0., 0.],
            }),
        }],
    )
    .unwrap()
}
