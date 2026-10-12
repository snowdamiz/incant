#[allow(dead_code)]
#[path = "animation_compound.rs"]
pub mod compound;
pub use compound::id;
use incant_anim::{
    RootMotionCatalog, RootMotionClip, RootMotionDelta, RootMotionTrack,
    track::{Interpolation, Kind},
};
use incant_runtime::*;
use incant_types::RootMotionApplication;
use std::sync::Arc;

pub fn scene() -> CookedScene {
    let base = compound::scene();
    let mut nodes = base.spatial().to_vec();
    for (n, node) in nodes.iter_mut().enumerate() {
        let Some(a) = node.animation.as_ref() else {
            continue;
        };
        let clips = a
            .clip_durations
            .iter()
            .enumerate()
            .map(|(i, &duration)| {
                let angle = (i + 1) as f64 * 0.125;
                RootMotionClip::new(
                    duration,
                    Some(
                        RootMotionTrack::new(
                            vec![0., duration],
                            vec![[0.; 4], [(i + 1) as f32, 0., (i % 2) as f32, 0.]],
                            if i % 2 == 0 {
                                Interpolation::Linear
                            } else {
                                Interpolation::Step
                            },
                            Kind::Vector,
                        )
                        .unwrap(),
                    ),
                    Some(
                        RootMotionTrack::new(
                            vec![0., duration],
                            vec![
                                [0., 0., 0., 1.],
                                [0., angle.sin() as f32, 0., angle.cos() as f32],
                            ],
                            Interpolation::Linear,
                            Kind::Rotation,
                        )
                        .unwrap(),
                    ),
                )
                .unwrap()
            })
            .collect();
        node.root_motion = Some(Arc::new(NativeRootMotion {
            catalog: Arc::new(
                RootMotionCatalog::new(
                    a.mesh_fingerprint,
                    0,
                    [0.; 3],
                    [0., 0., 0., 1.],
                    RootMotionDelta::IDENTITY,
                    clips,
                )
                .unwrap(),
            ),
            application: if n == 0 {
                RootMotionApplication::Transform
            } else {
                RootMotionApplication::ExtractOnly
            },
            applying: n != 1,
        }));
    }
    let mut entities = base.entities().to_vec();
    entities.extend((400..486).map(|n| CookedEntity {
        id: id(n),
        parent: None,
        transform: Default::default(),
        velocity: None,
    }));
    CookedScene::new(id(99), 60, entities)
        .unwrap()
        .with_spatial(base.visual_assets().to_vec(), nodes)
        .unwrap()
}
pub const VALUES: usize = 258;
/// Native numeric observations, independent of the wire encoder/SDK parser.
pub fn observations(w: &NativeWorld) -> [f64; VALUES] {
    let mut rows = Vec::new();
    w.snapshot_root_motion(&mut rows).unwrap();
    let mut values = [0.; VALUES];
    let mut at = 0;
    let mut push = |v| {
        values[at] = v;
        at += 1;
    };
    push(rows.len() as f64);
    for r in rows {
        for bytes in r.entity.0.as_chunks::<4>().0 {
            push(u32::from_be_bytes(*bytes) as f64);
        }
        for bytes in r.fingerprint.as_chunks::<4>().0 {
            push(u32::from_be_bytes(*bytes) as f64);
        }
        push(f64::from(r.root));
        push(match r.application {
            RootMotionApplication::ExtractOnly => 0.,
            RootMotionApplication::Transform => 1.,
            RootMotionApplication::Kinematic3d => 2.,
            RootMotionApplication::Character3d => 3.,
        });
        push(f64::from(r.applying));
        push((r.tick >> 32) as f64);
        push((r.tick as u32) as f64);
        for (i, c) in [Some(r.current), r.outgoing].into_iter().enumerate() {
            if i == 1 {
                push(f64::from(c.is_some()));
            }
            let c = c.unwrap_or(RootMotionClockSnapshot {
                state: None,
                clip: None,
                cycle: 0,
                seconds: 0.,
            });
            push(if c.state.is_some() {
                2.
            } else if c.clip.is_some() {
                1.
            } else {
                0.
            });
            for bytes in c.state.unwrap_or(id(0)).0.as_chunks::<4>().0 {
                push(u32::from_be_bytes(*bytes) as f64);
            }
            push(c.clip.map_or(-1., f64::from));
            push(((c.cycle as u64) >> 32) as f64);
            push((c.cycle as u32) as f64);
            push(c.seconds);
        }
        for d in [
            r.requested,
            r.applied,
            r.accumulated_requested,
            r.accumulated_applied,
        ] {
            for v in d.translation.into_iter().chain(d.rotation) {
                push(v);
            }
        }
    }
    values
}
pub fn oracle(w: &mut NativeWorld, turn: u32) {
    let values = observations(w);
    w.with_numeric_columns(|view| {
        view.for_each_chunk(|chunk| {
            for (id, t) in chunk
                .ids
                .iter()
                .zip(chunk.transforms.as_chunks_mut::<10>().0)
            {
                let n = u128::from_be_bytes(id.0);
                if (400..486).contains(&n) {
                    let offset = (n - 400) as usize * 3;
                    t[..3].copy_from_slice(&values[offset..offset + 3]);
                }
            }
        })
    });
    compound::oracle(w, turn);
    let mut c = FrameCommands::default();
    if [3, 8, 11, 15].contains(&turn) {
        c.push(StructuralCommand::ControlRootMotion {
            entity: id(1),
            control: RootMotionControl::SetApplying(turn == 8 || turn == 15),
        })
        .unwrap();
    }
    if turn == 17 {
        c.push(StructuralCommand::ControlRootMotion {
            entity: id(1),
            control: RootMotionControl::ResetAccumulated,
        })
        .unwrap();
    }
    w.apply(&mut c).unwrap();
}
