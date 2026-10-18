//! Light-transform regression. Owned numerical buffers only; no display surface.
//! Keep the unchecked predecessor and same-phase current-only arms separate from
//! the unconditional sample-PBR reference: their sampling footprints differ.
use super::*;

#[test]
fn radiance_policy_requires_accepted_continuity_and_previous_frame() {
    use super::super::super::continuity::{Camera, Continuity};
    let camera = Camera::new(crate::camera::CameraView::preview(), [0., 0., 48., 48.]);
    let light = incant_render_data::SceneLight::Point(incant_types::PointLight {
        color: [1.; 3],
        intensity: 12.,
        range: 10.,
    });
    let pack = |x| {
        let mut plan = crate::lighting::scene::LightPlan::default();
        plan.add_typed(
            &light,
            glam::DMat4::from_translation(glam::DVec3::new(x, 0., 3.)),
        )
        .unwrap();
        plan.finish().0
    };
    let old = pack(-1.);
    let moved = pack(1.);
    let mut accepted = Continuity::new(old.len());
    accepted.accept(camera, &old);
    assert!(Mode::Missing.admits_history(true, accepted.admits(camera, &old, true)));
    assert!(!Mode::Missing.admits_history(true, accepted.admits(camera, &moved, true)));
    assert!(!Mode::Missing.admits_history(true, accepted.admits(camera, &old, false)));
    // Looking at a rejected candidate cannot mutate the accepted certificate.
    assert!(Mode::Missing.admits_history(true, accepted.admits(camera, &old, true)));
    accepted.accept(camera, &moved);
    assert!(Mode::Missing.admits_history(true, accepted.admits(camera, &moved, true)));
    assert!(!Mode::Missing.admits_history(true, accepted.admits(camera, &old, true)));
    for mode in [
        Mode::Missing,
        Mode::NoHistory,
        Mode::AllCurrent,
        Mode::UncheckedHistory,
    ] {
        assert!(!mode.admits_history(false, true));
    }
    assert!(!Mode::NoHistory.admits_history(true, true));
    assert!(Mode::UncheckedHistory.admits_history(true, false));
}

#[test]
fn radiance_decisions_cannot_request_shading_or_history_for_clear_samples() {
    let visible = Shaded {
        color: [0.; 4],
        depth: 1.,
        mask: 1 << 4,
        visible_depth: 0.2,
        visible_owner: 1,
    };
    assert_eq!(validate_current(&[visible]), 0);
    let history = Shaded {
        mask: (1 << 8) | (1 << 16),
        ..visible
    };
    assert_eq!(validate_current(&[history]), 0);
    assert_eq!(
        validate_current(&[Shaded {
            mask: history.mask | (1 << 4),
            ..visible
        }]),
        0
    );
    for bad in [
        Shaded {
            mask: 1 << 8,
            ..visible
        },
        Shaded { mask: 0, ..visible },
        Shaded {
            mask: 1 << 12,
            ..visible
        },
        Shaded {
            visible_owner: 0,
            ..visible
        },
        Shaded {
            mask: 17,
            color: [1.; 4],
            depth: 0.2,
            ..visible
        },
    ] {
        assert!(std::panic::catch_unwind(|| validate_current(&[bad])).is_err());
    }
}

struct Snapshot {
    pixels: Vec<Pixel>,
    shading: Vec<Shaded>,
    raw: Vec<crate::motion::visibility::Sample>,
    frame: crate::motion::MotionFrame,
    ids: [u32; 2],
    changed: [bool; 2],
}
impl Snapshot {
    fn read(r: &Renderer, scene: &RetainedScene, pixels: &Reader, shading: &ShadedReader) -> Self {
        let ids = [
            scene.visibility_test_owner(0).unwrap(),
            scene.visibility_test_owner(2).unwrap(),
        ];
        Self {
            pixels: pixels.read(r, scene),
            shading: shading.read(r, scene),
            raw: scene.visibility_reconstruction_probe().read_fixture(r),
            frame: scene.motion_frame().unwrap(),
            changed: ids.map(|id| scene.motion_changes().unwrap().changed(id).unwrap()),
            ids,
        }
    }
    fn save(&self, root: &std::path::Path, key: &str) {
        write(
            root,
            &format!("{key}-final.bin"),
            bytemuck::cast_slice(&self.pixels),
        );
        write(
            root,
            &format!("{key}-decisions.bin"),
            bytemuck::cast_slice(&self.shading),
        );
        super::super::super::visibility_tests::save_samples(root, key, &self.raw);
        write(
            root,
            &format!("{key}.json"),
            &serde_json::to_vec_pretty(&json!({
                "sequence": self.frame.sequence(), "epoch": self.frame.epoch(),
                "phase": self.frame.jitter_phase(), "has_previous": self.frame.has_previous(),
                "ids": self.ids, "changed": self.changed,
                "decision_counts": self.counts(),
            }))
            .unwrap(),
        );
    }
    // Counts are diagnostic only. Exact lane decisions/final colors are retained.
    fn counts(&self) -> [usize; 3] {
        std::array::from_fn(|route| {
            self.shading
                .iter()
                .enumerate()
                .filter(|(i, s)| s.mask & (1 << (i % 4 + route * 4)) != 0)
                .count()
        })
    }
    fn validate(&self, pattern: [[f64; 2]; 4], pose: Pose, previous: Pose) {
        oracle::assert_inspection(&oracle::inspect(
            &self.raw,
            pattern,
            &Scene {
                current: pose,
                previous,
                words: std::array::from_fn(|i| oracle::word(self.ids[i], self.changed[i])),
                has_previous: self.frame.has_previous(),
            },
        ));
        validate_shading(&self.raw, &self.shading);
        for (pixel, samples) in self.pixels.iter().zip(self.raw.as_chunks::<4>().0) {
            let covered = samples.iter().filter(|s| s.owner[0] != 0).count();
            assert_eq!(
                pixel.color[3],
                covered as f32 * 0.25,
                "history cannot invent visibility"
            );
            assert!(pixel.color.iter().all(|v| v.is_finite() && *v >= 0.));
            if covered == 0 {
                assert_eq!(pixel.color, [0.; 4]);
                assert_eq!(pixel.owner, 0);
            }
            let owners: Vec<_> = samples
                .iter()
                .filter_map(|s| {
                    let id = s.owner[0] & 0x7fffffff;
                    (id != 0).then_some(id)
                })
                .collect();
            if owners.iter().any(|id| *id != owners[0]) {
                assert_eq!(pixel.owner, 0, "mixed owner history seed");
            }
        }
    }
}
fn assert_same(a: &Snapshot, b: &Snapshot) {
    assert_eq!(a.pixels, b.pixels);
    assert_eq!(a.shading, b.shading);
    assert_eq!(a.frame.sequence(), b.frame.sequence());
    assert_eq!(a.frame.epoch(), b.frame.epoch());
}
fn color_l1(a: &Snapshot, b: &Snapshot) -> f64 {
    a.pixels
        .iter()
        .zip(&b.pixels)
        .flat_map(|(a, b)| a.color.into_iter().zip(b.color))
        .map(|(a, b)| f64::from((a - b).abs()))
        .sum()
}
fn light(input: &mut SceneInput, x: f64) {
    input.nodes[4].world = glam::DMat4::from_translation(glam::DVec3::new(x, -1.59, 0.25));
}

#[test]
#[ignore = "coordinated light-transform owned-buffer regression; no window/OS capture/audio"]
fn light_transform_history_requires_accepted_radiance_continuity() {
    let root =
        std::path::PathBuf::from(std::env::var_os("INCANT_VISIBILITY_RADIANCE_OUTPUT").unwrap());
    std::fs::create_dir(&root).unwrap();
    let mut r = crate::test_support::checked(Renderer::headless().unwrap());
    pollster::block_on(r.enable_temporal_upscale()).unwrap();
    pollster::block_on(r.enable_motion_owner()).unwrap();
    let calibration = Probe::new(&r).calibrate(&r);
    let pattern = oracle::calibration(&calibration).unwrap();
    super::super::super::visibility_tests::save_samples(&root, "calibration", &calibration);
    let pixels = Reader::new(&r);
    let shading = ShadedReader::new(&r);
    let size = [64, 64];
    let texture = crate::hdr_output::texture(&r, wgpu::TextureFormat::Rgba8UnormSrgb, 64, 64);
    let view = texture.create_view(&Default::default());
    let mut input = pbr_fixture::scene(&[false]);
    if let Some(incant_render_data::SceneLight::Point(p)) = &mut input.nodes[4].light {
        p.range = 0.6;
    }
    let typed_light = input.nodes[4].light.clone();
    let initial = Pose::initial();
    apply(&mut input, initial);
    light(&mut input, -1.);
    let modes = [
        Mode::Missing,
        Mode::UncheckedHistory,
        Mode::NoHistory,
        Mode::AllCurrent,
        Mode::Missing,
    ];
    let names = [
        "candidate",
        "unchecked",
        "no-history",
        "all-current",
        "untouched",
    ];
    let mut owners = modes.map(|mode| current_owner(&r, &input, size, mode));
    let mut rows = Vec::new();
    let mut previous = initial;
    // Warm past a complete eight-phase cycle, then change the light at phase
    // six: retained GPU001 data has 48 missing current witnesses at that phase,
    // while phase zero has none. Keep the positive fresh-PBR assertion meaningful.
    // The unchecked arm and all coverage/first-winner thresholds are unchanged.
    for index in 0..18 {
        let pose = if index == 16 {
            Pose {
                mesh_y: 6.,
                ..initial
            }
        } else {
            initial
        };
        apply(&mut input, pose);
        light(&mut input, if index < 14 { -1. } else { 1. });
        assert!(
            input.nodes[4].light == typed_light,
            "only light world transform changes"
        );
        for owner in &mut owners {
            submit(&r, owner, &input, &view, size);
        }
        if index != 0 && index < 12 {
            continue;
        }
        let snapshots = owners
            .each_ref()
            .map(|owner| Snapshot::read(&r, owner, &pixels, &shading));
        for (name, snapshot) in names.iter().zip(&snapshots) {
            snapshot.save(&root, &format!("frame-{index:02}-{name}"));
            snapshot.validate(pattern, pose, previous);
            assert_eq!(snapshot.frame.sequence(), index + 1);
            assert_eq!(snapshot.frame.epoch(), 1);
        }
        assert_same(&snapshots[0], &snapshots[4]);
        let admitted = index != 0 && ![14, 16, 17].contains(&index);
        assert!(
            snapshots[0]
                .shading
                .iter()
                .all(|s| (s.mask & (1 << 16) != 0) == admitted)
        );
        if !admitted {
            assert_eq!(snapshots[0].counts()[2], 0, "discontinuous radiance reused");
            assert_same(&snapshots[0], &snapshots[2]);
        }
        if index == 13 || index == 15 {
            assert!(
                snapshots[0].counts()[2] > 0,
                "stable history must actually be exercised"
            );
        }
        let mut exact_fallback = 0;
        for (i, (a, b)) in snapshots[0]
            .shading
            .iter()
            .zip(&snapshots[3].shading)
            .enumerate()
        {
            if a.mask & (1 << (i % 4)) != 0 {
                assert_eq!(a.color, b.color);
                assert_eq!(a.depth, b.depth);
                exact_fallback += 1;
            }
        }
        let negative_l1 = color_l1(&snapshots[0], &snapshots[1]);
        if index == 14 {
            assert!(
                !snapshots[0].changed[0],
                "geometry did not move with the light"
            );
            assert!(snapshots[0].frame.has_previous());
            assert!(
                snapshots[1].counts()[2] > 0,
                "negative must reuse old radiance"
            );
            assert!(
                exact_fallback > 0,
                "unobserved current samples must use fresh PBR"
            );
            assert!(
                negative_l1 > 0.,
                "old lighting must be observable, not just a flag"
            );
        }
        rows.push(json!({"frame":index,"pose":[pose.mesh_y,pose.camera_y,pose.occluder_y,pose.occluder_z],
            "point_translation":if index<14 {[-1.,-1.59,0.25]}else{[1.,-1.59,0.25]},
            "candidate_history_admitted":admitted,"exact_fallback_samples":exact_fallback,
            "unchecked_output_L1":negative_l1,"all_current_output_L1_not_acceptance":color_l1(&snapshots[0],&snapshots[3])}));
        if index == 12 {
            // A candidate with a different packed light reaches output-format
            // refusal. Neither accepted pixels nor the light certificate may
            // change. Next accepted old-light output equals an untouched owner.
            light(&mut input, 1.);
            let error = owners[0]
                .submit(
                    &r,
                    &input,
                    RetainedTarget {
                        view: &view,
                        format: wgpu::TextureFormat::R32Float,
                        width: 64,
                        height: 64,
                        viewport: None,
                    },
                )
                .unwrap_err();
            assert!(matches!(
                error.downcast_ref::<crate::ResourceError>(),
                Some(crate::ResourceError::OutputFormat(
                    wgpu::TextureFormat::R32Float
                ))
            ));
            let after = Snapshot::read(&r, &owners[0], &pixels, &shading);
            after.save(&root, "rejected-light");
            assert_same(&after, &snapshots[0]);
            write(&root, "refusal.json", &serde_json::to_vec_pretty(&json!({
                "error":error.to_string(),"accepted_sequence":after.frame.sequence(),
                "accepted_epoch":after.frame.epoch(),"attempted_point_translation":[1.,-1.59,0.25],
                "next_untouched_comparison_frame":13,
            })).unwrap());
        }
        previous = pose;
    }
    write(
        &root,
        "result.json",
        &serde_json::to_vec_pretty(&json!({
            "source_baseline":"cc8c023d442ce315dae36b04ea24bb01781a53a4","frames":rows,
            "submitted_frames":18,"light_change_frame":14,"light_change_phase":6,
            "sample_positions":pattern,"output":size,"bytes_per_output_pixel":236,
            "mask_bits":{"fallback":[0,3],"current":[4,7],"history":[8,11],"history_policy":16},
            "same_phase_no_history_reference":true,"refusal_nonpublication":true,
            "global_stationary_gate_not_dynamic_gameplay_qualification":true,
            "appearance_accepted":false,"performance_accepted":false,
        }))
        .unwrap(),
    );
}
