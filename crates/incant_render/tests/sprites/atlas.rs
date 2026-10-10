use super::*;
const A: &str = "00000000000000000000000001";
const B: &str = "00000000000000000000000002";
fn repack(f: &mut Fixture) -> [u32; 4] {
    let cooked = cook_texture(
        f.root.path(),
        Path::new("sheet.atlas.json"),
        &f.root.path().join(".incant/cache/textures"),
        TextureUsage::Color,
    )
    .unwrap();
    let region = cooked.metadata.atlas.as_ref().unwrap().frames[B].region;
    let asset = f.project.assets.get_mut(&f.texture).unwrap();
    asset.path = "sheet.atlas.json".into();
    asset.sha256 = cooked.metadata.fingerprint;
    f.assets.sync_project(&f.project, f.root.path()).unwrap();
    region
}
#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn atlas_frame_ids_survive_repacking_source_removal_and_failed_replacement() {
    let renderer = Renderer::headless().unwrap();
    let mut f = Fixture::new(
        &[
            255, 0, 0, 255, 0, 255, 0, 128, 0, 0, 255, 255, 255, 255, 0, 255,
        ],
        2,
        2,
    );
    let original = f.pixels(&renderer);
    crate::materials::write_png(
        &f.root.path().join("extra.png"),
        &[255, 0, 255, 255].repeat(2),
        2,
        1,
    );
    let descriptor = json!({"version":1,"id":"00000000000000000000000009","maximum_size":[64,64],"padding":2,
        "frames":{(A):{"name":"Extra","path":"extra.png"},(B):{"name":"Target","path":"sprite.png"}}});
    std::fs::write(
        f.root.path().join("sheet.atlas.json"),
        descriptor.to_string(),
    )
    .unwrap();
    let before = repack(&mut f);
    f.edit("region", json!({"frame":B}));
    assert_eq!(f.pixels(&renderer), original);
    let retained = f.scene(&renderer);
    crate::materials::write_png(
        &f.root.path().join("extra.png"),
        &[0, 255, 255, 255].repeat(13 * 17),
        13,
        17,
    );
    let after = repack(&mut f);
    assert_ne!(
        before, after,
        "fixture must actually relocate the stable frame"
    );
    assert_eq!(f.pixels(&renderer), original);
    assert_eq!(
        decode(&renderer.screenshot_scene_png(&retained, 64, 64).unwrap()),
        original
    );
    assert_eq!(
        f.project.scenes[&f.scene].entities[&f.sprite].components["SpriteRenderer"]["region"],
        json!({"frame":B})
    );
    for path in ["sprite.png", "extra.png", "sheet.atlas.json"] {
        std::fs::remove_file(f.root.path().join(path)).unwrap();
    }
    f.assets = AssetStore::default();
    f.assets.sync_project(&f.project, f.root.path()).unwrap();
    assert_eq!(f.pixels(&renderer), original);
    f.edit("region", json!({"frame":"00000000000000000000000003"}));
    let error = renderer
        .prepare_scene(&f.project, &f.assets)
        .err()
        .expect("unknown frame must fail explicitly");
    assert!(error.to_string().contains("sprite atlas frame"));
    assert_eq!(
        decode(&renderer.screenshot_scene_png(&retained, 64, 64).unwrap()),
        original
    );
    f.edit("region", json!({"frame":B}));
    f.edit(
        "mode",
        json!({"kind":"nine_slice","border":[3,0,0,0],"pixels_per_unit":8}),
    );
    let error = renderer.prepare_scene(&f.project, &f.assets).err().unwrap();
    let incant_render::ResourceError::SpriteGeometry {
        entity,
        texture,
        region,
        source,
        ..
    } = error
        .downcast_ref::<incant_render::ResourceError>()
        .unwrap()
    else {
        panic!("expected resolved frame geometry diagnostic, got {error}");
    };
    assert_eq!(entity, &f.sprite);
    assert_eq!(texture, &f.texture);
    assert_eq!(
        *region, after,
        "diagnostic identifies the resolved frame rectangle"
    );
    assert_eq!(*source, incant_2d::SpriteError::Slice);
}
