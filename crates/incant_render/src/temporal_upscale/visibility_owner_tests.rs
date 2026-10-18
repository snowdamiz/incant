//! Capture completion is distinct from visibility acceptance: the unchanged
//! baseline is known to fail and must not prevent collecting the other arms.
use super::{
    raster_geometry as geometry,
    raster_readback::Reader,
    raster_tests::{render, write},
    visibility_oracle::{self as oracle, OCCLUDER, Pose, Scene},
    visibility_tests::{apply, prepare, save, save_samples},
};
use crate::{
    Renderer,
    motion::visibility::{MslSession, Probe, Sample, Storage, StorageRecord},
};
use serde_json::json;
use std::sync::Arc;
#[path = "visibility_compact_tests.rs"]
mod compact_tests;

pub(super) fn normalized(raw: &[Sample]) -> Vec<Sample> {
    raw.iter()
        .map(|sample| {
            let mut sample = *sample;
            sample.owner[1..].fill(0);
            sample
        })
        .collect()
}
pub(super) fn packing_errors(raw: &[Sample], explicit: bool) -> Vec<usize> {
    raw.iter()
        .enumerate()
        .filter_map(|(i, s)| {
            let halves = s.owner[1] <= 65535
                && s.owner[2] <= 65535
                && s.owner[0] == (s.owner[1] | (s.owner[2] << 16));
            let valid_low = if explicit && s.owner[0] != 0 {
                s.owner[3] == (i % 4) as u32
            } else {
                s.owner[3] == 0
            };
            (!halves || !valid_low).then_some(i)
        })
        .collect()
}
#[test]
fn visibility_owner_raw_lanes_detect_changed_halves_and_wrong_sample() {
    let mut raw = vec![
        Sample {
            motion: [0.; 4],
            position_depth: [0.; 4],
            owner: [0; 4]
        };
        4
    ];
    raw[3].owner = [0x80000002, 2, 32768, 3];
    assert!(packing_errors(&raw, true).is_empty());
    assert_eq!(packing_errors(&raw, false), [3]);
    raw[3].owner[1] = 1;
    assert_eq!(packing_errors(&raw, true), [3]);
    assert_eq!(normalized(&raw)[3].owner, [0x80000002, 0, 0, 0]);
    raw[3].owner = [0x80000002, 2, 32768, 1];
    assert_eq!(packing_errors(&raw, true), [3]);
}
#[test]
#[ignore = "coordinated four-arm visibility diagnostic; capture success is not geometry acceptance"]
fn output_visibility_owner_depth_four_arm_capture() {
    let root =
        std::path::PathBuf::from(std::env::var_os("INCANT_VISIBILITY_OWNER_OUTPUT").unwrap());
    capture(&root, Capture::Owner);
}

#[test]
#[ignore = "coordinated MRT storage diagnostic; capture success is not geometry acceptance"]
fn output_visibility_owner_storage_three_arm_capture() {
    let root =
        std::path::PathBuf::from(std::env::var_os("INCANT_VISIBILITY_STORAGE_OUTPUT").unwrap());
    capture(&root, Capture::Storage);
}

#[test]
#[ignore = "coordinated offscreen numerical MRT slot diagnostic; no desktop capture"]
fn output_visibility_owner_slot_two_arm_capture() {
    let root = std::path::PathBuf::from(std::env::var_os("INCANT_VISIBILITY_SLOT_OUTPUT").unwrap());
    capture(&root, Capture::Slots);
}
#[test]
#[ignore = "coordinated offscreen numerical owner dependency diagnostic; no desktop capture"]
fn output_visibility_owner_dependency_two_arm_capture() {
    let root =
        std::path::PathBuf::from(std::env::var_os("INCANT_VISIBILITY_DEPENDENCY_OUTPUT").unwrap());
    capture(&root, Capture::Dependency);
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Capture {
    Owner,
    Storage,
    Slots,
    Dependency,
    Compact,
}
fn capture(root: &std::path::Path, mode: Capture) {
    let storage_probe = mode != Capture::Owner;
    std::fs::create_dir(root).unwrap();
    let mut renderer = crate::test_support::checked(Renderer::headless().unwrap());
    assert_eq!(renderer.adapter.get_info().backend, wgpu::Backend::Metal);
    pollster::block_on(renderer.enable_temporal_upscale()).unwrap();
    let output = crate::hdr_output::texture(&renderer, wgpu::TextureFormat::Rgba8UnormSrgb, 64, 64);
    let view = output.create_view(&Default::default());
    let reader = Reader::new(&renderer);
    let _session = MslSession::start();
    let mut pattern = None;
    let mut results = Vec::new();
    let mut compact_reference = Vec::new();
    let mut compact_low = Vec::new();
    let arms = match mode {
        Capture::Storage => vec![
            ("original-forward", false, false, Storage::Original),
            ("redundant-forward", false, false, Storage::Redundant),
            ("rgba-forward", false, false, Storage::Rgba),
        ],
        Capture::Owner => vec![
            ("original-forward", false, false, Storage::Original),
            ("original-reverse", false, true, Storage::Original),
            ("explicit-forward", true, false, Storage::Original),
            ("explicit-reverse", true, true, Storage::Original),
        ],
        Capture::Slots => vec![
            ("rgba-forward", false, false, Storage::Rgba),
            ("rgba-swapped", false, false, Storage::Slots),
        ],
        Capture::Dependency => vec![
            ("rgba-forward", false, false, Storage::Rgba),
            ("rgba-sample-x", false, false, Storage::SampleX),
        ],
        Capture::Compact => vec![
            ("rgba-sample-x", false, false, Storage::SampleX),
            ("compact-three-plane", false, false, Storage::Compact),
        ],
    };
    let frame_count = 11 * arms.len();
    for (arm_index, (name, explicit, reverse, storage)) in arms.into_iter().enumerate() {
        let path = root.join(name);
        std::fs::create_dir(&path).unwrap();
        let probe = Arc::new(if storage_probe {
            Probe::storage_diagnostic(&renderer, &path, storage)
        } else {
            Probe::owner_diagnostic(&renderer, &path, explicit, reverse)
        });
        if pattern.is_none() {
            let raw = probe.calibrate(&renderer);
            save_samples(root, "calibration-raw", &raw);
            assert!(packing_errors(&raw, false).is_empty());
            pattern = Some(oracle::calibration(&normalized(&raw)).unwrap());
            write(root,"fixture.json",&serde_json::to_vec_pretty(&json!({"measured_sample_offsets":pattern,
                "adapter":format!("{:?}",renderer.adapter.get_info()),"edge":geometry::EDGE,"strip":geometry::STRIP,"occluder":OCCLUDER,
                "layout":"48 bytes little-endian: motion4 f32, sample_position.xy/current_depth/compute_lane f32, decoded_owner/raw_owner_lo/raw_owner_hi/raw_validity_lo u32",
                "explicit_sample_index":"raw_validity_lo, only for covered explicit arms; original has no observed fragment index",
                "validity_decoding":"explicit arm clears only low two bits before f32 reconstruction; original arm unmodified",
                "geometry_tie_distance_pixels":1e-5,"position_epsilon_pixels":2e-5,"depth_motion_epsilon":2e-6,
                "storage_probe":storage_probe,"slot_probe":mode == Capture::Slots,
                "dependency_probe":mode == Capture::Dependency,
                "sample_companion":"rgba-sample-x only: ownerXY unchanged, ownerZW holds exact f32 sample_position.x low/high16; uncovered RGBA remains zero",
                "storage_record":"Original storage:48-byte semantic record; all other storage:80 bytes = same48-byte prefix + raw motionZW4xu32 + raw ownerRGBA4xu32",
                "redundant_packing":"finite positive-sign depth[0,1]; validity+0/+1 in depth sign bit; full owner in original validity lanes; invalid inputs yield NaN refusal",
                "no_new_color_resolver":true,"capture_completion_is_acceptance":false})).unwrap());
            if mode == Capture::Compact {
                write(root, "compact-layout.json", &serde_json::to_vec_pretty(&json!({
                    "physical_planes": ["motion.xy RGBA16Uint4", "previous depth/validity + owner RGBA16Uint4", "sample.xy RGBA16Uint4", "current Depth32Float4"],
                    "visibility_bytes_per_pixel":112,"reference_visibility_bytes_per_pixel":144,
                    "duplicate_X_validation":false,"reference_duplicate_X_validation":true,
                    "compact_raw_owner_words":"absent plane represented by four reserved zero words, never a duplicate-X witness",
                    "signed_zero":"depth sign in bit30; binary validity in bit31; positive depth magnitude unchanged",
                    "comparison":"all 16384 samples of all 11 frames, exact owner/motion/sample/current+previous depth bits and low planes",
                    "reconstruction_and_fallback_unchanged":true,"packing_is_not_a_driver_cause_claim":true
                })).unwrap());
            }
        }
        let pattern = pattern.unwrap();
        let initial = Pose::initial();
        let mut input = geometry::fixture();
        let mut occluder = geometry::fixture_with(&[OCCLUDER], [0.; 3], 1)
            .nodes
            .remove(0);
        occluder.id = "occluder".into();
        input.nodes.push(occluder);
        apply(&mut input, initial);
        let mut retained = prepare(&renderer, &input, &probe);
        let mut previous = initial;
        let poses = [
            initial,
            initial,
            initial,
            initial,
            initial,
            initial,
            initial,
            initial,
            Pose {
                mesh_y: 0.125,
                ..initial
            },
            Pose {
                mesh_y: 0.125,
                camera_y: -0.125,
                ..initial
            },
            Pose {
                occluder_y: 0.,
                ..initial
            },
        ];
        for (index, current) in poses.into_iter().enumerate() {
            apply(&mut input, current);
            let low = render(&renderer, &mut retained, &input, &view, &reader);
            let extra = (storage != Storage::Original).then(|| probe.read_storage(&renderer));
            let raw = extra.as_ref().map_or_else(
                || probe.read(&renderer),
                |records| records.iter().map(|v| v.sample).collect(),
            );
            let samples = normalized(&raw);
            let comparison = if mode == Capture::Compact {
                let low_bits: Vec<_> = low.0.iter().flatten().map(|v| v.to_bits()).collect();
                if arm_index == 0 {
                    compact_reference.push(samples.clone());
                    compact_low.push(low_bits);
                    None
                } else {
                    Some((
                        compact_tests::differences(&samples, &compact_reference[index]),
                        low_bits == compact_low[index],
                    ))
                }
            } else {
                None
            };
            let frame = retained.motion_frame().unwrap();
            let changes = retained.motion_changes().unwrap();
            assert!(frame.same_owner(changes.frame()));
            assert_eq!(frame.epoch(), 1);
            assert_eq!(frame.sequence(), index as u64 + 1);
            assert_eq!(frame.jitter_phase(), Some(index as u32 % 8));
            assert_eq!(frame.has_previous(), index != 0);
            let ids = [
                retained.visibility_test_owner(0).unwrap(),
                retained.visibility_test_owner(2).unwrap(),
            ];
            let moved = [
                current.mesh_y != previous.mesh_y,
                current.occluder_y != previous.occluder_y
                    || current.occluder_z != previous.occluder_z,
            ];
            for (id, moved) in ids.into_iter().zip(moved) {
                assert_eq!(changes.changed(id), Some(index == 0 || moved));
            }
            let words = ids.map(|id| oracle::word(id, changes.changed(id).unwrap()));
            let inspection = oracle::inspect(
                &samples,
                pattern,
                &Scene {
                    current,
                    previous,
                    words,
                    has_previous: frame.has_previous(),
                },
            );
            let packed = extra.as_ref().map_or_else(
                || packing_errors(&raw, explicit),
                |records| {
                    if storage == Storage::Compact {
                        compact_tests::packing_errors(records)
                    } else {
                        storage_packing_errors(records, storage)
                    }
                },
            );
            let duplicate = extra
                .as_ref()
                .filter(|_| storage == Storage::Redundant)
                .map(|records| {
                    let duplicate_samples: Vec<_> = records
                        .iter()
                        .map(|v| {
                            let mut s = v.sample;
                            s.owner = [v.raw_motion_zw[2] | (v.raw_motion_zw[3] << 16), 0, 0, 0];
                            s
                        })
                        .collect();
                    oracle::inspect(
                        &duplicate_samples,
                        pattern,
                        &Scene {
                            current,
                            previous,
                            words,
                            has_previous: frame.has_previous(),
                        },
                    )
                });
            if index == 7 {
                write(
                    &path,
                    "static-area.json",
                    &serde_json::to_vec_pretty(&oracle::static_metrics(&samples, ids[0])).unwrap(),
                );
            }
            let accepted = inspection["mismatches"].as_array().unwrap().is_empty()
                && packed.is_empty()
                && comparison
                    .as_ref()
                    .is_none_or(|(diff, low)| diff.is_empty() && *low)
                && duplicate
                    .as_ref()
                    .is_none_or(|v| v["mismatches"].as_array().unwrap().is_empty());
            if mode == Capture::Compact || index == 10 || !accepted {
                if let Some(extra) = &extra {
                    // The 80-byte record already contains all original semantic fields.
                    // Retain it once; avoid another identical 48-byte plane in the packet.
                    let low_bytes: Vec<_> = low
                        .0
                        .iter()
                        .flatten()
                        .flat_map(|v| v.to_le_bytes())
                        .collect();
                    write(&path, &format!("frame-{index}-low.f32le"), &low_bytes);
                    write(
                        &path,
                        &format!("frame-{index}-storage.bin"),
                        bytemuck::cast_slice(extra),
                    );
                } else {
                    save(&path, &format!("frame-{index}"), &low, &raw);
                }
            }
            let report = json!({"storage_arm":storage.name(),"duplicate_owner_oracle":duplicate,"frame_index":index,"sequence":frame.sequence(),"epoch":frame.epoch(),"phase":frame.jitter_phase(),
                "has_previous":frame.has_previous(),"owner_words":words,"explicit":explicit,"reverse":reverse,
                "current_pose":[current.mesh_y,current.camera_y,current.occluder_y,current.occluder_z],
                "previous_pose":[previous.mesh_y,previous.camera_y,previous.occluder_y,previous.occluder_z],
                "oracle":inspection,"packing_mismatches":packed,"visibility_accepted":accepted});
            write(
                &path,
                &format!("frame-{index}.json"),
                &serde_json::to_vec_pretty(&report).unwrap(),
            );
            if let Some((differences, low_exact)) = &comparison {
                write(&path, &format!("frame-{index}-comparison.json"), &serde_json::to_vec_pretty(&json!({
                    "exact_semantic_differences":differences,"low_planes_exact":low_exact,
                    "compared_samples":samples.len(),"duplicate_X_validation":false,
                    "reference":"rgba-sample-x, separately rendered unchanged shader and geometric oracle"
                })).unwrap());
            }
            if index < 10 {
                assert!(
                    accepted,
                    "unexpected failure before arriving occluder: {report}"
                );
            }
            if storage_probe && mode != Capture::Compact && arm_index == 0 && index == 10 {
                let errors = inspection["mismatches"].as_array().unwrap();
                assert_eq!(
                    errors.len(),
                    48,
                    "baseline did not reproduce; no repair inference"
                );
                for (x, e) in (6..54).zip(errors) {
                    assert_eq!(
                        [e[0].as_u64(), e[1].as_u64(), e[2].as_u64()],
                        [Some(x), Some(44), Some(3)],
                        "baseline discrepancy changed"
                    );
                }
                assert!(packed.is_empty());
            }
            if index == 10 {
                results.push(json!({"arm":name,"visibility_accepted":accepted,"oracle_mismatches":inspection["mismatches"].as_array().unwrap().len(),"packing_mismatches":packed.len(),"duplicate_owner_oracle":duplicate}));
            }
            previous = current;
        }
    }
    write(
        root,
        "capture.json",
        &serde_json::to_vec_pretty(&json!({"capture_complete":true,"arms":results,
        "visibility_accepted":results.iter().all(|v|v["visibility_accepted"]==true),
        "accepted_frames":frame_count,"visibility_passes":frame_count+1,"baseline_reproduction_required":storage_probe && mode != Capture::Compact,"original_oracle_unchanged":true,"appearance_or_performance_accepted":false}))
        .unwrap(),
    );
    if mode == Capture::Compact {
        assert!(
            results.iter().all(|v| v["visibility_accepted"] == true),
            "compact result differs; raw data retained"
        );
    }
}

fn storage_packing_errors(raw: &[StorageRecord], storage: Storage) -> Vec<usize> {
    raw.iter()
        .enumerate()
        .filter_map(|(i, r)| {
            let v = r.raw_motion_zw;
            let o = r.raw_owner;
            let s = r.sample;
            let bits = if storage == Storage::Redundant {
                [
                    v[0] | ((v[1] & 32767) << 16),
                    if v[1] & 32768 == 0 { 0 } else { 1f32.to_bits() },
                ]
            } else {
                [v[0] | (v[1] << 16), v[2] | (v[3] << 16)]
            };
            let decoded = o[0] | (o[1] << 16);
            let owner_range = o.into_iter().all(|x| x <= 65535);
            let spare_lanes = match storage {
                Storage::SampleX if s.position_depth[2] == 1. => o == [0; 4],
                Storage::SampleX => (o[2] | (o[3] << 16)) == s.position_depth[0].to_bits(),
                Storage::Rgba | Storage::Slots => o[2..] == [0, 0],
                _ => true,
            };
            let good = v.into_iter().all(|x| x <= 65535)
                && owner_range
                && s.owner == [decoded, o[0], o[1], v[2]]
                && s.motion[2].to_bits() == bits[0]
                && s.motion[3].to_bits() == bits[1]
                && spare_lanes;
            (!good).then_some(i)
        })
        .collect()
}
#[test]
fn visibility_storage_raw_records_preserve_distinct_owners_and_refuse_corruption() {
    let mut r = StorageRecord {
        sample: Sample {
            motion: [0.; 4],
            position_depth: [0.; 4],
            owner: [0x80000002, 2, 32768, 1],
        },
        raw_motion_zw: [0, 0, 1, 32768],
        raw_owner: [2, 32768, 0, 1],
    };
    // Different duplicate owner is evidence, not a packing error silently repaired.
    assert!(storage_packing_errors(&[r], Storage::Redundant).is_empty());
    assert_ne!(
        r.sample.owner[0],
        r.raw_motion_zw[2] | (r.raw_motion_zw[3] << 16)
    );
    r.raw_motion_zw[1] = 32768;
    assert_eq!(storage_packing_errors(&[r], Storage::Redundant), [0]);
    r.sample.motion[3] = 1.;
    assert!(storage_packing_errors(&[r], Storage::Redundant).is_empty());
    r.raw_owner[0] = 1;
    assert_eq!(storage_packing_errors(&[r], Storage::Redundant), [0]);
    r.raw_owner[0] = 2;
    assert_eq!(storage_packing_errors(&[r], Storage::Rgba), [0]);
}

#[test]
fn visibility_dependency_companion_checks_position_bits_and_uncovered_zero() {
    let x = 6.625f32;
    let mut r = StorageRecord {
        sample: Sample {
            motion: [0.; 4],
            position_depth: [x, 44.875, 0.1783567, 3.],
            owner: [0x80000002, 2, 32768, 0],
        },
        raw_motion_zw: [0; 4],
        raw_owner: [2, 32768, x.to_bits() & 65535, x.to_bits() >> 16],
    };
    assert!(storage_packing_errors(&[r], Storage::SampleX).is_empty());
    assert_eq!(storage_packing_errors(&[r], Storage::Rgba), [0]);
    r.raw_owner[2] ^= 1;
    assert_eq!(storage_packing_errors(&[r], Storage::SampleX), [0]);
    r.raw_owner[2] ^= 1;
    r.sample.position_depth[0] = x + 0.125;
    assert_eq!(storage_packing_errors(&[r], Storage::SampleX), [0]);
    r.sample.position_depth = [0., 0., 1., 3.];
    r.sample.owner = [0; 4];
    r.raw_owner = [0; 4];
    assert!(storage_packing_errors(&[r], Storage::SampleX).is_empty());
    r.raw_owner[3] = x.to_bits() >> 16;
    assert_eq!(storage_packing_errors(&[r], Storage::SampleX), [0]);
    r.raw_owner = [1, 0, 0, 0];
    r.sample.owner = [1, 1, 0, 0];
    assert_eq!(storage_packing_errors(&[r], Storage::SampleX), [0]);
}
