use super::*;

#[test]
fn equal_count_clip_skin_and_geometry_changes_refuse_without_mutating_the_live_catalog() {
    let root = tempfile::tempdir().unwrap();
    let original = model(root.path(), Change::Original);
    let world = NativeWorld::new(scene(root.path(), vec![original], false));
    let input = load(root.path(), &world).unwrap();
    let old = mesh(&input).source.clone();
    for change in [
        Change::Duration,
        Change::ClipOrder,
        Change::ClipName,
        Change::ChannelTarget,
        Change::Interpolation,
        Change::InverseBind,
        Change::JointOrder,
        Change::SkinBinding,
        Change::Influences,
        Change::Rest,
        Change::Geometry,
    ] {
        // Each replacement is actually cooked/decoded, with the same asset ID,
        // clip count and primitive/material count; only the binding changes.
        let next = model(root.path(), change);
        assert_ne!(next.fingerprint, original.fingerprint);
        let (models, textures) = caches(root.path());
        assert!(matches!(
            input.texture_refresh_ticket().prepare_visuals(
                &[next],
                VisualCaches {
                    models: &models,
                    textures: &textures
                }
            ),
            Err(NativeRenderError::TextureRefresh(_))
        ));
        assert!(Arc::ptr_eq(&old, &mesh(&input).source));
        assert_eq!((input.asset_generation(), world.tick()), (0, 0));
    }
}

#[test]
fn bad_mixed_resource_and_animation_payload_budget_preserve_both_old_versions() {
    let root = tempfile::tempdir().unwrap();
    let original = model(root.path(), Change::Original);
    let original_texture = texture(root.path(), RED);
    let world = NativeWorld::new(scene(root.path(), vec![original, original_texture], false));
    let input = load(root.path(), &world).unwrap();
    let old = mesh(&input).source.clone();
    let old_texture = input.input().environment.as_ref().unwrap().source.clone();
    let next = model(root.path(), Change::Keys);
    let next_texture = texture(root.path(), DARK);
    let (models, textures) = caches(root.path());
    for path in [
        models.join(format!("{}.incmodel", key(next))),
        textures.join(format!("{}.json", key(next_texture))),
    ] {
        let saved = fs::read(&path).unwrap();
        fs::write(&path, b"invalid owned cached fixture").unwrap();
        assert!(
            input
                .texture_refresh_ticket()
                .prepare_visuals(
                    &[next, next_texture],
                    VisualCaches {
                        models: &models,
                        textures: &textures
                    }
                )
                .is_err()
        );
        fs::write(path, saved).unwrap();
        assert!(Arc::ptr_eq(&old, &mesh(&input).source));
        assert!(Arc::ptr_eq(
            &old_texture,
            &input.input().environment.as_ref().unwrap().source
        ));
    }
    // Extra key payload must count against the existing published-set cap.
    assert!(matches!(
        input.texture_refresh_ticket().prepare_visuals_with_budget(
            &[next, original_texture],
            VisualCaches {
                models: &models,
                textures: &textures
            },
            input.payload_bytes()
        ),
        Err(NativeRenderError::PayloadLimit)
    ));
    assert!(Arc::ptr_eq(&old, &mesh(&input).source));
}

#[test]
fn changed_baked_root_tracks_refuse_but_nonroot_keys_keep_simulation_catalog_and_clock() {
    let root = tempfile::tempdir().unwrap();
    let original = model(root.path(), Change::Original);
    let mut world = NativeWorld::new(scene(root.path(), vec![original], true));
    let mut input = load(root.path(), &world).unwrap();
    for _ in 0..12 {
        world.step().unwrap();
    }
    sync(&mut input, &world);
    let old = mesh(&input).source.clone();
    let changed_root = model(root.path(), Change::Keys);
    let candidate = prepared(&input, root.path(), &[changed_root]);
    assert!(matches!(
        input.stage_texture_refresh(&world.spatial_view().unwrap(), candidate),
        Err(NativeRenderError::Animation {
            reason: "root catalog differs from cooked model",
            ..
        })
    ));
    assert!(Arc::ptr_eq(&old, &mesh(&input).source));
    let next = model(root.path(), Change::NonRootRotation);
    let state = world.animation_state(id(1)).unwrap();
    let mut before = Vec::new();
    world.snapshot_root_motion(&mut before).unwrap();
    let mut stage = input
        .stage_texture_refresh(
            &world.spatial_view().unwrap(),
            prepared(&input, root.path(), &[next]),
        )
        .unwrap();
    input
        .try_publish_textures(&world.spatial_view().unwrap(), &mut stage, || {
            Ok::<_, ()>(())
        })
        .unwrap();
    let mut after = Vec::new();
    world.snapshot_root_motion(&mut after).unwrap();
    assert_eq!(before, after);
    assert_eq!(world.animation_state(id(1)), Some(state));
    assert_eq!(mesh(&input).animation_root, Some(1));
    assert_eq!(mesh(&input).source.info().fingerprint, key(next));
    world.step().unwrap();
    sync(&mut input, &world);
    assert!((world.inspect(id(1)).unwrap().transform.translation[0] - 13. / 60.).abs() < 2e-6);
}

#[test]
fn a_changed_native_binding_after_staging_refuses_before_consumer_publication() {
    let root = tempfile::tempdir().unwrap();
    let original = model(root.path(), Change::Original);
    let mut world = NativeWorld::new(scene(root.path(), vec![original], false));
    let mut input = load(root.path(), &world).unwrap();
    let old = mesh(&input).source.clone();
    let next = model(root.path(), Change::Keys);
    let mut staged = input
        .stage_texture_refresh(
            &world.spatial_view().unwrap(),
            prepared(&input, root.path(), &[next]),
        )
        .unwrap();
    let mut declaration = world.spatial_view().unwrap().nodes()[0]
        .declaration
        .unwrap()
        .clone();
    declaration.mesh.as_mut().unwrap().cast_shadows = false;
    apply(
        &mut world,
        StructuralCommand::SetSpatial {
            entity: id(1),
            spatial: Some(declaration),
        },
    );
    assert!(
        input
            .try_publish_textures(
                &world.spatial_view().unwrap(),
                &mut staged,
                || -> Result<(), ()> { panic!("stale binding reached publication") }
            )
            .is_err()
    );
    assert!(Arc::ptr_eq(&old, &mesh(&input).source));
    assert_eq!(input.asset_generation(), 0);
    sync(&mut input, &world);
    assert!(Arc::ptr_eq(&old, &mesh(&input).source));
}
