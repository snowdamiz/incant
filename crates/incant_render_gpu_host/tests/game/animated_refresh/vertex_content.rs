//! Numerical same-clock references for compatible vertex refresh. These are
//! offscreen correctness controls, not appearance approval or timing evidence.
use super::*;
#[path = "vertex_fixture.rs"]
mod vertex_fixture;
use vertex_fixture::Kind;

fn counter(game: &AnimationGame) -> f64 {
    game.world()
        .unwrap()
        .inspect(id(3))
        .unwrap()
        .transform
        .translation[0]
}
fn retain(output: &Path, name: &str, bytes: &[u8]) {
    super::super::model_refresh::retain_pixels(output, name, bytes);
    let file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output.join(format!("{name}.png")))
        .unwrap();
    let mut png = png::Encoder::new(file, 160, 96);
    png.set_color(png::ColorType::Rgba);
    png.set_depth(png::BitDepth::Eight);
    png.write_header().unwrap().write_image_data(bytes).unwrap();
}

#[test]
#[ignore = "requires a native GPU; run explicitly after resource grant"]
fn compatible_geometry_matches_fresh_pixels_at_same_clock_and_retains_refusals() {
    exercise(Kind::Geometry);
}

#[test]
#[ignore = "requires a native GPU; run explicitly after resource grant"]
fn generated_tangents_match_fresh_pixels_with_identical_material_and_image() {
    exercise(Kind::Tangents);
}

fn exercise(kind: Kind) {
    let temporary = tempfile::tempdir().unwrap();
    let output = std::env::var_os("INCANT_ANIMATED_VERTEX_PIXELS")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| temporary.path().join("pixels"));
    assert!(output.is_absolute());
    fs::create_dir(&output).unwrap();
    let root = tempfile::tempdir().unwrap();
    let original = kind.model(root.path(), false);
    let next = kind.model(root.path(), true);
    kind.check(root.path(), original, next);
    let invalid = kind.invalid(root.path());
    let epoch = Arc::new(AtomicUsize::new(1));
    let (mut live, probe) = game_with_light(root.path(), original, epoch.clone(), kind.lit());
    let old_asset = source(&live).clone();
    let identity = live
        .world()
        .unwrap()
        .spatial_view()
        .unwrap()
        .identity()
        .clone();
    live.tick(&[]).unwrap();
    let state = live.world().unwrap().animation_state(id(1)).unwrap();
    assert!((state.sample.seconds - 0.5).abs() < 1e-12);
    assert_eq!(counter(&live), 1.);
    assert!(
        !live
            .host()
            .unwrap()
            .consumer()
            .motion_frame()
            .unwrap()
            .has_previous()
    );
    let initial = pixels(&probe);
    retain(&output, "initial", &initial);
    live.refresh_host().unwrap();
    let old_motion = live.host().unwrap().consumer().motion_frame().unwrap();
    assert!(old_motion.has_previous());
    assert_eq!(pixels(&probe), initial);

    // Independent cold startup with the changed asset, never staged through the
    // refresh implementation. Both scripts have accepted exactly one tick.
    let (mut fresh, fresh_probe) =
        game_with_light(root.path(), next, Arc::new(AtomicUsize::new(1)), kind.lit());
    let adapter = format!("{:?}", probe.renderer.adapter.get_info());
    assert_eq!(
        format!("{:?}", fresh_probe.renderer.adapter.get_info()),
        adapter
    );
    fresh.tick(&[]).unwrap();
    assert_eq!(fresh.world().unwrap().animation_state(id(1)), Some(state));
    assert_eq!(fresh.world().unwrap().tick(), 1);
    assert_eq!(counter(&fresh), 1.);
    let expected = pixels(&fresh_probe);
    retain(&output, "fresh_reference", &expected);
    fresh.stop();
    assert_eq!(fresh_probe.dropped.load(Ordering::SeqCst), 1);
    drop(fresh);
    drop(fresh_probe);

    if let Some(invalid) = invalid {
        let a = vertex_fixture::loaded(root.path(), original);
        let b = vertex_fixture::loaded(root.path(), invalid);
        assert_eq!(a.meshes[0].vertices, b.meshes[0].vertices);
        assert_eq!(a.animation, b.animation);
        assert_ne!(a.meshes[0].indices, b.meshes[0].indices);
        assert!(matches!(
            live.host()
                .unwrap()
                .texture_refresh_ticket()
                .prepare_visuals(
                    &[invalid],
                    VisualCaches {
                        models: &root.path().join(".incant/cache/models"),
                        textures: &root.path().join(".incant/cache/textures"),
                    },
                ),
            Err(incant_render_native::NativeRenderError::TextureRefresh(
                "animated model topology, rest or skin bindings changed"
            ))
        ));
        assert!(Arc::ptr_eq(source(&live), &old_asset));
        assert_eq!(probe.presents.load(Ordering::SeqCst), 2);
        live.refresh_host().unwrap();
        let bytes = pixels(&probe);
        retain(&output, "invalid_topology_retained", &bytes);
        assert_eq!(bytes, initial);
    }
    let mut refused = TextureRefreshOperation::Publish(stage(&mut live, next, root.path()));
    let presents = probe.presents.load(Ordering::SeqCst);
    epoch.store(2, Ordering::SeqCst);
    let mut ready = live.prepare_host_update().unwrap();
    assert!(matches!(
        ready.update(&mut refused).unwrap(),
        TextureRefreshReply::Refused(_)
    ));
    drop(ready);
    drop(refused);
    assert_eq!(probe.presents.load(Ordering::SeqCst), presents);
    assert_eq!(live.host().unwrap().render_input().asset_generation(), 0);
    assert!(Arc::ptr_eq(source(&live), &old_asset));
    epoch.store(1, Ordering::SeqCst);
    live.refresh_host().unwrap();
    let refused_pixels = pixels(&probe);
    retain(&output, "late_refusal_retained", &refused_pixels);
    assert_eq!(refused_pixels, initial);

    let mut publication = TextureRefreshOperation::Publish(stage(&mut live, next, root.path()));
    let deleted_root = root.path().to_path_buf();
    root.close().unwrap();
    assert!(
        !deleted_root.exists(),
        "source and all cooked cache files removed before publication"
    );
    let presents = probe.presents.load(Ordering::SeqCst);
    let frames = live.host().unwrap().consumer().accepted_frames();
    let mut ready = live.prepare_host_update().unwrap();
    let receipt = match ready.update(&mut publication).unwrap() {
        TextureRefreshReply::Published(receipt) => receipt,
        _ => panic!("compatible vertex publication must succeed"),
    };
    assert_eq!((receipt.changed, receipt.generation), (1, 1));
    drop(ready);
    assert_eq!(probe.presents.load(Ordering::SeqCst), presents);
    assert_eq!(live.host().unwrap().consumer().accepted_frames(), frames);
    assert_eq!(live.world().unwrap().tick(), 1);
    assert_eq!(live.world().unwrap().animation_state(id(1)), Some(state));
    assert_eq!(
        live.world().unwrap().spatial_view().unwrap().identity(),
        &identity
    );
    assert_eq!(counter(&live), 1.);
    assert_eq!(probe.dropped.load(Ordering::SeqCst), 0);
    assert_eq!(source(&live).info().fingerprint, key(next));
    assert_ne!(source(&live).version_id(), old_asset.version_id());
    assert_eq!(old_asset.info().fingerprint, key(original));
    live.refresh_host().unwrap();
    let replacement = pixels(&probe);
    retain(&output, "replacement", &replacement);
    assert_eq!(
        replacement, expected,
        "full RGBA frame equals independent fresh startup at the same clock"
    );
    assert_ne!(initial, replacement);
    let motion = live.host().unwrap().consumer().motion_frame().unwrap();
    assert!(!motion.has_previous());
    assert!(!motion.same_owner(&old_motion));
    let pixel_oracle = match kind {
        Kind::Geometry => {
            let before = red_mask(&initial);
            let after = red_mask(&replacement);
            let old_count = before.iter().filter(|&&p| p).count();
            let new_count = after.iter().filter(|&&p| p).count();
            let removed = before
                .iter()
                .zip(&after)
                .filter(|(a, b)| **a && !**b)
                .count();
            let added = before
                .iter()
                .zip(&after)
                .filter(|(a, b)| !**a && **b)
                .count();
            // Moving the lower-left source vertex +0.25X moves its projected
            // position six pixels inside the old triangle. Other positions,
            // normal, material and image remain identical.
            assert!(new_count > 0 && new_count < old_count);
            assert!(removed > 0);
            assert_eq!(added, 0);
            json!({"kind":"geometry_silhouette", "old_red":old_count,"new_red":new_count,
                "removed":removed,"added":added,"vertex_delta_x":0.25})
        }
        Kind::Tangents => {
            // Interior of the unchanged projected triangle, away from all
            // edges. Opposite bitangents under the same -Y/+Z light must affect
            // actual shading; a material color change cannot satisfy this case.
            let mut changed = 0;
            for y in 48..54 {
                for x in 72..78 {
                    let i = (y * 160 + x) * 4;
                    if (0..3).any(|c| initial[i + c].abs_diff(replacement[i + c]) > 8) {
                        changed += 1;
                    }
                }
            }
            assert_eq!(
                changed, 36,
                "every selected interior pixel responds to tangent basis"
            );
            json!({"kind":"tangent_only_shading","interior_changed":changed,
                "material_image_position_normal_uv_identical":true,"old_w":1,"new_w":-1})
        }
    };
    let mut ready = live.prepare_host_update().unwrap();
    assert!(matches!(
        ready.update(&mut publication).unwrap(),
        TextureRefreshReply::Refused(_)
    ));
    drop(ready);
    drop(publication);
    live.refresh_host().unwrap();
    let stale_pixels = pixels(&probe);
    retain(&output, "stale_candidate_retained", &stale_pixels);
    assert_eq!(stale_pixels, replacement);
    assert!(
        live.host()
            .unwrap()
            .consumer()
            .motion_frame()
            .unwrap()
            .has_previous()
    );
    assert_eq!(live.world().unwrap().tick(), 1);
    assert_eq!(live.world().unwrap().animation_state(id(1)), Some(state));
    live.tick(&[]).unwrap();
    let continued_clock = live
        .world()
        .unwrap()
        .animation_state(id(1))
        .unwrap()
        .sample
        .seconds;
    assert!((continued_clock - (0.5 + 1. / 60.)).abs() < 1e-12);
    assert_eq!(counter(&live), 3.);
    assert_eq!(live.world().unwrap().tick(), 2);
    assert_eq!(
        live.world().unwrap().spatial_view().unwrap().identity(),
        &identity
    );
    assert!(!deleted_root.exists());
    retain(&output, "continued", &pixels(&probe));
    let frames = live.host().unwrap().consumer().accepted_frames();
    let presents = probe.presents.load(Ordering::SeqCst);
    assert_eq!(frames as usize, presents);
    live.stop();
    assert_eq!(probe.dropped.load(Ordering::SeqCst), 1);
    use std::io::Write;
    fs::OpenOptions::new().write(true).create_new(true).open(output.join("proof.json")).unwrap()
        .write_all(&serde_json::to_vec_pretty(&json!({
            "scope":"offscreen numerical correctness only; no window, appearance or performance claim",
            "width":160,"height":96,"adapter":adapter,
            "original_fingerprint":key(original),"replacement_fingerprint":key(next),
            "pixel_oracle":pixel_oracle,"full_frame_matches_fresh":true,
            "same_clock_seconds":state.sample.seconds,"continued_clock_seconds":continued_clock,
            "source_and_cache_deleted_before_publication":true,"world_identity_preserved":true,
            "fresh_motion_owner":!motion.same_owner(&old_motion),"first_replacement_has_previous":motion.has_previous(),
            "invalid_topology_pixels_equal":invalid.map(|_| true),"late_refusal_pixels_equal":refused_pixels==initial,
            "stale_candidate_pixels_equal":stale_pixels==replacement,"tick":2,"vm_counter_before":1,"vm_counter_after":3,
            "catalog_generation":1,"frames":frames,"presents":presents,"target_drops":1
        })).unwrap()).unwrap();
    eprintln!(
        "animated-vertex refresh: same-clock fresh RGBA match; {pixel_oracle}; tick=2 VM=3 frames={frames}"
    );
}
