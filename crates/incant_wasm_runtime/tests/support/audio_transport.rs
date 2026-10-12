//! Independent typed native sequence shared by direct/actual-host/package oracles.
use incant_runtime::*;
use incant_types::{AudioSpatial, Transform, Velocity};
pub fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}
pub fn scene() -> CookedScene {
    let mut entities = Vec::new();
    for n in (1..=6).chain(100..162) {
        entities.push(CookedEntity {
            id: id(n),
            parent: None,
            transform: Transform::default(),
            velocity: Some(Velocity { linear: [0.; 3] }),
        });
    }
    let bus = |n, parent, gain_db| CookedAudio {
        entity: id(n),
        bus: Some(NativeAudioBus { parent, gain_db }),
        source: None,
        listener: false,
    };
    let source = |n, streaming| CookedAudio {
        entity: id(n),
        bus: None,
        listener: false,
        source: Some(NativeAudioSource {
            transport_epoch: if streaming { 17 } else { (1_u64 << 53) + 1 },
            clip: id(if streaming { 91 } else { 90 }),
            bus: Some(id(1)),
            gain_db: -4.,
            pan: if streaming { 0. } else { 0.25 },
            rate: if streaming { 0.87 } else { 1.13 },
            start_seconds: 0.00037,
            playing: true,
            looping: streaming,
            streaming,
            spatial: streaming.then_some(AudioSpatial {
                min_distance: 1.,
                max_distance: 30.,
            }),
        }),
    };
    CookedScene::new(id(99), 60, entities)
        .unwrap()
        .with_audio(vec![
            bus(1, Some(id(4)), -3.),
            source(2, false),
            source(3, true),
            bus(4, None, -1.),
            CookedAudio {
                entity: id(5),
                bus: None,
                source: None,
                listener: true,
            },
        ])
        .unwrap()
}
/// Exact identity/u64 words as f64 are safe because each word is only u32.
fn words(out: &mut [f64], id: StableId) {
    for (target, word) in out.iter_mut().zip(id.0.as_chunks::<4>().0) {
        *target = f64::from(u32::from_be_bytes(*word));
    }
}
pub fn observations(world: &NativeWorld) -> [f64; 186] {
    let mut out = [0.; 186];
    let mut count = 0;
    for (node, matrix) in world.audio_nodes() {
        let r = &mut out[count * 36..][..36];
        words(&mut r[..4], node.entity);
        if let Some(bus) = &node.bus {
            r[4] += 1.;
            r[5] = f64::from(bus.parent.is_some());
            if let Some(parent) = bus.parent {
                words(&mut r[6..10], parent);
            }
            r[10] = f64::from(bus.gain_db);
        }
        if let Some(s) = &node.source {
            r[4] += 2.;
            words(&mut r[11..15], s.clip);
            r[15] = f64::from(s.bus.is_some());
            r[16] = f64::from(
                u32::from(s.playing)
                    | (u32::from(s.looping) << 1)
                    | (u32::from(s.streaming) << 2)
                    | (u32::from(s.spatial.is_some()) << 3),
            );
            if let Some(bus) = s.bus {
                words(&mut r[17..21], bus);
            }
            r[21] = f64::from((s.transport_epoch >> 32) as u32);
            r[22] = f64::from(s.transport_epoch as u32);
            r[23..27].copy_from_slice(&[
                f64::from(s.gain_db),
                f64::from(s.pan),
                s.rate,
                s.start_seconds,
            ]);
            if let Some(spatial) = &s.spatial {
                r[27] = f64::from(spatial.min_distance);
                r[28] = f64::from(spatial.max_distance);
                r[29..32].copy_from_slice(&matrix[3][..3]);
            }
        }
        if node.listener {
            r[4] += 4.;
            r[29..32].copy_from_slice(&matrix[3][..3]);
            r[32..36].copy_from_slice(&world.world_rotation(node.entity).unwrap());
        }
        count += 1;
    }
    assert_eq!(count, 5);
    words(&mut out[180..184], world.scene_id());
    out[184] = f64::from((world.tick() >> 32) as u32);
    out[185] = f64::from(world.tick() as u32);
    out
}
pub fn controls(turn: u32) -> Vec<(u128, AudioControl)> {
    use AudioControl::*;
    match turn {
        0 => vec![
            (2, Restart),
            (3, Restart),
            (2, Seek(0.0013)),
            (3, Seek(0.0021)),
        ],
        2 => vec![(2, Stop), (2, Seek(0.0023)), (3, Seek(0.01))],
        4 => vec![(2, SourceGain(-8.)), (1, BusGain(-6.)), (4, BusGain(-3.))],
        5 => vec![(2, SetPlaying(true))],
        8 => vec![(2, Restart), (2, Restart)],
        9 => vec![(3, SetPlaying(false)), (3, Seek(0.0123))],
        11 => vec![(3, SetPlaying(true))],
        13 => vec![(2, Seek(0.01))],
        15 => vec![(2, SetPlaying(true))],
        17 | 30 | 36 => vec![(2, Restart)],
        19 => vec![(3, Stop), (2, Stop)],
        23 => vec![(3, Restart), (3, Seek(0.031))],
        32 => vec![(2, Stop)],
        41 => vec![(3, SetPlaying(false))],
        43 => vec![(3, SetPlaying(true))],
        _ => vec![],
    }
}
pub fn oracle(world: &mut NativeWorld, turn: u32) {
    let values = observations(world);
    world.with_numeric_columns(|view| {
        view.for_each_chunk(|chunk| {
            for (id, transform) in chunk
                .ids
                .iter()
                .zip(chunk.transforms.as_chunks_mut::<10>().0)
            {
                let n = u128::from_be_bytes(id.0);
                if (100..162).contains(&n) {
                    let at = (n as usize - 100) * 3;
                    transform[..3].copy_from_slice(&values[at..at + 3]);
                } else if n == 3 {
                    transform[0] = f64::from(turn % 13) / 16.;
                } else if n == 5 {
                    transform[2] = -f64::from(turn % 7) / 32.;
                }
            }
        });
    });
    let mut queue = FrameCommands::default();
    for (n, control) in controls(turn) {
        queue
            .push(StructuralCommand::ControlAudio {
                entity: id(n),
                control,
            })
            .unwrap();
    }
    world.apply(&mut queue).unwrap();
}
