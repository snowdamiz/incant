//! Numerical existing-model material publication. No native-window/appearance gate.
use super::*;
use crate::animated_refresh::fixture::images as fixture;
#[path = "../../../../incant_render_native/tests/common/material_refresh.rs"]
#[allow(dead_code)] // Shared writer also exposes reorder/default CPU-only cases.
mod material_fixture;
use incant_assets_runtime::{AlphaMode, RuntimeAssetData};
use material_fixture::Tables;

fn material_model(root: &Path, tables: Tables) -> VisualAssetRef {
    material_fixture::material_model(root, tables);
    for (name, color) in [
        ("other.png", [0, 0, 255, 255]),
        ("added.png", [0, 255, 0, 255]),
    ] {
        let mut encoder = png::Encoder::new(fs::File::create(root.join(name)).unwrap(), 1, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&color)
            .unwrap();
    }
    let path = root.join("tri.gltf");
    let mut source: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    for material in source["materials"].as_array_mut().unwrap() {
        let index = material["pbrMetallicRoughness"]["baseColorTexture"]["index"].clone();
        material["pbrMetallicRoughness"]["baseColorFactor"] = json!([1, 1, 1, 1]);
        material["pbrMetallicRoughness"]["metallicFactor"] = json!(0);
        material["emissiveTexture"] = json!({"index":index});
        material["emissiveFactor"] = json!([1, 1, 1]);
        material["doubleSided"] = json!(true);
        if material["alphaMode"] != "MASK" {
            material["alphaMode"] = json!("OPAQUE");
        }
    }
    fs::write(path, serde_json::to_vec(&source).unwrap()).unwrap();
    material_fixture::recook(root)
}

#[test]
fn imported_material_gpu_fixture_has_identical_vertices_and_explicit_primary_role_data() {
    let root = tempfile::tempdir().unwrap();
    let references = [Tables::Initial, Tables::Rebound, Tables::Expanded]
        .map(|tables| material_model(root.path(), tables));
    let models = references.map(|r| {
        incant_assets_runtime::load_model(
            &root.path().join(".incant/cache/models"),
            &fixture::key(r),
        )
        .unwrap()
    });
    assert_eq!(models[0].metadata.mesh_materials, [Some(0)]);
    assert_eq!(models[1].metadata.mesh_materials, [Some(1)]);
    assert_eq!(models[2].metadata.mesh_materials, [Some(2)]);
    assert_eq!(models[0].meshes, models[1].meshes);
    assert_eq!(models[0].meshes, models[2].meshes);
    for (index, model) in models.iter().enumerate() {
        let material = &model.materials[index];
        let image = &model.images[material.maps[4].unwrap().image];
        let pixel = &image.levels[0][..4];
        assert_eq!(
            pixel,
            [[255, 0, 0, 255], [0, 0, 255, 255], [0, 255, 0, 255]][index]
        );
    }
    assert_eq!(models[2].materials.len(), 3);
    assert_eq!(
        models[2].materials[2].maps.map(|m| m.unwrap().texture),
        [2, 1, 0, 1, 2]
    );
    assert_eq!(models[2].materials[2].alpha_mode, AlphaMode::Mask);
}

fn publish(
    game: &mut ModelGame,
    probe: &Probe,
    lock: &Mutex<()>,
    operation: &mut TextureRefreshOperation<GpuRenderConsumer<ModelTargets>>,
) {
    let tick = game.world().unwrap().tick();
    let frames = game.host().unwrap().consumer().accepted_frames();
    let presents = probe.presents.load(Ordering::SeqCst);
    let mut ready = game.prepare_host_update().unwrap();
    let guard = lock.lock().unwrap();
    assert!(matches!(
        ready.update(operation).unwrap(),
        TextureRefreshReply::Published(_)
    ));
    assert_eq!(probe.presents.load(Ordering::SeqCst), presents);
    assert_eq!(probe.dropped.load(Ordering::SeqCst), 0);
    drop(guard);
    drop(ready);
    assert_eq!(game.world().unwrap().tick(), tick);
    assert_eq!(game.host().unwrap().consumer().accepted_frames(), frames);
}
fn mask(pixels: &[u8], channel: usize) -> Vec<bool> {
    pixels
        .as_chunks::<4>()
        .0
        .iter()
        .map(|pixel| {
            pixel[channel] > 150
                && (0..3)
                    .filter(|&i| i != channel)
                    .all(|i| i16::from(pixel[channel]) - i16::from(pixel[i]) > 128)
                && pixel[3] > 200
        })
        .collect()
}

#[test]
#[ignore = "requires a native GPU; run explicitly after resource grant"]
fn material_reassignment_and_table_replacement_reset_history_and_retain_refused_pixels() {
    let output_temporary = tempfile::tempdir().unwrap();
    let output = std::env::var_os("INCANT_MATERIAL_REFRESH_PIXELS")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| output_temporary.path().join("pixels"));
    assert!(output.is_absolute());
    fs::create_dir(&output).unwrap();
    let root = tempfile::tempdir().unwrap();
    let first = material_model(root.path(), Tables::Initial);
    let rebound = material_model(root.path(), Tables::Rebound);
    let expanded = material_model(root.path(), Tables::Expanded);
    let policy = Arc::new(AtomicBool::new(false));
    let (mut game, probe) = fixture_with_motion(root.path(), first, policy.clone(), true);
    let lock = Mutex::new(());
    drop(lock.lock().unwrap());
    let identity = game
        .world()
        .unwrap()
        .spatial_view()
        .unwrap()
        .identity()
        .clone();
    let old = source(&game).clone();
    game.tick(&[]).unwrap();
    let initial = pixels(&probe);
    retain_pixels(&output, "initial", &initial);
    let first_motion = game.host().unwrap().consumer().motion_frame().unwrap();
    assert!(!first_motion.has_previous());
    game.refresh_host().unwrap();
    let accepted_motion = game.host().unwrap().consumer().motion_frame().unwrap();
    assert!(accepted_motion.has_previous() && accepted_motion.same_owner(&first_motion));
    assert_eq!(pixels(&probe), initial);
    let mut rejected = TextureRefreshOperation::Publish(stage(&mut game, rebound, root.path()));
    policy.store(true, Ordering::SeqCst);
    let mut ready = game.prepare_host_update().unwrap();
    let guard = lock.lock().unwrap();
    assert!(matches!(
        ready.update(&mut rejected).unwrap(),
        TextureRefreshReply::Refused(_)
    ));
    assert_eq!(probe.presents.load(Ordering::SeqCst), 2);
    drop(guard);
    drop(ready);
    drop(rejected);
    policy.store(false, Ordering::SeqCst);
    game.refresh_host().unwrap();
    let rejected_pixels = pixels(&probe);
    retain_pixels(&output, "late_refusal", &rejected_pixels);
    let mut replacement = TextureRefreshOperation::Publish(stage(&mut game, rebound, root.path()));
    publish(&mut game, &probe, &lock, &mut replacement);
    assert!(game.host().unwrap().consumer().motion_frame().is_none());
    assert!(accepted_motion.has_previous() && accepted_motion.same_owner(&first_motion));
    game.refresh_host().unwrap();
    let blue = pixels(&probe);
    retain_pixels(&output, "reassignment", &blue);
    let blue_motion = game.host().unwrap().consumer().motion_frame().unwrap();
    assert!(!blue_motion.has_previous() && !blue_motion.same_owner(&accepted_motion));
    let middle = source(&game).clone();
    let mut table = TextureRefreshOperation::Publish(stage(&mut game, expanded, root.path()));
    let mut stale = TextureRefreshOperation::Publish(stage(&mut game, expanded, root.path()));
    root.close().unwrap(); // Only the owned fixture; every source/resource is already staged.
    publish(&mut game, &probe, &lock, &mut table);
    game.refresh_host().unwrap();
    let green = pixels(&probe);
    retain_pixels(&output, "table_replacement", &green);
    let green_motion = game.host().unwrap().consumer().motion_frame().unwrap();
    assert!(!green_motion.has_previous() && !green_motion.same_owner(&blue_motion));
    let mut ready = game.prepare_host_update().unwrap();
    let guard = lock.lock().unwrap();
    assert!(matches!(
        ready.update(&mut stale).unwrap(),
        TextureRefreshReply::Refused(_)
    ));
    assert_eq!(probe.presents.load(Ordering::SeqCst), 5);
    drop(guard);
    drop(ready);
    drop(stale);
    game.refresh_host().unwrap();
    let stale_pixels = pixels(&probe);
    retain_pixels(&output, "stale_retained", &stale_pixels);
    // Same paused clock and geometry: material selection is the only visible change.
    let red_mask = mask(&initial, 0);
    let blue_mask = mask(&blue, 2);
    let green_mask = mask(&green, 1);
    let covered = red_mask.iter().filter(|&&v| v).count();
    assert!(covered >= 80);
    assert_eq!(red_mask, blue_mask);
    assert_eq!(red_mask, green_mask);
    assert_eq!(rejected_pixels, initial);
    assert_eq!(stale_pixels, green);
    assert!(
        initial
            .as_chunks::<4>()
            .0
            .iter()
            .zip(blue.as_chunks::<4>().0)
            .zip(green.as_chunks::<4>().0)
            .all(|((a, b), c)| a[3] == b[3] && a[3] == c[3])
    );
    assert_eq!(game.world().unwrap().tick(), 1);
    assert_eq!(
        game.world().unwrap().spatial_view().unwrap().identity(),
        &identity
    );
    assert_eq!(game.host().unwrap().render_input().asset_generation(), 2);
    for (asset, assignment, materials) in [(&old, 0, 2), (&middle, 1, 2), (source(&game), 2, 3)] {
        let RuntimeAssetData::Model(model) = asset.data() else {
            panic!("model")
        };
        assert_eq!(model.metadata.mesh_materials, [Some(assignment)]);
        assert_eq!(model.materials.len(), materials);
    }
    // Retire old GPU scenes only after all publication guards and accepted frames.
    drop(replacement);
    drop(table);
    game.tick(&[]).unwrap();
    let continued = pixels(&probe);
    retain_pixels(&output, "continued", &continued);
    assert_eq!(game.world().unwrap().tick(), 2);
    assert!(
        (game
            .world()
            .unwrap()
            .inspect(id(1))
            .unwrap()
            .transform
            .translation[0]
            - 0.3)
            .abs()
            < 1e-12,
        "VM calls1+2 survive both publications"
    );
    assert_eq!(game.host().unwrap().consumer().accepted_frames(), 7);
    assert_eq!(probe.presents.load(Ordering::SeqCst), 7);
    assert_eq!(probe.dropped.load(Ordering::SeqCst), 0);
    fs::write(output.join("result.json"), serde_json::to_vec_pretty(&json!({
        "width":160,"height":96,"covered_primary_pixels":covered,"same_clock_tick":1,"final_tick":2,
        "catalog_generation":2,"frames":7,"presents":7,"VM_translation_x":0.3,
        "late_refusal_exact":true,"stale_refusal_exact":true,"history_owner_reset_each":true,
        "scope":"numerical material publication only; no native window or appearance/performance gate"
    })).unwrap()).unwrap();
    game.stop();
    assert_eq!(probe.dropped.load(Ordering::SeqCst), 1);
}
