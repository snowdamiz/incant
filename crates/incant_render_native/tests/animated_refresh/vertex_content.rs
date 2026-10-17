use super::*;

#[test]
fn replacement_vertices_use_the_current_pose_and_preserve_root_motion_without_publication_allocations()
 {
    let root = tempfile::tempdir().unwrap();
    let original = model(root.path(), Change::Original);
    let mut world = NativeWorld::new(scene(root.path(), vec![original], true));
    let mut input = load(root.path(), &world).unwrap();
    for _ in 0..12 {
        world.step().unwrap();
    }
    sync(&mut input, &world);
    let old = mesh(&input).source.clone();
    let next = model(root.path(), Change::Geometry);
    let mut staged = input
        .stage_texture_refresh(
            &world.spatial_view().unwrap(),
            prepared(&input, root.path(), &[next]),
        )
        .unwrap();
    // The stage cannot freeze the older pose while the game keeps playing.
    world.step().unwrap();
    sync(&mut input, &world);
    let state = world.animation_state(id(1)).unwrap();
    let identity = world.spatial_view().unwrap().identity().clone();
    let mut before_root = Vec::new();
    world.snapshot_root_motion(&mut before_root).unwrap();
    let (_, allocations, disposals) = allocations::record_lifecycle(|| {
        input
            .try_publish_textures(&world.spatial_view().unwrap(), &mut staged, || {
                Ok::<_, ()>(())
            })
            .unwrap()
    });
    assert_eq!((allocations, disposals), (0, 0));
    assert_eq!(world.animation_state(id(1)), Some(state));
    assert_eq!(world.spatial_view().unwrap().identity(), &identity);
    assert_eq!(mesh(&input).animation, Some(state.sample));
    assert_eq!(mesh(&input).animation_root, Some(1));
    assert_eq!(mesh(&input).source.info().fingerprint, key(next));
    let mut after_root = Vec::new();
    world.snapshot_root_motion(&mut after_root).unwrap();
    assert_eq!(before_root, after_root);
    // Flat-normal generation expands the source triangle [2, 0, 1]. Its
    // changed source vertex 0 is therefore cooked vertex 1, with joint 0 weight 1.
    let changed_position = |source: &RuntimeAsset| {
        let RuntimeAssetData::Model(model) = source.data() else {
            panic!()
        };
        let mut player = AnimationPlayer::new(model).unwrap();
        player
            .sample(state.sample.clip.map(usize::from), state.sample.seconds)
            .unwrap();
        let vertex = model.meshes[0].vertices[1];
        player
            .skin_position(3, 0, 1, [vertex[0], vertex[1], vertex[2]])
            .unwrap()
    };
    let before = changed_position(&old);
    let after = changed_position(&mesh(&input).source);
    assert!((after[0] - before[0] - 0.25).abs() < 1e-9);
    assert_eq!(&after[1..], &before[1..]);
    // Retired assets and subsequent playback remain usable with the cache gone.
    root.close().unwrap();
    world.step().unwrap();
    sync(&mut input, &world);
    assert_eq!(world.tick(), 14);
    assert!((world.inspect(id(1)).unwrap().transform.translation[0] - 14. / 60.).abs() < 2e-6);
    assert!((mesh(&input).animation.unwrap().seconds - 14. / 60.).abs() < 1e-12);
}

#[test]
fn regenerated_vertex_attributes_publish_as_a_complete_asset_and_undo_preserves_the_clock() {
    let root = tempfile::tempdir().unwrap();
    let original = model(root.path(), Change::Original);
    let mut world = NativeWorld::new(scene(root.path(), vec![original], false));
    let mut input = load(root.path(), &world).unwrap();
    world.step().unwrap();
    sync(&mut input, &world);
    let old = mesh(&input).source.clone();
    let next = model(root.path(), Change::GeneratedTangents);
    let RuntimeAssetData::Model(old_model) = old.data() else {
        panic!()
    };
    let candidate = prepared(&input, root.path(), &[next]);
    let mut staged = input
        .stage_texture_refresh(&world.spatial_view().unwrap(), candidate)
        .unwrap();
    let state = world.animation_state(id(1)).unwrap();
    input
        .try_publish_textures(&world.spatial_view().unwrap(), &mut staged, || {
            Ok::<_, ()>(())
        })
        .unwrap();
    let RuntimeAssetData::Model(new_model) = mesh(&input).source.data() else {
        panic!()
    };
    assert_ne!(old_model.meshes[0].vertices, new_model.meshes[0].vertices);
    assert_eq!(old_model.meshes[0].indices, new_model.meshes[0].indices);
    assert_eq!(
        position(&old, state.sample),
        position(&mesh(&input).source, state.sample)
    );
    for reference in [original, next] {
        let mut staged = input
            .stage_texture_refresh(
                &world.spatial_view().unwrap(),
                prepared(&input, root.path(), &[reference]),
            )
            .unwrap();
        input
            .try_publish_textures(&world.spatial_view().unwrap(), &mut staged, || {
                Ok::<_, ()>(())
            })
            .unwrap();
        assert_eq!(mesh(&input).source.info().fingerprint, key(reference));
        assert_eq!(world.animation_state(id(1)), Some(state));
        assert_eq!(world.tick(), 1);
    }
}
