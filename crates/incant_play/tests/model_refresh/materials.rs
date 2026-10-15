use super::{material_fixture::*, *};
use incant_assets::RuntimeAssetData;
use incant_cmd::{Actor, CommandBus};
use incant_import::ImportSnapshot;

fn import(bus: &mut CommandBus, root: &Path) {
    ImportSnapshot::capture(bus)
        .prepare(root, None, &imports())
        .unwrap()
        .commit(
            bus,
            Actor::user("import"),
            "replace model material tables and texture",
        )
        .unwrap();
}
fn current_model(game: &PlaySession<OfflineBackend, Observer>) -> &incant_assets::CookedModel {
    let RuntimeAssetData::Model(model) = game.consumer().unwrap().unwrap().model.data() else {
        panic!("model")
    };
    model
}
fn prepare_current(
    basis: &TextureRefreshBasis,
    bus: &CommandBus,
    root: &Path,
    game: &mut PlaySession<OfflineBackend, Observer>,
) {
    let prepared = basis
        .prepare(bus.project(), root, game.texture_refresh_ticket().unwrap())
        .unwrap()
        .unwrap();
    publish(game, prepared);
}

#[test]
fn real_material_import_undo_redo_and_table_resize_preserve_world_vm_and_old_resources() {
    let root = tempfile::tempdir().unwrap();
    material_model(root.path(), Tables::Initial);
    texture(root.path(), RED);
    let mut bus = CommandBus::new(incant_doc::Project::empty("Material refresh")).unwrap();
    import(&mut bus, root.path());
    let first = imported_references(&bus);
    let basis = TextureRefreshBasis::capture(bus.project(), &first).unwrap();
    let mut game = game(root.path(), first.clone());
    game.step(&[]).unwrap();
    let identity = game
        .world()
        .unwrap()
        .spatial_view()
        .unwrap()
        .identity()
        .clone();
    let old = game.consumer().unwrap().unwrap().model.clone();
    for (tables, count, material) in [
        (Tables::Rebound, 2, Some(1)),
        (Tables::Expanded, 3, Some(2)),
        (Tables::Reordered, 2, Some(0)),
        (Tables::Empty, 0, None),
        (Tables::Rebound, 2, Some(1)),
    ] {
        material_model(root.path(), tables);
        import(&mut bus, root.path());
        let next = imported_references(&bus);
        assert_eq!(
            next.iter().map(|r| r.id).collect::<Vec<_>>(),
            first.iter().map(|r| r.id).collect::<Vec<_>>()
        );
        prepare_current(&basis, &bus, root.path(), &mut game);
        assert_eq!(current_model(&game).materials.len(), count);
        assert_eq!(current_model(&game).metadata.mesh_materials, [material]);
        let accepted = game.consumer().unwrap().unwrap().model.clone();
        bus.undo().unwrap();
        prepare_current(&basis, &bus, root.path(), &mut game);
        bus.redo().unwrap();
        prepare_current(&basis, &bus, root.path(), &mut game);
        assert_eq!(
            game.consumer().unwrap().unwrap().model.info().fingerprint,
            accepted.info().fingerprint
        );
        assert_eq!(game.world().unwrap().tick(), 1);
        assert_eq!(
            game.world().unwrap().spatial_view().unwrap().identity(),
            &identity
        );
    }
    let RuntimeAssetData::Model(original) = old.data() else {
        panic!("old model")
    };
    assert_eq!(original.materials.len(), 2);
    assert_eq!(original.metadata.mesh_materials, [Some(0)]);
    root.close().unwrap();
    game.step(&[]).unwrap();
    assert_eq!(
        game.world()
            .unwrap()
            .inspect(id(1))
            .unwrap()
            .transform
            .translation[0],
        3.
    ); // calls1+2, not a new VM
}

#[test]
fn bad_source_then_bad_cooked_member_cannot_partially_publish_rebound_materials() {
    let root = tempfile::tempdir().unwrap();
    material_model(root.path(), Tables::Initial);
    texture(root.path(), RED);
    let mut bus = CommandBus::new(incant_doc::Project::empty("Atomic material batch")).unwrap();
    import(&mut bus, root.path());
    let first = imported_references(&bus);
    let basis = TextureRefreshBasis::capture(bus.project(), &first).unwrap();
    let mut game = game(root.path(), first.clone());
    game.step(&[]).unwrap();
    let old_model = game.consumer().unwrap().unwrap().model.clone();
    let old_texture = game
        .consumer()
        .unwrap()
        .unwrap()
        .texture
        .as_ref()
        .unwrap()
        .clone();
    material_model(root.path(), Tables::Expanded);
    let source_path = root.path().join("tri.gltf");
    let source = std::fs::read(&source_path).unwrap();
    let mut dangling: serde_json::Value = serde_json::from_slice(&source).unwrap();
    dangling["materials"][2]["normalTexture"]["index"] = serde_json::json!(999);
    std::fs::write(&source_path, serde_json::to_vec(&dangling).unwrap()).unwrap();
    assert!(
        ImportSnapshot::capture(&bus)
            .prepare(root.path(), None, &imports())
            .is_err()
    );
    assert_eq!(bus.revision(), 1);
    assert_eq!(imported_references(&bus), first);
    std::fs::write(source_path, source).unwrap();
    std::fs::write(root.path().join("pixel.png"), b"invalid owned PNG").unwrap();
    assert!(
        ImportSnapshot::capture(&bus)
            .prepare(root.path(), None, &imports())
            .is_err()
    );
    assert_eq!(bus.revision(), 1);
    assert_eq!(imported_references(&bus), first);
    texture(root.path(), DARK);
    import(&mut bus, root.path());
    let next = imported_references(&bus);
    let next_texture = next
        .iter()
        .find(|r| r.kind == VisualAssetKind::Texture)
        .unwrap();
    let cache = root.path().join(format!(
        ".incant/cache/textures/{}.json",
        key(*next_texture)
    ));
    let saved = std::fs::read(&cache).unwrap();
    std::fs::write(&cache, b"invalid owned cooked texture").unwrap();
    assert!(
        basis
            .prepare(
                bus.project(),
                root.path(),
                game.texture_refresh_ticket().unwrap()
            )
            .is_err()
    );
    assert!(Arc::ptr_eq(
        &game.consumer().unwrap().unwrap().model,
        &old_model
    ));
    assert!(Arc::ptr_eq(
        game.consumer().unwrap().unwrap().texture.as_ref().unwrap(),
        &old_texture
    ));
    assert_eq!(game.world().unwrap().tick(), 1);
    std::fs::write(cache, saved).unwrap();
    prepare_current(&basis, &bus, root.path(), &mut game);
    assert_eq!(current_model(&game).metadata.mesh_materials, [Some(2)]);
    assert_eq!(current_model(&game).materials.len(), 3);
    assert_ne!(
        game.consumer()
            .unwrap()
            .unwrap()
            .texture
            .as_ref()
            .unwrap()
            .version_id(),
        old_texture.version_id()
    );
    bus.undo().unwrap();
    prepare_current(&basis, &bus, root.path(), &mut game);
    assert_eq!(current_model(&game).metadata.mesh_materials, [Some(0)]);
    game.step(&[]).unwrap();
    assert_eq!(
        game.world()
            .unwrap()
            .inspect(id(1))
            .unwrap()
            .transform
            .translation[0],
        3.
    );
}
