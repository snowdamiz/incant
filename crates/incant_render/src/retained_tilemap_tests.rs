//! Numeric readback and bounded retained tile consumption. No appearance review.
use super::*;
use crate::hdr_output::{read, texture};
use incant_assets_runtime::RuntimeAssetData;
use incant_render_data::{SceneNode, SpriteInput};
use incant_render_native::{NativeRenderInput, UpdateKind};
use incant_runtime::{FrameCommands, NativeWorld, StableId, StructuralCommand};
fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}

// Independent expansion oracle: enumerate raw world declarations, resolve frames
// from the retained atlas, and draw ordinary sprite nodes with translated worlds.
// Production tiles instead keep one world slot and offset local vertex positions.
fn ordinary_sprite_reference(world: &NativeWorld, input: &NativeRenderInput) -> SceneInput {
    let mut result = SceneInput::default();
    for node in &input.input().nodes {
        if let Some(binding) = &node.tilemap {
            let declaration = world
                .spatial_view()
                .unwrap()
                .nodes()
                .find(|n| format!("{:032x}", u128::from_be_bytes(n.id.0)) == node.id)
                .unwrap()
                .declaration
                .unwrap()
                .clone();
            let map = declaration.tilemap.as_ref().unwrap();
            let RuntimeAssetData::Texture(texture) = binding.source().data() else {
                panic!("texture")
            };
            for layer in &map.layers {
                for (index, frame) in layer.cells.iter().enumerate() {
                    let Some(frame) = frame else {
                        continue;
                    };
                    let name = match *frame {
                        f if f == id(1) => "00000000000000000000000001",
                        f if f == id(2) => "00000000000000000000000002",
                        _ => panic!("fixture frame"),
                    };
                    let [x, y] = [index as u32 % map.size[0], index as u32 / map.size[0]];
                    result.nodes.push(SceneNode {
                        id: format!(
                            "{}:{:032x}:{y:08}:{x:08}",
                            node.id,
                            u128::from_be_bytes(layer.id.0)
                        ),
                        world: node.world
                            * glam::DMat4::from_translation(glam::DVec3::new(
                                f64::from(x) * map.cell_size[0],
                                f64::from(y) * map.cell_size[1],
                                0.,
                            )),
                        camera: None,
                        light: None,
                        mesh: None,
                        tilemap: None,
                        diagnostic: false,
                        sprite: Some(SpriteInput {
                            frames: None,
                            frame: 0,
                            source: Arc::clone(binding.source()),
                            sprite: incant_2d::SpriteRenderer {
                                texture: binding.source().info().id.clone(),
                                region: texture.metadata.atlas.as_ref().unwrap().frames[name]
                                    .region
                                    .into(),
                                size: map.cell_size,
                                pivot: [0.; 2],
                                tint: [1.; 4],
                                flip_x: false,
                                flip_y: false,
                                layer: layer.layer,
                                order: layer.order,
                                sampling: Default::default(),
                                mode: Default::default(),
                            },
                        }),
                    });
                }
            }
        } else {
            result.nodes.push(SceneNode {
                id: node.id.clone(),
                world: node.world,
                camera: node.camera.clone(),
                light: node.light.clone(),
                mesh: node.mesh.as_ref().map(|m| incant_render_data::MeshInput {
                    source: Arc::clone(&m.source),
                    cast_shadows: m.cast_shadows,
                    animation: m.animation,
                }),
                sprite: node.sprite.as_ref().map(|s| SpriteInput {
                    frames: None,
                    frame: 0,
                    source: Arc::clone(&s.source),
                    sprite: s.sprite.clone(),
                }),
                tilemap: None,
                diagnostic: false,
            });
        }
    }
    result
}
#[test]
fn source_free_v6_tile_bindings_share_atlas_and_retain_one_world_slot_per_map() {
    let (mut world, mut input, _) = super::sprite_tests::tile_fixture();
    let maps: Vec<_> = input
        .input()
        .nodes
        .iter()
        .filter_map(|n| n.tilemap.as_ref().map(Arc::clone))
        .collect();
    assert_eq!(maps.len(), 2);
    assert_eq!(maps[0].cells().len(), 6);
    assert_eq!(maps[0].cell_slots(), 12);
    assert!(Arc::ptr_eq(maps[0].source(), maps[1].source()));
    let first = &maps[0].cells()[0];
    assert_eq!(
        (first.layer, first.order, first.layer_id, first.cell),
        (-2, 3, id(3), [0, 0])
    );
    for _ in 0..16 {
        world.step().unwrap();
        input
            .try_sync(&world.spatial_view().unwrap(), |candidate, _, kind| {
                assert_eq!(kind, UpdateKind::Motion);
                for (i, map) in candidate
                    .nodes
                    .iter()
                    .filter_map(|n| n.tilemap.as_ref())
                    .enumerate()
                {
                    assert!(Arc::ptr_eq(map, &maps[i]));
                }
                Ok::<_, ()>(())
            })
            .unwrap();
    }
}
#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn cooked_source_free_tile_frames_match_raw_cell_sprite_oracle_and_survive_rejection() {
    let (mut world, mut input, _) = super::sprite_tests::tile_fixture();
    let renderer = Renderer::headless().unwrap();
    let mut gpu = renderer
        .prepare_retained(input.input(), input.camera_label())
        .unwrap();
    assert_eq!(gpu.stats().tilemap_entities, 2);
    assert_eq!(gpu.stats().tile_cells, 12);
    assert_eq!(gpu.stats().sprite_entities, 3);
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let output = texture(&renderer, format, 64, 64);
    let view = output.create_view(&Default::default());
    let target = |width| RetainedTarget {
        view: &view,
        format,
        width,
        height: 64,
        viewport: None,
    };
    gpu.submit(&renderer, input.input(), target(64)).unwrap();
    let initial = read(&renderer, &output);
    let mut changed = false;
    let mut rebuilt = 0;
    for frame in 0..8 {
        world.with_numeric_columns(|columns| {
            columns.for_each_chunk(|chunk| {
                for (entity, t) in chunk
                    .ids
                    .iter()
                    .zip(chunk.transforms.as_chunks_mut::<10>().0)
                {
                    if *entity == id(6) {
                        t[7] = if frame % 2 == 0 { -1. } else { 1. };
                        t[8] = 1. + f64::from(frame) * 0.0625;
                    }
                }
            })
        });
        if frame == 4 {
            let mut decl = world
                .spatial_view()
                .unwrap()
                .nodes()
                .find(|n| n.id == id(6))
                .unwrap()
                .declaration
                .unwrap()
                .clone();
            let map = decl.tilemap.as_mut().unwrap();
            map.layers[0].order = 9;
            map.layers[1].cells[2] = Some(id(2));
            let mut commands = FrameCommands::default();
            commands
                .push(StructuralCommand::SetSpatial {
                    entity: id(6),
                    spatial: Some(decl),
                })
                .unwrap();
            world.apply(&mut commands).unwrap();
        }
        world.step().unwrap();
        let spatial = world.spatial_view().unwrap();
        if frame == 3 {
            let tick = input.tick();
            let old = renderer.screenshot_scene_png(&gpu.live, 64, 64).unwrap();
            assert!(
                input
                    .try_sync(&spatial, |scene, _, _| gpu.submit(
                        &renderer,
                        scene,
                        target(0)
                    ))
                    .is_err()
            );
            assert_eq!(input.tick(), tick);
            assert_eq!(
                renderer.screenshot_scene_png(&gpu.live, 64, 64).unwrap(),
                old
            );
        }
        let next = input
            .try_sync(
                &spatial,
                |scene, camera, kind| -> Result<Option<RetainedScene>> {
                    if kind == UpdateKind::Rebuilt {
                        rebuilt += 1;
                        let mut next = renderer.prepare_retained(scene, camera)?;
                        next.submit(&renderer, scene, target(64))?;
                        Ok(Some(next))
                    } else {
                        gpu.submit(&renderer, scene, target(64))?;
                        Ok(None)
                    }
                },
            )
            .unwrap();
        if let Some(next) = next {
            gpu = next;
        }
        let actual = read(&renderer, &output);
        changed |= actual != initial;
        let reference = renderer
            .prepare(&ordinary_sprite_reference(&world, &input))
            .unwrap()
            .with_camera(input.camera_label())
            .unwrap();
        renderer.queue.submit([renderer
            .draw_scene(&reference, &view, format, 64, 64, None)
            .unwrap()]);
        assert_eq!(actual, read(&renderer, &output), "tile frame {frame}");
    }
    assert!(changed);
    assert_eq!(rebuilt, 1);
}

#[test]
fn sprite_and_tile_counts_share_one_cap_before_expanded_vertex_allocation() {
    let (world, input, _) = super::sprite_tests::tile_fixture();
    let source = Arc::clone(
        input
            .input()
            .nodes
            .iter()
            .find_map(|n| n.tilemap.as_ref())
            .unwrap()
            .source(),
    );
    let texture = world.spatial_view().unwrap().assets()[0].id;
    let map = incant_runtime::NativeTilemap {
        tileset: texture,
        size: [512, 330],
        cell_size: [1.; 2],
        layers: vec![incant_runtime::NativeTilemapLayer {
            id: id(1),
            layer: 0,
            order: 0,
            solid: false,
            cells: vec![Some(id(1)); 512 * 330],
        }],
    };
    let data = Arc::new(incant_render_data::TilemapInput::new(Arc::clone(&source), &map).unwrap());
    let mut candidate = SceneInput::default();
    for i in 0..2 {
        candidate.nodes.push(SceneNode {
            id: format!("map{i}"),
            world: glam::DMat4::IDENTITY,
            camera: None,
            light: None,
            mesh: None,
            sprite: None,
            tilemap: Some(Arc::clone(&data)),
            diagnostic: false,
        });
    }
    // Each map fits its slot limit; together their minimum geometry still fits.
    assert_eq!(
        crate::sprites::test_vertex_count(&candidate).unwrap(),
        337920 * 6
    );
    let sprite = &input
        .input()
        .nodes
        .iter()
        .find_map(|n| n.sprite.as_ref())
        .unwrap()
        .sprite;
    for i in 0..12000 {
        candidate.nodes.push(SceneNode {
            id: format!("sprite{i}"),
            world: glam::DMat4::IDENTITY,
            camera: None,
            light: None,
            mesh: None,
            tilemap: None,
            diagnostic: false,
            sprite: Some(SpriteInput {
                frames: None,
                frame: 0,
                source: Arc::clone(&source),
                sprite: sprite.clone(),
            }),
        });
    }
    let error = crate::sprites::test_vertex_count(&candidate).unwrap_err();
    assert!(error.to_string().contains("sprite/tile geometry"));
    // The combined minimum exceeds 128 MiB; no GPU or expanded vertex allocation
    // is possible on this CPU preflight path. A separate map-only cap would miss it.
}

#[test]
fn authoring_and_native_bind_the_same_cells_after_both_sources_are_gone() {
    let (world, input, _, authoring) = super::sprite_tests::tile_fixture_with_authoring();
    let authored: Vec<_> = authoring
        .nodes
        .iter()
        .filter(|n| n.tilemap.is_some())
        .collect();
    let native: Vec<_> = input
        .input()
        .nodes
        .iter()
        .filter(|n| n.tilemap.is_some())
        .collect();
    assert_eq!(authored.len(), 2);
    for (a, n) in authored.into_iter().zip(native) {
        assert_eq!(a.world, n.world);
        let (a, n) = (a.tilemap.as_ref().unwrap(), n.tilemap.as_ref().unwrap());
        assert_eq!(a.source().info().fingerprint, n.source().info().fingerprint);
        assert_eq!(a.cell_slots(), n.cell_slots());
        assert_eq!(a.cell_size(), n.cell_size());
        let cells = |m: &incant_render_data::TilemapInput| {
            m.cells()
                .iter()
                .map(|c| (c.layer_id, c.layer, c.order, c.cell, c.region))
                .collect::<Vec<_>>()
        };
        assert_eq!(cells(a), cells(n));
    }
    // A typed consumer cannot relabel the sealed atlas or reference a missing
    // frame; prior live bindings remain usable after these startup refusals.
    let node = world
        .spatial_view()
        .unwrap()
        .nodes()
        .find(|n| n.id == id(6))
        .unwrap();
    let mut map = node
        .declaration
        .unwrap()
        .tilemap
        .as_ref()
        .unwrap()
        .as_ref()
        .clone();
    let source = Arc::clone(
        input
            .input()
            .nodes
            .iter()
            .find_map(|n| n.tilemap.as_ref())
            .unwrap()
            .source(),
    );
    map.tileset = id(99);
    assert!(matches!(
        incant_render_data::TilemapInput::new(Arc::clone(&source), &map),
        Err(SceneError::TilemapBinding)
    ));
    map.tileset = node.declaration.unwrap().tilemap.as_ref().unwrap().tileset;
    map.layers[0].cells[0] = Some(id(99));
    assert!(
        matches!(incant_render_data::TilemapInput::new(source,&map),Err(SceneError::TilemapFrame(f)) if f==id(99))
    );
    assert_eq!(
        crate::scene::resolve(&authoring).unwrap().stats.tile_cells,
        12
    );
}
