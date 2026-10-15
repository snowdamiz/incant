use super::{material_fixture::*, *};
use incant_assets_runtime::{AlphaMode, CookedModel, RuntimeAsset};
use std::path::Path;

pub(super) fn data(asset: &RuntimeAsset) -> &CookedModel {
    let RuntimeAssetData::Model(model) = asset.data() else {
        panic!("model")
    };
    model
}
pub(super) fn publish(
    input: &mut NativeRenderInput,
    world: &NativeWorld,
    root: &Path,
    next: &[VisualAssetRef],
) {
    let mut candidate = input
        .stage_texture_refresh(&world.spatial_view().unwrap(), prepared(input, root, next))
        .unwrap();
    let (receipt, allocations, disposals) = allocations::record_lifecycle(|| {
        input
            .try_publish_textures(&world.spatial_view().unwrap(), &mut candidate, || {
                Ok::<_, ()>(())
            })
            .unwrap()
    });
    assert_eq!((allocations, disposals), (0, 0));
    assert!(receipt.changed > 0);
}

#[test]
fn complete_material_tables_rebind_grow_reorder_and_shrink_without_changing_world_or_old_owners() {
    let root = tempfile::tempdir().unwrap();
    let first = material_model(root.path(), Tables::Initial);
    let world = world(vec![first]);
    let mut input = load(root.path(), &world);
    let original = model_asset(&input);
    let mut versions = vec![original.clone()];
    for (index, tables) in [
        Tables::Rebound,
        Tables::Expanded,
        Tables::Reordered,
        Tables::Empty,
        Tables::Rebound,
    ]
    .into_iter()
    .enumerate()
    {
        let next = material_model(root.path(), tables);
        // Legacy texture-only entry cannot acquire model publication authority.
        assert!(
            input
                .texture_refresh_ticket()
                .prepare(&[next], &root.path().join(".incant/cache/textures"))
                .is_err()
        );
        publish(&mut input, &world, root.path(), &[next]);
        assert_eq!(input.asset_generation(), index as u64 + 1);
        assert_eq!(world.tick(), 0);
        let current = model_asset(&input);
        let model = data(&current);
        assert_eq!(model.meshes, data(&original).meshes);
        match tables {
            Tables::Rebound => {
                assert_eq!(
                    (model.materials.len(), model.metadata.textures.len()),
                    (2, 2)
                );
                assert_eq!(model.metadata.mesh_materials, [Some(1)]);
                assert_eq!(model.materials[1].alpha_mode, AlphaMode::Blend);
                assert_eq!(model.materials[1].base_color, [0.2, 0.6, 0.4, 0.5]);
            }
            Tables::Expanded => {
                assert_eq!(
                    (model.materials.len(), model.metadata.textures.len()),
                    (3, 3)
                );
                assert_eq!(model.metadata.mesh_materials, [Some(2)]);
                assert!(model.images.len() > data(&original).images.len());
                let material = &model.materials[2];
                assert_eq!(material.maps.map(|m| m.unwrap().texture), [2, 1, 0, 1, 2]);
                assert_eq!(material.alpha_mode, AlphaMode::Mask);
                assert!(material.double_sided);
                assert_eq!(model.metadata.textures[1].wrap_s, 33071);
            }
            Tables::Reordered => {
                assert_eq!(model.materials.len(), 2);
                assert_eq!(model.metadata.mesh_materials, [Some(0)]);
                assert_eq!(model.materials[0].base_color, [0.2, 0.6, 0.4, 0.5]);
            }
            Tables::Empty => {
                assert!(
                    model.materials.is_empty()
                        && model.images.is_empty()
                        && model.metadata.textures.is_empty()
                );
                assert_eq!(model.metadata.mesh_materials, [None]);
            }
            Tables::Initial => unreachable!(),
        }
        assert_eq!(data(&original).metadata.mesh_materials, [Some(0)]);
        assert_eq!(data(&original).materials.len(), 2);
        assert_eq!(original.info().fingerprint, key(first));
        versions.push(current);
    }
    assert_eq!(versions.len(), 6);
}

#[test]
fn mixed_table_and_texture_failure_stale_ticket_and_late_refusal_keep_complete_catalog() {
    let root = tempfile::tempdir().unwrap();
    let first = material_model(root.path(), Tables::Initial);
    let original_texture = texture(root.path(), RED);
    let world = world(vec![first, original_texture]);
    let mut input = load(root.path(), &world);
    let original = model_asset(&input);
    let next = material_model(root.path(), Tables::Expanded);
    let next_texture = texture(root.path(), DARK);
    let references = [next, next_texture];
    let cache = root
        .path()
        .join(format!(".incant/cache/textures/{}.json", key(next_texture)));
    let bytes = fs::read(&cache).unwrap();
    fs::write(&cache, b"corrupt owned mixed candidate").unwrap();
    assert!(
        input
            .texture_refresh_ticket()
            .prepare_visuals(
                &references,
                VisualCaches {
                    models: &root.path().join(".incant/cache/models"),
                    textures: &root.path().join(".incant/cache/textures")
                }
            )
            .is_err()
    );
    fs::write(cache, bytes).unwrap();
    assert!(matches!(
        input.texture_refresh_ticket().prepare_visuals_with_budget(
            &references,
            VisualCaches {
                models: &root.path().join(".incant/cache/models"),
                textures: &root.path().join(".incant/cache/textures")
            },
            0
        ),
        Err(NativeRenderError::PayloadLimit)
    ));
    let mut refused = input
        .stage_texture_refresh(
            &world.spatial_view().unwrap(),
            prepared(&input, root.path(), &references),
        )
        .unwrap();
    assert!(
        input
            .try_publish_textures(&world.spatial_view().unwrap(), &mut refused, || Err(
                "late consumer refusal"
            ))
            .is_err()
    );
    assert_eq!(model_asset(&input).version_id(), original.version_id());
    assert_eq!(
        input
            .input()
            .environment
            .as_ref()
            .unwrap()
            .source
            .info()
            .fingerprint,
        key(original_texture)
    );
    let mut stale = input
        .stage_texture_refresh(
            &world.spatial_view().unwrap(),
            prepared(&input, root.path(), &references),
        )
        .unwrap();
    publish(&mut input, &world, root.path(), &references);
    let accepted = model_asset(&input);
    assert!(
        input
            .try_publish_textures(
                &world.spatial_view().unwrap(),
                &mut stale,
                || -> Result<(), ()> { panic!("stale candidate must not reach consumer") }
            )
            .is_err()
    );
    assert_eq!(model_asset(&input).version_id(), accepted.version_id());
    assert_ne!(accepted.version_id(), original.version_id());
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
    assert_eq!(world.tick(), 0);
}
