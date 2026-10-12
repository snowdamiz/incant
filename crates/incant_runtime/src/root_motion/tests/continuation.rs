use super::*;
use sha2::{Digest, Sha256};
fn rehash(bytes: &mut [u8]) {
    let mut hash = Sha256::new();
    hash.update(&bytes[..32]);
    hash.update(&bytes[64..]);
    bytes[32..64].copy_from_slice(&hash.finalize());
}
#[test]
fn v11_exact_continuation_retains_cycles_and_both_active_or_paused_fade_owners() {
    for mode in 0..3 {
        let scene = if mode == 2 {
            tree::scene()
        } else {
            let mut scene = native_scene(RootMotionApplication::Transform, false);
            if mode == 1 {
                let (_, mut playback) = fixture();
                graph(&mut playback, false);
                scene.spatial[0].animation = Some(playback.definition);
            }
            scene
        };
        for paused in [false, true] {
            let mut first = NativeWorld::new(scene.clone());
            first.step().unwrap();
            if mode == 0 {
                command(
                    &mut first,
                    StructuralCommand::ControlAnimation {
                        entity: id(10),
                        control: AnimationControl::Crossfade {
                            target: AnimationSample {
                                clip: Some(1),
                                seconds: 3.9,
                            },
                            duration: 0.3,
                        },
                    },
                );
            }
            command(
                &mut first,
                StructuralCommand::ControlAnimation {
                    entity: id(10),
                    control: AnimationControl::SetSpeed(-2.),
                },
            );
            first.step().unwrap();
            if paused {
                command(
                    &mut first,
                    StructuralCommand::ControlAnimation {
                        entity: id(10),
                        control: AnimationControl::SetPlaying(false),
                    },
                );
            }
            let saved = first.save_bytes().unwrap();
            assert_eq!(&saved[8..12], &11u32.to_le_bytes());
            let mut restored = NativeWorld::from_save_bytes_for_scene(&saved, &scene).unwrap();
            assert_eq!(restored.save_bytes().unwrap(), saved);
            assert_eq!(observation(&first), observation(&restored));
            for tick in 0..40 {
                if tick == 3 && paused {
                    for world in [&mut first, &mut restored] {
                        command(
                            world,
                            StructuralCommand::ControlAnimation {
                                entity: id(10),
                                control: AnimationControl::SetPlaying(true),
                            },
                        );
                    }
                }
                first.step().unwrap();
                restored.step().unwrap();
                assert_eq!(first.snapshot(), restored.snapshot());
                assert_eq!(observation(&first), observation(&restored));
                assert_eq!(first.save_bytes().unwrap(), restored.save_bytes().unwrap());
            }
        }
    }
}

#[test]
fn v11_checksum_valid_malformed_rows_and_downgrades_preserve_live_world() {
    let scene = tree::scene();
    let mut world = NativeWorld::new(scene.clone());
    world.step().unwrap();
    world.step().unwrap();
    let good = world.save_bytes().unwrap();
    let base = good.len() - 304;
    let mut cases = Vec::new();
    for offset in [16, 17, 33, 36, 57, 60] {
        let mut bad = good.clone();
        bad[base + offset] = 255;
        cases.push(bad);
    }
    let mut bad = good.clone();
    bad[base + 16] &= !2;
    cases.push(bad); // nonzero absent outgoing cursor
    let mut bad = good.clone();
    bad[base + 24..base + 32].copy_from_slice(&u64::MAX.to_le_bytes());
    cases.push(bad);
    let mut bad = good.clone();
    bad[base + 48..base + 56].copy_from_slice(&f64::NAN.to_le_bytes());
    cases.push(bad);
    let mut bad = good.clone();
    bad[base + 80..base + 88].copy_from_slice(&f64::INFINITY.to_le_bytes());
    cases.push(bad);
    let mut bad = good.clone();
    bad[base + 104..base + 136].fill(0);
    cases.push(bad);
    let mut bad = good.clone();
    bad[116..120].copy_from_slice(&0u32.to_le_bytes());
    cases.push(bad);
    let mut old = good.clone();
    old.truncate(base);
    old.drain(116..120);
    old[8..12].copy_from_slice(&10u32.to_le_bytes());
    let length = (old.len() - 64) as u64;
    old[24..32].copy_from_slice(&length.to_le_bytes());
    cases.push(old);
    for mut bad in cases {
        rehash(&mut bad);
        assert!(world.restore_bytes(&bad).is_err());
        assert_eq!(world.save_bytes().unwrap(), good);
    }
    let mut forged = native_scene(RootMotionApplication::Transform, false);
    let (other, _) = fixture_rotations(true);
    Arc::make_mut(forged.spatial[0].root_motion.as_mut().unwrap()).catalog = other.catalog;
    let mut payload = NativeWorld::new(forged).save_bytes().unwrap();
    payload[64..96].copy_from_slice(&good[64..96]);
    rehash(&mut payload);
    assert!(matches!(
        world.restore_bytes(&payload),
        Err(SaveError::Invalid("root-motion original captured binding"))
    ));
    assert_eq!(world.save_bytes().unwrap(), good);
}
