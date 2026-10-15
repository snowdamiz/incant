use super::materials::{data, publish};
use super::{material_fixture::*, *};

fn set_navigation(world: &mut NativeWorld, reference: VisualAssetRef) {
    let mut commands = FrameCommands::default();
    commands
        .push(StructuralCommand::SetNavigation {
            entity: id(2),
            navigation: Some(Box::new(nav(reference))),
        })
        .unwrap();
    world.apply(&mut commands).unwrap();
}
#[test]
fn material_only_navigation_accepts_before_and_after_stage_but_geometry_still_refuses() {
    for add_after_stage in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let first = material_model(root.path(), Tables::Initial);
        let mut world = world(vec![first]);
        if !add_after_stage {
            set_navigation(&mut world, first);
        }
        let mut input = load(root.path(), &world);
        let next = material_model(root.path(), Tables::Expanded);
        let mut candidate = input
            .stage_texture_refresh(
                &world.spatial_view().unwrap(),
                prepared(&input, root.path(), &[next]),
            )
            .unwrap();
        if add_after_stage {
            set_navigation(&mut world, first);
        }
        let report = world.navigation_report(id(2)).unwrap().clone();
        input
            .try_publish_textures(&world.spatial_view().unwrap(), &mut candidate, || {
                Ok::<_, ()>(())
            })
            .unwrap();
        assert_eq!(world.navigation_report(id(2)).unwrap(), &report);
        assert_eq!(
            world.spatial_view().unwrap().assets()[0].fingerprint,
            first.fingerprint
        );
        assert_eq!(model_asset(&input).info().fingerprint, key(next));
        let geometry = model(root.path(), 2., DARK, [1.; 4], Variant::Static);
        assert!(
            input
                .stage_texture_refresh(
                    &world.spatial_view().unwrap(),
                    prepared(&input, root.path(), &[geometry])
                )
                .is_err()
        );
        assert_eq!(model_asset(&input).info().fingerprint, key(next));
    }
}

#[test]
fn a_signed_zero_geometry_change_is_not_admitted_as_material_only_for_baked_navigation() {
    let root = tempfile::tempdir().unwrap();
    let first = material_model(root.path(), Tables::Initial);
    let mut world = world(vec![first]);
    set_navigation(&mut world, first);
    let input = load(root.path(), &world);
    let binary = root.path().join("tri.bin");
    let mut bytes = fs::read(&binary).unwrap();
    bytes[4..8].copy_from_slice(&(-0.0_f32).to_le_bytes());
    fs::write(binary, bytes).unwrap();
    let next = recook(root.path());
    let changed =
        incant_assets_runtime::load_model(&root.path().join(".incant/cache/models"), &key(next))
            .unwrap();
    assert_ne!(
        data(&model_asset(&input)).meshes[0].vertices[0][1].to_bits(),
        changed.meshes[0].vertices[0][1].to_bits()
    );
    assert!(
        input
            .stage_texture_refresh(
                &world.spatial_view().unwrap(),
                prepared(&input, root.path(), &[next])
            )
            .is_err()
    );
    assert_eq!(model_asset(&input).info().fingerprint, key(first));
}

#[test]
fn prior_geometry_divergence_survives_material_changes_restoration_and_unrelated_texture_refresh() {
    let root = tempfile::tempdir().unwrap();
    let first = material_model(root.path(), Tables::Initial);
    let old_texture = texture(root.path(), RED);
    let mut world = world(vec![first, old_texture]);
    let mut input = load(root.path(), &world);
    let changed = material_model_with_scale(root.path(), Tables::Initial, 2.);
    publish(&mut input, &world, root.path(), &[changed, old_texture]);
    let accepted = model_asset(&input);
    let material = material_model_with_scale(root.path(), Tables::Expanded, 2.);
    assert_eq!(
        data(&accepted).meshes,
        incant_assets_runtime::load_model(
            &root.path().join(".incant/cache/models"),
            &key(material)
        )
        .unwrap()
        .meshes
    );
    let mut staged = input
        .stage_texture_refresh(
            &world.spatial_view().unwrap(),
            prepared(&input, root.path(), &[material, old_texture]),
        )
        .unwrap();
    set_navigation(&mut world, first); // Newly declared source still carries startup geometry authority.
    assert!(
        input
            .try_publish_textures(
                &world.spatial_view().unwrap(),
                &mut staged,
                || -> Result<(), ()> {
                    panic!("startup geometry divergence must refuse before consumer")
                }
            )
            .is_err()
    );
    assert_eq!(model_asset(&input).version_id(), accepted.version_id());
    // An unchanged diverged model must not block a separate texture publication.
    let next_texture = texture(root.path(), DARK);
    publish(&mut input, &world, root.path(), &[changed, next_texture]);
    assert_eq!(model_asset(&input).version_id(), accepted.version_id());
    for model in [material, first] {
        // Even restoring startup content conservatively requires restart.
        assert!(
            input
                .stage_texture_refresh(
                    &world.spatial_view().unwrap(),
                    prepared(&input, root.path(), &[model, next_texture])
                )
                .is_err()
        );
    }
    assert_eq!(
        input
            .input()
            .environment
            .as_ref()
            .unwrap()
            .source
            .info()
            .fingerprint,
        key(next_texture)
    );
}

#[test]
fn restoring_startup_geometry_does_not_clear_the_conservative_latch_until_fresh_load() {
    let root = tempfile::tempdir().unwrap();
    let first = material_model(root.path(), Tables::Initial);
    let mut world = world(vec![first]);
    let mut input = load(root.path(), &world);
    let changed = material_model_with_scale(root.path(), Tables::Initial, 2.);
    publish(&mut input, &world, root.path(), &[changed]);
    publish(&mut input, &world, root.path(), &[first]); // Same cooked version as an asset Undo.
    set_navigation(&mut world, first);
    let materials = material_model(root.path(), Tables::Expanded);
    assert!(
        input
            .stage_texture_refresh(
                &world.spatial_view().unwrap(),
                prepared(&input, root.path(), &[materials])
            )
            .is_err()
    );
    assert_eq!(model_asset(&input).info().fingerprint, key(first));
    let mut restarted = load(root.path(), &world);
    publish(&mut restarted, &world, root.path(), &[materials]);
    assert_eq!(model_asset(&restarted).info().fingerprint, key(materials));
}
