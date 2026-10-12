use incant_runtime::*;
use incant_types::{Transform, Velocity};
pub fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}
pub fn source(streaming: bool, playing: bool, looping: bool) -> CookedAudio {
    CookedAudio {
        entity: id(2),
        bus: None,
        listener: false,
        source: Some(NativeAudioSource {
            transport_epoch: 0,
            clip: id(90),
            bus: Some(id(1)),
            gain_db: 0.,
            pan: 0.,
            rate: 1.,
            start_seconds: 0.,
            playing,
            looping,
            streaming,
            spatial: None,
        }),
    }
}
pub fn scene(streaming: bool, playing: bool, looping: bool) -> CookedScene {
    CookedScene::new(
        id(99),
        60,
        (1..=3)
            .map(|n| CookedEntity {
                id: id(n),
                parent: None,
                transform: Transform::default(),
                velocity: Some(Velocity { linear: [0.; 3] }),
            })
            .collect(),
    )
    .unwrap()
    .with_audio(vec![
        CookedAudio {
            entity: id(1),
            bus: Some(NativeAudioBus {
                parent: None,
                gain_db: 0.,
            }),
            source: None,
            listener: false,
        },
        source(streaming, playing, looping),
    ])
    .unwrap()
}
pub fn audio(world: &NativeWorld) -> Vec<CookedAudio> {
    world.audio_nodes().map(|(node, _)| node.clone()).collect()
}
