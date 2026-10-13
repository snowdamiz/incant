//! Retained text on the GPU: motion, rebuild and changed-text identity reuse,
//! refusal atomicity, resize and depth. Numeric agreement, not appearance.
use super::*;
use crate::{
    hdr_output::{read, texture},
    retained_alloc as allocation,
    text::{
        TextScene,
        draw_tests::{camera, pixel_camera, scene},
        tests::{input, reset_input},
    },
};
use glam::{DMat4, DQuat, DVec3};
use incant_types::CameraProjection;

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
fn perspective() -> incant_render_data::SceneNode {
    camera(
        CameraProjection::Perspective {},
        DMat4::look_at_rh(DVec3::new(0.4, 0.3, 4.), DVec3::new(0.2, 0., 0.), DVec3::Y).inverse(),
    )
}
fn fresh(renderer: &Renderer, input: &SceneInput, size: [u32; 2]) -> Vec<u8> {
    let scene = renderer
        .prepare(input)
        .unwrap()
        .with_camera("camera")
        .unwrap();
    let output = texture(renderer, FORMAT, size[0], size[1]);
    renderer.queue.submit([renderer
        .draw_scene(
            &scene,
            &output.create_view(&Default::default()),
            FORMAT,
            size[0],
            size[1],
            None,
        )
        .unwrap()]);
    read(renderer, &output)
}
fn submit(
    renderer: &Renderer,
    retained: &mut RetainedScene,
    input: &SceneInput,
    size: [u32; 2],
) -> Vec<u8> {
    let output = texture(renderer, FORMAT, size[0], size[1]);
    let view = output.create_view(&Default::default());
    retained
        .submit(
            renderer,
            input,
            RetainedTarget {
                view: &view,
                format: FORMAT,
                width: size[0],
                height: size[1],
                viewport: None,
            },
        )
        .unwrap();
    read(renderer, &output)
}
fn input_text(world: DMat4) -> incant_render_data::TextInput {
    input(&[(1, "ffi Hello", 64., world)])
}
fn text_scene(retained: &RetainedScene) -> (&TextScene, &TextScene) {
    (
        retained.live.text.as_deref().unwrap(),
        retained.staging.text.as_deref().unwrap(),
    )
}

#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn retained_motion_rebuild_and_changed_text_reuse_or_replace_exact_identities() {
    let renderer = crate::test_support::checked(Renderer::headless().unwrap());
    let size = [192, 96];
    let worlds = [
        DMat4::from_translation(DVec3::new(-1.4, 0.6, 0.)),
        DMat4::from_translation(DVec3::new(-0.6, -0.1, -0.5)),
    ];
    let mut input = scene(
        input(&[
            (1, "ffi Hello", 24., worlds[0]),
            (2, "سلام 中文", 32., worlds[1]),
        ]),
        perspective(),
    );
    let before = renderer.text_uploads();
    let mut retained = renderer.prepare_retained(&input, "camera").unwrap();
    let uploads = renderer.text_uploads();
    assert_eq!(uploads.atlas_textures, before.atlas_textures + 1);
    assert_eq!(uploads.glyph_buffers, before.glyph_buffers + 1);
    let (live, staging) = text_scene(&retained);
    assert!(live.shares_geometry(staging));
    assert_eq!(retained.stats().text_entities, 2);
    // Accepted plus staged transforms and one shared atlas prefix/quad buffer.
    assert_eq!(
        retained.stats().text_gpu_bytes,
        live.gpu_bytes() + staging.transform_bytes()
    );
    let initial = submit(&renderer, &mut retained, &input, size);
    assert_eq!(initial, fresh(&renderer, &input, size));
    let rows = |r: &RetainedScene| {
        let (live, staging) = text_scene(r);
        let mut rows = [live.update_capacity(), staging.update_capacity()];
        rows.sort();
        rows
    };
    let reserved = rows(&retained);
    let mut changed = false;
    for frame in 1..=8 {
        let t = frame as f64 * 0.15;
        let text = input.text.as_mut().unwrap();
        text.nodes[0].world = worlds[0]
            * DMat4::from_scale_rotation_translation(
                DVec3::new(1. + t * 0.2, 1., 1.),
                DQuat::from_rotation_z(t * 0.3),
                DVec3::new(t * 0.4, -t * 0.1, 0.),
            );
        text.nodes[1].world = worlds[1] * DMat4::from_rotation_y(t * 0.8);
        input.nodes[0].world = DMat4::look_at_rh(
            DVec3::new(0.4 + t * 0.1, 0.3, 4. - t * 0.2),
            DVec3::new(0.2, 0., 0.),
            DVec3::Y,
        )
        .inverse();
        assert_eq!(allocation::measured(|| retained.stage(&input).unwrap()), 0);
        let pixels = submit(&renderer, &mut retained, &input, size);
        changed |= pixels != initial;
        assert_eq!(
            pixels,
            fresh(&renderer, &input, size),
            "moving frame {frame}"
        );
        assert_eq!(
            renderer.text_uploads(),
            uploads,
            "motion uploaded text at {frame}"
        );
    }
    assert!(changed);
    assert_eq!(rows(&retained), reserved);
    let (live, staging) = text_scene(&retained);
    assert_eq!(
        retained.stats().text_gpu_bytes,
        live.gpu_bytes() + staging.transform_bytes(),
        "accounting survives live/staging swaps"
    );
    // A rebuild with unchanged text identity (e.g. spatial topology) shares
    // the accepted geometry and uploads nothing.
    let rebuilt = renderer.prepare_retained(&input, "camera").unwrap();
    assert!(
        text_scene(&rebuilt)
            .0
            .shares_geometry(text_scene(&retained).0)
    );
    assert_eq!(renderer.text_uploads(), uploads);
    // Equal content in a new layout Arc is a different identity: a retained
    // frame refuses it without publishing, and the accepted frame survives.
    let accepted = renderer
        .screenshot_scene_png(&retained.live, 160, 96)
        .unwrap();
    let mut other = SceneInput {
        nodes: vec![perspective()],
        text: Some(crate::text::tests::input(&[
            (1, "ffi Hello", 24., worlds[0]),
            (2, "سلام 中文", 32., worlds[1]),
        ])),
        environment: None,
    };
    other.nodes[0].world = input.nodes[0].world;
    assert!(retained.stage(&other).is_err());
    let mut swapped = input.text.clone().unwrap();
    swapped.nodes[0].layout = Arc::clone(&other.text.as_ref().unwrap().nodes[0].layout);
    let swapped = SceneInput {
        nodes: vec![perspective()],
        text: Some(swapped),
        environment: None,
    };
    assert!(retained.stage(&swapped).is_err());
    let mut without = SceneInput {
        nodes: vec![perspective()],
        text: None,
        environment: None,
    };
    without.nodes[0].world = input.nodes[0].world;
    assert!(retained.stage(&without).is_err());
    let output = texture(&renderer, FORMAT, size[0], size[1]);
    let view = output.create_view(&Default::default());
    let invalid = RetainedTarget {
        view: &view,
        format: FORMAT,
        width: 0,
        height: size[1],
        viewport: None,
    };
    assert!(retained.submit(&renderer, &input, invalid).is_err());
    assert_eq!(
        renderer
            .screenshot_scene_png(&retained.live, 160, 96)
            .unwrap(),
        accepted
    );
    assert_eq!(
        submit(&renderer, &mut retained, &input, size),
        fresh(&renderer, &input, size)
    );
    // Changed text: one new atlas prefix and one new quad buffer, once.
    let replacement = renderer.prepare_retained(&other, "camera").unwrap();
    let after = renderer.text_uploads();
    assert_eq!(after.atlas_textures, uploads.atlas_textures + 1);
    assert_eq!(after.glyph_buffers, uploads.glyph_buffers + 1);
    assert!(
        !text_scene(&replacement)
            .0
            .shares_geometry(text_scene(&retained).0)
    );
    drop(rebuilt);
    // Same atlas, different node set: the atlas texture is shared by identity
    // while the glyph quads are rebuilt once for the new layout set.
    let mut subset = input.text.clone().unwrap();
    subset.nodes.truncate(1);
    let subset = SceneInput {
        nodes: vec![perspective()],
        text: Some(subset),
        environment: None,
    };
    let partial = renderer.prepare_retained(&subset, "camera").unwrap();
    let now = renderer.text_uploads();
    assert_eq!(now.atlas_textures, after.atlas_textures);
    assert_eq!(now.glyph_buffers, after.glyph_buffers + 1);
    assert!(
        !text_scene(&partial)
            .0
            .shares_geometry(text_scene(&retained).0)
    );
    // Resize: the same retained text at other target sizes and aspects.
    for size in [[96, 160], [320, 72], [192, 96]] {
        assert_eq!(
            submit(&renderer, &mut retained, &input, size),
            fresh(&renderer, &input, size)
        );
    }
}

#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn foreign_or_reset_handles_refuse_preparation_before_any_text_upload() {
    let renderer = crate::test_support::checked(Renderer::headless().unwrap());
    let a = input(&[(1, "Hello", 20., DMat4::IDENTITY)]);
    let b = input(&[(1, "Hello", 20., DMat4::IDENTITY)]);
    let mut foreign = a.clone();
    foreign.nodes[0].glyphs = Arc::clone(&b.nodes[0].glyphs);
    let before = renderer.text_uploads();
    for text in [foreign, reset_input()] {
        let input = scene(text, pixel_camera([64, 64]));
        assert!(renderer.prepare_retained(&input, "camera").is_err());
        assert!(renderer.prepare(&input).is_err());
    }
    assert_eq!(renderer.text_uploads(), before);
    // A valid identical-content scene still prepares, uploading once.
    renderer
        .prepare_retained(&scene(a, pixel_camera([64, 64])), "camera")
        .unwrap();
    assert_eq!(
        renderer.text_uploads().atlas_textures,
        before.atlas_textures + 1
    );
}

#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn opaque_geometry_occludes_text_behind_it_and_text_in_front_draws_over_it() {
    let renderer = crate::test_support::checked(Renderer::headless().unwrap());
    let directory = tempfile::tempdir().unwrap();
    let (project, assets, _) = crate::test_support::fixture(directory.path());
    let size = [160, 120];
    let with = |z: Option<f64>, yaw: f64| {
        let mut input = incant_render_authoring::project_input(&project, Some(&assets)).unwrap();
        input.nodes.push(camera(
            CameraProjection::Perspective {},
            DMat4::look_at_rh(DVec3::new(1., 2., 6.), DVec3::new(1., 2., 0.), DVec3::Y).inverse(),
        ));
        input.text = z.map(|z| {
            input_text(
                DMat4::from_translation(DVec3::new(-0.6, 2.4, z)) * DMat4::from_rotation_y(yaw),
            )
        });
        input
    };
    let plain = fresh(&renderer, &with(None, 0.), size);
    let behind = fresh(&renderer, &with(Some(-0.5), 0.), size);
    let front = fresh(&renderer, &with(Some(0.5), 0.), size);
    let reversed = fresh(&renderer, &with(Some(0.5), std::f64::consts::PI), size);
    let backdrop = |p: &[u8]| p[..3] == [20, 21, 25];
    let (mut visible, mut over, mut model) = (0, 0, 0);
    for ((a, b), c) in plain
        .as_chunks::<4>()
        .0
        .iter()
        .zip(behind.as_chunks::<4>().0)
        .zip(front.as_chunks::<4>().0)
    {
        if backdrop(a) {
            visible += usize::from(b != a);
        } else {
            model += 1;
            assert_eq!(a, b, "text behind opaque geometry must be hidden");
            over += usize::from(c != a);
        }
    }
    assert!(model > 200, "{model}");
    assert!(visible > 50, "{visible}");
    assert!(over > 50, "{over}");
    assert_ne!(reversed, plain, "two-sided text stays visible from behind");
}
