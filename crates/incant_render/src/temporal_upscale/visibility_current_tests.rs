//! New current-PBR arm. The original missing-radiance control is unchanged.
use super::*;
use crate::temporal_upscale::visibility_fallback::{self, Mode};
#[path = "visibility_current_readback.rs"]
mod current_readback;
use current_readback::{Reader as ShadedReader, Sample as Shaded};
#[path = "visibility_current_fixture.rs"]
mod pbr_fixture;
#[path = "visibility_radiance_tests.rs"]
mod radiance;

#[test]
fn current_radiance_shaders_validate_uniform_texture_sampling() {
    for source in [
        crate::motion::visibility_shading::source(),
        visibility_fallback::classify_source(Mode::Missing),
        visibility_fallback::classify_source(Mode::AllCurrent),
        visibility_fallback::resolve_source(Mode::Missing),
        visibility_fallback::resolve_source(Mode::AllCurrent),
        visibility_fallback::classify_source(Mode::NoHistory),
        visibility_fallback::resolve_source(Mode::NoHistory),
        visibility_fallback::classify_source(Mode::UncheckedHistory),
        visibility_fallback::resolve_source(Mode::UncheckedHistory),
    ] {
        let m = naga::front::wgsl::parse_str(&source).unwrap();
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&m)
        .unwrap();
    }
    assert_eq!(
        crate::temporal_upscale::visibility_reconstruction::fallback_bytes([64, 64], true).unwrap(),
        966656
    );
    assert!(
        crate::temporal_upscale::visibility_reconstruction::fallback_bytes([129, 64], true)
            .is_err()
    );
}
fn validate_shading(raw: &[crate::motion::visibility::Sample], shaded: &[Shaded]) -> usize {
    assert_eq!(raw.len(), shaded.len());
    for (v, s) in raw.iter().zip(shaded) {
        assert_eq!(v.owner[0], s.visible_owner);
        assert_eq!(v.position_depth[2].to_bits(), s.visible_depth.to_bits());
    }
    validate_current(shaded)
}
fn validate_current(shaded: &[Shaded]) -> usize {
    let mut filled = 0;
    for (i, s) in shaded.iter().enumerate() {
        assert!(
            s.color
                .iter()
                .chain([&s.depth, &s.visible_depth])
                .all(|v| v.is_finite())
        );
        assert_eq!(s.mask & !0x10fff, 0);
        let requested = s.mask & (1 << (i % 4)) != 0;
        let current = s.mask & (1 << (i % 4 + 4)) != 0;
        let history = s.mask & (1 << (i % 4 + 8)) != 0;
        assert!(
            !history || s.mask & (1 << 16) != 0,
            "history without admission"
        );
        assert!(
            !requested || !(current || history),
            "redundant current shading"
        );
        if s.visible_owner & 0x7fffffff == 0 {
            assert!(
                !(requested || current || history),
                "clear sample has a witness"
            );
        } else {
            assert!(
                requested || current || history,
                "visible sample has no radiance route"
            );
        }
        if requested {
            assert_ne!(s.visible_owner & 0x7fffffff, 0, "clear sample requested");
            assert_eq!(s.color[3], 1., "requested sample unwritten");
            assert_eq!(
                s.depth.to_bits(),
                s.visible_depth.to_bits(),
                "wrong depth winner"
            );
            filled += 1;
        } else {
            assert_eq!(s.color, [0.; 4]);
            assert_eq!(s.depth, 1.);
        }
    }
    filled
}
#[test]
fn current_radiance_oracle_refuses_missing_unrequested_and_wrong_depth() {
    let raw = crate::motion::visibility::Sample {
        motion: [0.; 4],
        position_depth: [0.5, 0.5, 0.2, 1.],
        owner: [1, 0, 0, 0],
    };
    let good = Shaded {
        color: [0.2, 0.4, 0.6, 1.],
        depth: 0.2,
        mask: 1,
        visible_depth: 0.2,
        visible_owner: 1,
    };
    assert_eq!(validate_shading(&[raw], &[good]), 1);
    for bad in [
        Shaded {
            color: [0.; 4],
            ..good
        },
        Shaded { mask: 0, ..good },
        Shaded { depth: 0.3, ..good },
        Shaded { mask: 17, ..good },
    ] {
        assert!(std::panic::catch_unwind(|| validate_shading(&[raw], &[bad])).is_err());
    }
    let clear = crate::motion::visibility::Sample {
        owner: [0; 4],
        ..raw
    };
    assert!(std::panic::catch_unwind(|| validate_shading(&[clear], &[good])).is_err());
}
fn current_owner(r: &Renderer, input: &SceneInput, size: [u32; 2], mode: Mode) -> RetainedScene {
    let mut s = super::super::raster_tests::owner(r, input);
    s.prepare_current_radiance_mode(r, size, Some(mode))
        .unwrap();
    s
}
fn poses() -> Vec<Pose> {
    let initial = Pose::initial();
    let mut p = vec![initial; 16];
    p.extend([
        Pose {
            mesh_y: 0.125,
            ..initial
        },
        Pose {
            mesh_y: 0.25,
            ..initial
        },
        Pose {
            mesh_y: 0.125,
            ..initial
        },
        initial,
        Pose {
            camera_y: 0.125,
            ..initial
        },
        Pose {
            camera_y: -0.125,
            ..initial
        },
        initial,
        Pose {
            occluder_y: 0.,
            ..initial
        },
        Pose {
            occluder_y: 0.,
            occluder_z: 2.,
            ..initial
        },
        Pose {
            occluder_y: 0.,
            occluder_z: 9.8,
            ..initial
        },
        initial,
        Pose {
            mesh_y: 6.,
            ..initial
        },
        initial,
    ]);
    p
}
#[test]
#[ignore = "coordinated owned-buffer current-PBR numerical test; no surface/window or OS capture"]
fn selective_current_radiance_fills_disocclusion_and_preserves_geometry() {
    let root =
        std::path::PathBuf::from(std::env::var_os("INCANT_VISIBILITY_CURRENT_OUTPUT").unwrap());
    std::fs::create_dir(&root).unwrap();
    let mut r = crate::test_support::checked(Renderer::headless().unwrap());
    pollster::block_on(r.enable_temporal_upscale()).unwrap();
    pollster::block_on(r.enable_motion_owner()).unwrap();
    let pattern = oracle::calibration(&Probe::new(&r).calibrate(&r)).unwrap();
    let reader = Reader::new(&r);
    let shaded_reader = ShadedReader::new(&r);
    let size = [64, 64];
    let texture = crate::hdr_output::texture(&r, wgpu::TextureFormat::Rgba8UnormSrgb, 64, 64);
    let view = texture.create_view(&Default::default());
    let mut input = scene();
    let initial = Pose::initial();
    apply(&mut input, initial);
    let mut candidate = current_owner(&r, &input, size, Mode::Missing);
    let mut reference = current_owner(&r, &input, size, Mode::AllCurrent);
    let mut previous = initial;
    let mut summaries = Vec::new();
    for (index, pose) in poses().into_iter().enumerate() {
        apply(&mut input, pose);
        submit(&r, &mut candidate, &input, &view, size);
        let raw = candidate.visibility_reconstruction_probe().read_fixture(&r);
        let pixels = reader.read(&r, &candidate);
        let shaded = shaded_reader.read(&r, &candidate);
        submit(&r, &mut reference, &input, &view, size);
        let expected = reader.read(&r, &reference);
        write(
            &root,
            &format!("frame-{index:02}.bin"),
            bytemuck::cast_slice(&pixels),
        );
        super::super::visibility_tests::save_samples(&root, &format!("frame-{index:02}"), &raw);
        write(
            &root,
            &format!("frame-{index:02}-current.bin"),
            bytemuck::cast_slice(&shaded),
        );
        write(
            &root,
            &format!("frame-{index:02}-reference.bin"),
            bytemuck::cast_slice(&expected),
        );
        let changes = candidate.motion_changes().unwrap();
        let frame = candidate.motion_frame().unwrap();
        let ids = [
            candidate.visibility_test_owner(0).unwrap(),
            candidate.visibility_test_owner(2).unwrap(),
        ];
        write(&root,&format!("frame-{index:02}.json"),&serde_json::to_vec_pretty(&json!({"frame":index,"pose":[pose.mesh_y,pose.camera_y,pose.occluder_y,pose.occluder_z],"previous_pose":[previous.mesh_y,previous.camera_y,previous.occluder_y,previous.occluder_z],"owners":ids,"changed":ids.map(|id|changes.changed(id).unwrap()),"sequence":frame.sequence(),"epoch":frame.epoch(),"has_previous":frame.has_previous(),"sample_positions":pattern})).unwrap());
        oracle::assert_inspection(&oracle::inspect(
            &raw,
            pattern,
            &Scene {
                current: pose,
                previous,
                words: ids.map(|id| oracle::word(id, changes.changed(id).unwrap())),
                has_previous: frame.has_previous(),
            },
        ));
        let filled = validate_shading(&raw, &shaded);
        let m = metrics(&pixels, &raw, ids);
        assert_eq!(m["sample_area_error"], 0.);
        assert_eq!(m["sample_RGB_error"], 0.);
        assert_eq!(
            pixels.iter().map(|p| p.color).collect::<Vec<_>>(),
            expected.iter().map(|p| p.color).collect::<Vec<_>>()
        );
        assert_eq!(frame.sequence(), index as u64 + 1);
        assert_eq!(frame.epoch(), 1);
        if [26, 28].contains(&index) {
            assert!(filled >= 48, "disocclusion must execute current shading");
        }
        summaries.push(json!({"frame":index,"pose":[pose.mesh_y,pose.camera_y,pose.occluder_y,pose.occluder_z],"filled_samples":filled,"metrics":m}));
        previous = pose;
    }
    controls(&r, &mut input, &view, &reader);
    let pbr = pbr_checks(&r, &root, &reader, &shaded_reader);
    let clusters = cluster_boundary_checks(&r, &root, &shaded_reader);
    write(&root,"result.json",&serde_json::to_vec_pretty(&json!({"frames":summaries,"PBR":pbr,"cluster_boundary":clusters,"bytes_per_output_pixel":236,"sample_positions":pattern,"refusal_cut_resize_removal":true,"ordinary_production_unchanged":true,"appearance_accepted":false,"performance_accepted":false})).unwrap());
}
fn controls(r: &Renderer, input: &mut SceneInput, view: &wgpu::TextureView, reader: &Reader) {
    let initial = Pose::initial();
    let size = [64, 64];
    apply(input, initial);
    let mut tested = current_owner(r, input, size, Mode::Missing);
    let mut untouched = current_owner(r, input, size, Mode::Missing);
    for _ in 0..8 {
        submit(r, &mut tested, input, view, size);
        submit(r, &mut untouched, input, view, size);
    }
    let before = reader.read(r, &tested);
    let old = tested.motion_frame().unwrap();
    let error = tested
        .submit(
            r,
            input,
            RetainedTarget {
                view,
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
    assert_eq!(reader.read(r, &tested), before);
    assert_eq!(tested.motion_frame().unwrap().sequence(), old.sequence());
    submit(r, &mut tested, input, view, size);
    submit(r, &mut untouched, input, view, size);
    assert_eq!(reader.read(r, &tested), reader.read(r, &untouched));
    tested.invalidate_motion_history();
    submit(r, &mut tested, input, view, size);
    let mut fresh = current_owner(r, input, size, Mode::Missing);
    submit(r, &mut fresh, input, view, size);
    assert_eq!(reader.read(r, &tested), reader.read(r, &fresh));
    for size in [[80, 64], [64, 64]] {
        let texture =
            crate::hdr_output::texture(r, wgpu::TextureFormat::Rgba8UnormSrgb, size[0], size[1]);
        let v = texture.create_view(&Default::default());
        tested
            .prepare_current_radiance_mode(r, size, Some(Mode::Missing))
            .unwrap();
        submit(r, &mut tested, input, &v, size);
        let mut fresh = current_owner(r, input, size, Mode::Missing);
        submit(r, &mut fresh, input, &v, size);
        assert_eq!(reader.read(r, &tested), reader.read(r, &fresh));
    }
    input.nodes[0].mesh = None;
    let mut removed = current_owner(r, input, size, Mode::Missing);
    submit(r, &mut removed, input, view, size);
    assert!(
        reader
            .read(r, &removed)
            .iter()
            .all(|p| p.color == [0.; 4] && p.owner == 0)
    );
}
fn pbr_checks(
    r: &Renderer,
    root: &std::path::Path,
    reader: &Reader,
    shaded_reader: &ShadedReader,
) -> serde_json::Value {
    let size = [64, 64];
    let texture = crate::hdr_output::texture(r, wgpu::TextureFormat::Rgba8UnormSrgb, 64, 64);
    let view = texture.create_view(&Default::default());
    let initial = Pose::initial();
    let mut compared = 0;
    let mut rows = Vec::new();
    for material in [false, true] {
        let mut input = pbr_fixture::scene_material(&[false], material);
        apply(&mut input, initial);
        let mut candidate = current_owner(r, &input, size, Mode::Missing);
        let mut reference = current_owner(r, &input, size, Mode::AllCurrent);
        for (i, pose) in [
            initial,
            Pose {
                occluder_y: 0.,
                ..initial
            },
            initial,
            Pose {
                mesh_y: 6.,
                ..initial
            },
            initial,
        ]
        .into_iter()
        .enumerate()
        {
            let key = format!("pbr-{}-{i}", u8::from(material));
            apply(&mut input, pose);
            // A different current light at the disocclusion must affect fresh PBR.
            input.nodes[4].world =
                glam::DMat4::from_translation(glam::DVec3::new(i as f64 * 0.1, 0., 3.));
            submit(r, &mut candidate, &input, &view, size);
            let raw = candidate.visibility_reconstruction_probe().read_fixture(r);
            let actual = shaded_reader.read(r, &candidate);
            submit(r, &mut reference, &input, &view, size);
            let expected = shaded_reader.read(r, &reference);
            write(
                root,
                &format!("{key}-current.bin"),
                bytemuck::cast_slice(&actual),
            );
            write(
                root,
                &format!("{key}-reference.bin"),
                bytemuck::cast_slice(&expected),
            );
            super::super::visibility_tests::save_samples(root, &key, &raw);
            write(root,&format!("{key}.json"),&serde_json::to_vec_pretty(&json!({"frame":i,"material_variant":material,"pose":[pose.mesh_y,pose.camera_y,pose.occluder_y,pose.occluder_z],"point_translation":[i as f64*0.1,0.,3.],"reference":"unconditional current PBR; all local lights"})).unwrap());
            let filled = validate_shading(&raw, &actual);
            validate_shading(&raw, &expected);
            for (lane, (a, b)) in actual.iter().zip(&expected).enumerate() {
                if a.mask & (1 << (lane % 4)) != 0 {
                    assert_eq!(a.color, b.color, "current PBR mismatch at {i}/{lane}");
                    assert_eq!(a.depth, b.depth);
                    compared += 1;
                }
            }
            let pixels = reader.read(r, &candidate);
            let expected_pixels = reader.read(r, &reference);
            let difference: f64 = pixels
                .iter()
                .zip(&expected_pixels)
                .flat_map(|(a, b)| a.color.into_iter().zip(b.color))
                .map(|(a, b)| f64::from((a - b).abs()))
                .sum();
            rows.push(json!({"frame":i,"material_variant":material,"filled_samples":filled,"whole_output_L1_not_an_acceptance_threshold":difference}));
        }
    }
    assert!(
        compared >= 48,
        "textured disocclusion must shade actual missing samples"
    );
    // Same owner, same-depth triangles with different UVs: first winner must
    // equal the single first layer, not the last. Independent scene construction.
    let mut rendered = Vec::new();
    for (i, layers) in [
        &[false][..],
        &[true][..],
        &[false, true][..],
        &[true, false][..],
    ]
    .into_iter()
    .enumerate()
    {
        let mut input = pbr_fixture::scene(layers);
        apply(&mut input, initial);
        let mut scene = current_owner(r, &input, size, Mode::AllCurrent);
        submit(r, &mut scene, &input, &view, size);
        let raw = scene.visibility_reconstruction_probe().read_fixture(r);
        let samples = shaded_reader.read(r, &scene);
        validate_shading(&raw, &samples);
        write(
            root,
            &format!("coplanar-{i}.bin"),
            bytemuck::cast_slice(&samples),
        );
        super::super::visibility_tests::save_samples(root, &format!("coplanar-{i}"), &raw);
        rendered.push(samples.iter().map(|s| s.color).collect::<Vec<_>>());
    }
    assert_ne!(rendered[0], rendered[1], "UV reversal must be observable");
    assert_eq!(rendered[0], rendered[2]);
    assert_eq!(rendered[1], rendered[3]);
    json!({"frames":rows,"exact_compared_missing_samples":compared,"same_owner_coplanar_first_winner":true,"different_material_footprint_not_equated_to_centroid":true})
}

// Output tiles split at x=64; the reduced 96x48 rectangle has a different
// world-space tile boundary. Short-range lights make stale low-grid selection
// observable instead of allowing every cluster to contain the same light.
fn cluster_boundary_checks(
    r: &Renderer,
    root: &std::path::Path,
    reader: &ShadedReader,
) -> serde_json::Value {
    let size = [128, 64];
    let texture =
        crate::hdr_output::texture(r, wgpu::TextureFormat::Rgba8UnormSrgb, size[0], size[1]);
    let view = texture.create_view(&Default::default());
    let mut rows = Vec::new();
    for (index, x) in [-1.0, -0.125, 0.125, 1.0].into_iter().enumerate() {
        let mut input = pbr_fixture::scene(&[false]);
        input.environment.as_mut().unwrap().intensity = 0.;
        if let Some(incant_render_data::SceneLight::Directional(light)) = &mut input.nodes[3].light
        {
            light.intensity = 0.;
        }
        input.nodes[4].world = glam::DMat4::from_translation(glam::DVec3::new(x, -1.59, 0.25));
        if let Some(incant_render_data::SceneLight::Point(light)) = &mut input.nodes[4].light {
            light.range = 0.6;
        }
        apply(
            &mut input,
            Pose {
                mesh_y: 6.,
                ..Pose::initial()
            },
        );
        let mut candidate = current_owner(r, &input, size, Mode::Missing);
        for _ in 0..2 {
            submit(r, &mut candidate, &input, &view, size);
        }
        apply(&mut input, Pose::initial());
        submit(r, &mut candidate, &input, &view, size);
        let actual = reader.read(r, &candidate);
        let mut reference = current_owner(r, &input, size, Mode::AllCurrent);
        submit(r, &mut reference, &input, &view, size);
        let expected = reader.read(r, &reference);
        if let Some(incant_render_data::SceneLight::Point(light)) = &mut input.nodes[4].light {
            light.intensity = 0.;
        }
        submit(r, &mut reference, &input, &view, size);
        let dark = reader.read(r, &reference);
        for (name, samples) in [
            ("current", &actual),
            ("reference", &expected),
            ("light-off", &dark),
        ] {
            write(
                root,
                &format!("cluster-{index}-{name}.bin"),
                bytemuck::cast_slice(samples),
            );
        }
        write(root,&format!("cluster-{index}.json"),&serde_json::to_vec_pretty(&json!({"output":size,"input":[96,48],"light":[x,-1.59,0.25],"range":0.6,"output_tile_boundary":64,"sequence":3,"jitter_phase":2})).unwrap());
        validate_current(&actual);
        validate_current(&expected);
        validate_current(&dark);
        let mut compared = [0; 2];
        let mut lit = [0; 2];
        for (i, ((a, b), d)) in actual.iter().zip(&expected).zip(&dark).enumerate() {
            if a.mask & (1 << (i % 4)) == 0 {
                continue;
            }
            let tile = (i / 4 % 128) / 64;
            assert_eq!(a.color, b.color, "cluster mismatch at {index}/{i}");
            assert_eq!(a.visible_depth, b.visible_depth);
            compared[tile] += 1;
            if a.color[..3].iter().zip(&d.color).any(|(a, d)| a > d) {
                lit[tile] += 1;
            }
        }
        assert!(
            compared.iter().all(|n| *n > 0),
            "both output clusters require shaded missing samples"
        );
        let side = usize::from(x > 0.);
        assert!(
            lit[side] > 0,
            "short-range light must affect a requested sample on its side"
        );
        rows.push(
            json!({"light_x":x,"compared_samples_per_tile":compared,"lit_samples_per_tile":lit}),
        );
    }
    json!(rows)
}
