use incant_runtime::*;
use incant_types::{Camera, Transform, Velocity};
use sha2::{Digest, Sha256};
use std::sync::Arc;
fn id(n: u8) -> StableId {
    StableId([n; 16])
}
fn entity(n: u8) -> CookedEntity {
    CookedEntity {
        id: id(n),
        parent: None,
        transform: Transform::default(),
        velocity: None,
    }
}
fn fixture() -> CookedScene {
    let assets = vec![
        VisualAssetRef {
            id: id(90),
            kind: VisualAssetKind::Texture,
            fingerprint: [7; 32],
        },
        VisualAssetRef {
            id: id(91),
            kind: VisualAssetKind::Model,
            fingerprint: [8; 32],
        },
    ];
    let mut actor = CookedSpatial::empty(id(1));
    actor.sprite = Some(Box::new(NativeSpriteRenderer {
        texture: id(90),
        region: NativeSpriteRegion::Pixels([0, 0, 2, 2]),
        size: [1.; 2],
        pivot: [0.5; 2],
        tint: [1.; 4],
        flip_x: true,
        flip_y: false,
        layer: 2,
        order: -3,
        sampling: SpriteSampling::Nearest,
        mode: SpriteMode::Simple,
    }));
    actor.flipbook = Some(Arc::new(NativeFlipbook {
        frames: (0..7)
            .map(|x| NativeSpriteRegion::Pixels([x * 2, 0, 2, 2]))
            .collect::<Vec<_>>()
            .into(),
        rate: FrameRate {
            numerator: 30000,
            denominator: 1001,
        },
        looping: true,
        playing: true,
    }));
    let mut camera = CookedSpatial::empty(id(2));
    camera.camera = Some(Camera {
        fov_degrees: 60.,
        near: 0.1,
        far: 100.,
        projection: incant_types::CameraProjection::Perspective {},
    });
    let mut map = CookedSpatial::empty(id(3));
    map.tilemap = Some(Box::new(NativeTilemap {
        tileset: id(90),
        size: [2, 2],
        cell_size: [0.25, 0.5],
        layers: vec![NativeTilemapLayer {
            id: id(80),
            cells: vec![Some(id(81)), None, Some(id(82)), Some(id(81))],
            layer: -2,
            order: 4,
            solid: false,
        }],
    }));
    let mut floor = CookedSpatial::empty(id(4));
    floor.mesh = Some(NativeMeshRenderer {
        mesh: id(91),
        cast_shadows: true,
    });
    let mut entities: Vec<_> = (1..=5).map(entity).collect();
    entities[0].velocity = Some(Velocity {
        linear: [0.125, 0., 0.],
    });
    entities[1].parent = Some(id(1));
    CookedScene::new(id(0), 60, entities)
        .unwrap()
        .with_spatial(assets.clone(), vec![actor, camera, map, floor])
        .unwrap()
        .with_navigation(vec![CookedNavigation {
            entity: id(5),
            grid: Some(NavigationGrid {
                dimensions: [3, 2],
                costs: vec![1, 0, 1, 1, 1, 1],
            }),
            mesh: Some(NativeNavigationMesh {
                settings: NavigationSettings {
                    min: [-4., -1., -4.],
                    max: [4., 3., 4.],
                    cell_size: 0.25,
                    cell_height: 0.1,
                    tile_cells: 16,
                    agent_radius: 0.25,
                    agent_height: 1.,
                    max_climb: 0.3,
                    max_slope_degrees: 45.,
                },
                sources: vec![NativeNavigationSource {
                    entity: id(4),
                    geometry: NativeNavigationGeometry::Mesh {
                        asset: assets[1],
                        geometry: Arc::new(NavigationGeometry {
                            vertices: vec![
                                [-4., 0., -4.],
                                [4., 0., -4.],
                                [4., 0., 4.],
                                [-4., 0., 4.],
                            ],
                            triangles: vec![[0, 2, 1], [0, 3, 2]],
                        }),
                    },
                }],
                links: vec![],
            }),
        }])
        .unwrap()
}
fn grid() -> GridPathRequest {
    GridPathRequest {
        start: [0, 0],
        end: [2, 0],
        diagonal: false,
        max_expansions: 100,
    }
}
fn path() -> PathRequest {
    PathRequest {
        start: [-2., 0., 0.],
        end: [2., 0., 0.],
        snap_distance: 1.,
        max_visited: 1000,
    }
}
fn command(world: &mut NativeWorld, value: StructuralCommand) {
    let mut commands = FrameCommands::default();
    commands.push(value).unwrap();
    world.apply(&mut commands).unwrap();
}
fn control(world: &mut NativeWorld, control: FlipbookControl) {
    command(
        world,
        StructuralCommand::ControlFlipbook {
            entity: id(1),
            control,
        },
    );
}
fn step(world: &mut NativeWorld, count: u64) {
    for _ in 0..count {
        // Absolute tick controls straddle the checkpoint with a nonzero fraction.
        match world.tick() {
            5 => control(world, FlipbookControl::Pause),
            9 => control(world, FlipbookControl::Play),
            _ => (),
        }
        world.step().unwrap();
    }
}
fn rehash(bytes: &mut [u8]) {
    let hash = Sha256::digest([&bytes[..32], &bytes[64..]].concat());
    bytes[32..64].copy_from_slice(&hash);
}
fn same(a: &NativeWorld, b: &NativeWorld) {
    assert_eq!(a.snapshot(), b.snapshot());
    assert_eq!(a.tick(), b.tick());
    assert_eq!(a.source_scene_fingerprint(), b.source_scene_fingerprint());
    assert_eq!(a.flipbook_state(id(1)), b.flipbook_state(id(1)));
    assert_eq!(
        a.grid_path(id(5), &grid()).unwrap().unwrap().cells,
        b.grid_path(id(5), &grid()).unwrap().unwrap().cells
    );
    assert_eq!(
        a.navigation_path(id(5), &path()).unwrap().unwrap().points,
        b.navigation_path(id(5), &path()).unwrap().unwrap().points
    );
    assert_eq!(
        a.spatial_view().unwrap().assets(),
        b.spatial_view().unwrap().assets()
    );
    assert_eq!(
        a.spatial_view()
            .unwrap()
            .nodes()
            .map(|n| n.declaration.cloned())
            .collect::<Vec<_>>(),
        b.spatial_view()
            .unwrap()
            .nodes()
            .map(|n| n.declaration.cloned())
            .collect::<Vec<_>>()
    );
    assert_eq!(a.save_bytes().unwrap(), b.save_bytes().unwrap());
}
#[test]
fn exact_fraction_spatial_navigation_and_original_binding_survive_continuation() {
    let mut world = NativeWorld::new(fixture());
    let origin = world.source_scene_fingerprint();
    step(&mut world, 17);
    let saved = world.save_bytes().unwrap();
    assert_eq!(&saved[8..12], &3u32.to_le_bytes());
    let mut restored = NativeWorld::from_save_bytes(&saved).unwrap();
    same(&world, &restored);
    assert_eq!(restored.source_scene_fingerprint(), origin);
    assert!(
        restored
            .flipbook_state(id(1))
            .unwrap()
            .checkpoint()
            .remainder
            > 0
    );
    for _ in 0..137 {
        step(&mut world, 1);
        step(&mut restored, 1);
        same(&world, &restored);
    }
}
#[test]
fn paused_and_completed_once_cursors_do_not_restart() {
    let mut world = NativeWorld::new(fixture());
    step(&mut world, 3);
    control(&mut world, FlipbookControl::Pause);
    let mut other = NativeWorld::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
    let state = other.flipbook_state(id(1)).unwrap();
    assert!(state.checkpoint().remainder > 0);
    for _ in 0..20 {
        world.step().unwrap();
        other.step().unwrap();
    }
    same(&world, &other);
    assert_eq!(other.flipbook_state(id(1)), Some(state));
    let mut node = world
        .spatial_view()
        .unwrap()
        .nodes()
        .find(|n| n.id == id(1))
        .unwrap()
        .declaration
        .unwrap()
        .clone();
    let animation = Arc::make_mut(node.flipbook.as_mut().unwrap());
    animation.looping = false;
    animation.playing = true;
    command(
        &mut world,
        StructuralCommand::SetSpatial {
            entity: id(1),
            spatial: Some(node),
        },
    );
    for _ in 0..40 {
        world.step().unwrap();
    }
    assert!(world.flipbook_state(id(1)).unwrap().completed());
    let mut other = NativeWorld::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
    for _ in 0..10 {
        world.step().unwrap();
        other.step().unwrap();
    }
    same(&world, &other);
}
#[test]
fn restore_invalidates_old_incarnation_paths_and_retained_indices_atomically() {
    let mut world = NativeWorld::new(fixture());
    step(&mut world, 17);
    let saved = world.save_bytes().unwrap();
    let identity = world.spatial_view().unwrap().identity().clone();
    let incarnation = world.incarnation();
    let token = world.entity_identity(id(1));
    let old = world.navigation_path(id(5), &path()).unwrap().unwrap();
    let old_grid = world.grid_path(id(5), &grid()).unwrap().unwrap();
    let gens = (
        world.spatial_view().unwrap().topology_generation(),
        world.spatial_view().unwrap().binding_generation(),
    );
    command(
        &mut world,
        StructuralCommand::SetParent {
            entity: id(2),
            parent: None,
        },
    );
    step(&mut world, 8);
    world.restore_bytes(&saved).unwrap();
    assert_eq!(world.spatial_view().unwrap().identity(), &identity);
    assert_ne!(world.incarnation(), incarnation);
    assert_ne!(world.entity_identity(id(1)), token);
    assert!(
        world
            .navigation_path(id(5), &path())
            .unwrap()
            .unwrap()
            .generation
            > old.generation
    );
    assert!(world.grid_path(id(5), &grid()).unwrap().unwrap().generation > old_grid.generation);
    assert!(world.spatial_view().unwrap().topology_generation() > gens.0);
    assert!(world.spatial_view().unwrap().binding_generation() > gens.1);
    assert_eq!(world.save_bytes().unwrap(), saved);
    // A different original scene with the same public ID/rate is not this binding.
    let foreign = NativeWorld::new(CookedScene::new(id(0), 60, vec![entity(20)]).unwrap())
        .save_bytes()
        .unwrap();
    assert_eq!(world.restore_bytes(&foreign), Err(SaveError::WorldMismatch));
    assert_eq!(world.save_bytes().unwrap(), saved);
}
#[test]
fn dirty_navigation_refuses_then_commits_without_mutating_failed_save() {
    let mut world = NativeWorld::new(fixture());
    world.with_numeric_columns(|columns| {
        columns.for_each_chunk(|chunk| {
            for (row, e) in chunk.ids.iter().enumerate() {
                if *e == id(4) {
                    chunk.transforms[row * TRANSFORM_STRIDE] = 0.25;
                }
            }
        })
    });
    let before = world.snapshot();
    assert_eq!(world.save_bytes(), Err(SaveError::UncommittedNavigation));
    assert_eq!(world.snapshot(), before);
    world.apply(&mut FrameCommands::default()).unwrap();
    let restored = NativeWorld::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
    same(&world, &restored);
}
#[test]
fn queued_rejected_and_removed_state_are_never_implicitly_published() {
    let mut world = NativeWorld::new(fixture());
    let before = world.save_bytes().unwrap();
    let mut pending = FrameCommands::default();
    pending.push(StructuralCommand::Despawn(id(1))).unwrap();
    assert_eq!(world.save_bytes().unwrap(), before);
    assert_eq!(pending.len(), 1);
    // Queue remains the caller's responsibility; it is not replayed on restore.
    let loaded = NativeWorld::from_save_bytes(&before).unwrap();
    assert!(loaded.inspect(id(1)).is_some());
    assert!(world.apply(&mut pending).is_err());
    assert_eq!(world.save_bytes().unwrap(), before);
    command(
        &mut world,
        StructuralCommand::SetParent {
            entity: id(2),
            parent: None,
        },
    );
    command(&mut world, StructuralCommand::Despawn(id(1)));
    command(&mut world, StructuralCommand::Spawn(entity(1)));
    let saved = world.save_bytes().unwrap();
    let mut loaded = NativeWorld::from_save_bytes(&saved).unwrap();
    assert!(loaded.flipbook_state(id(1)).is_none());
    assert_eq!(loaded.save_bytes().unwrap(), saved);
    loaded.step().unwrap();
}
#[test]
fn checksum_valid_bad_sections_and_cursor_states_leave_live_state_untouched() {
    let mut world = NativeWorld::new(fixture());
    step(&mut world, 17);
    let saved = world.save_bytes().unwrap();
    let row = saved.len() - 28;
    let mut cases = Vec::new();
    for (offset, bytes) in [
        (96, u32::MAX.to_le_bytes().to_vec()),
        (100, u32::MAX.to_le_bytes().to_vec()),
        (row + 16, 7u16.to_le_bytes().to_vec()),
        (row + 18, vec![2]),
        (row + 19, vec![1]),
        (row + 20, u64::MAX.to_le_bytes().to_vec()),
        (row, id(3).0.to_vec()),
    ] {
        let mut b = saved.clone();
        b[offset..offset + bytes.len()].copy_from_slice(&bytes);
        cases.push(b);
    }
    let mut missing = saved[..row].to_vec();
    missing[100..104].copy_from_slice(&0u32.to_le_bytes());
    cases.push(missing);
    let mut duplicate = saved.clone();
    duplicate.extend(&saved[row..]);
    duplicate[100..104].copy_from_slice(&3u32.to_le_bytes());
    cases.push(duplicate);
    for mut bytes in cases {
        let size = bytes.len() - 64;
        bytes[24..32].copy_from_slice(&(size as u64).to_le_bytes());
        rehash(&mut bytes);
        assert!(world.restore_bytes(&bytes).is_err());
        assert_eq!(world.save_bytes().unwrap(), saved);
    }
    for end in 0..saved.len() {
        assert!(world.restore_bytes(&saved[..end]).is_err());
    }
    assert_eq!(world.save_bytes().unwrap(), saved);
}
#[test]
fn legacy_plain_saves_load_but_never_smuggle_spatial_or_solver_state() {
    fn legacy(scene: CookedScene) -> Vec<u8> {
        let payload = scene.to_bytes();
        let mut bytes = vec![0; 64];
        bytes[..8].copy_from_slice(b"INCANTSV");
        bytes[8..12].copy_from_slice(&1u32.to_le_bytes());
        bytes[16..24].copy_from_slice(&21u64.to_le_bytes());
        bytes[24..32].copy_from_slice(&(payload.len() as u64).to_le_bytes());
        bytes.extend(payload);
        rehash(&mut bytes);
        bytes
    }
    let plain = CookedScene::new(id(0), 60, vec![entity(1)]).unwrap();
    let legacy_plain = legacy(plain);
    let world = NativeWorld::from_save_bytes(&legacy_plain).unwrap();
    let mut extended = NativeWorld::new(fixture());
    let before = extended.save_bytes().unwrap();
    assert_eq!(
        extended.restore_bytes(&legacy_plain),
        Err(SaveError::NavigationUnsupported)
    );
    assert_eq!(extended.save_bytes().unwrap(), before);
    assert_eq!(world.tick(), 21);
    assert_eq!(&world.save_bytes().unwrap()[8..12], &3u32.to_le_bytes());
    assert!(matches!(
        NativeWorld::from_save_bytes(&legacy(fixture())),
        Err(SaveError::NavigationUnsupported)
    ));
}

// This worker is invoked by the following test in separate OS processes. Only
// checked binary scene/save data crosses the boundary; no authoring/cache input.
#[test]
fn retained_save_process_worker() {
    let Some(input) = std::env::var_os("INCANT_TEST_SAVE_INPUT") else {
        return;
    };
    let bytes = std::fs::read(&input).unwrap();
    let mut world = if std::path::Path::new(&input)
        .extension()
        .is_some_and(|v| v == "scene")
    {
        NativeWorld::from_bytes(&bytes).unwrap()
    } else {
        NativeWorld::from_save_bytes(&bytes).unwrap()
    };
    let ticks = std::env::var("INCANT_TEST_SAVE_TICKS")
        .unwrap()
        .parse()
        .unwrap();
    step(&mut world, ticks);
    assert_eq!(
        world.grid_path(id(5), &grid()).unwrap().unwrap().cells,
        vec![[0, 0], [0, 1], [1, 1], [2, 1], [2, 0]]
    );
    assert!(world.navigation_path(id(5), &path()).unwrap().is_some());
    let output = std::env::var_os("INCANT_TEST_SAVE_OUTPUT").unwrap();
    let mut file = std::fs::File::options()
        .write(true)
        .create_new(true)
        .open(output)
        .unwrap();
    std::io::Write::write_all(&mut file, &world.save_bytes().unwrap()).unwrap();
}
#[test]
fn fresh_process_resume_matches_uninterrupted_after_deleting_the_binary_scene() {
    let path = std::env::temp_dir().join(format!(
        "incant-retained-save-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&path).unwrap();
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(path.clone());
    let scene = path.join("input.scene");
    std::fs::write(&scene, fixture().to_bytes()).unwrap();
    let run = |input: &std::path::Path, ticks: u64, name: &str| {
        let output = path.join(name);
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "retained_save_process_worker",
                "--test-threads=1",
            ])
            .env("INCANT_TEST_SAVE_INPUT", input)
            .env("INCANT_TEST_SAVE_TICKS", ticks.to_string())
            .env("INCANT_TEST_SAVE_OUTPUT", &output)
            .current_dir(&path)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{} {}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        std::fs::read(output).unwrap()
    };
    let baseline = run(&scene, 154, "baseline.save");
    let partial = run(&scene, 17, "partial.save");
    std::fs::remove_file(scene).unwrap();
    assert_eq!(
        run(&path.join("partial.save"), 137, "resumed.save"),
        baseline
    );
    assert_eq!(
        run(&path.join("partial.save"), 137, "repeated.save"),
        baseline
    );
    assert_eq!(
        run(&path.join("partial.save"), 0, "unchanged.save"),
        partial
    );
    assert_eq!(std::fs::read(path.join("partial.save")).unwrap(), partial);
}

#[test]
fn valid_v2_solid_tilemap_cannot_reconstruct_an_uncheckpointed_solver() {
    let scene = fixture();
    let mut nodes = scene.spatial().to_vec();
    nodes
        .iter_mut()
        .find(|n| n.entity == id(3))
        .unwrap()
        .tilemap
        .as_mut()
        .unwrap()
        .layers[0]
        .solid = true;
    let solid = CookedScene::new(scene.id(), scene.tick_rate(), scene.entities().to_vec())
        .unwrap()
        .with_spatial(scene.visual_assets().to_vec(), nodes)
        .unwrap()
        .with_navigation(scene.navigation().to_vec())
        .unwrap();
    let mut world = NativeWorld::new(fixture());
    step(&mut world, 17);
    let good = world.save_bytes().unwrap();
    let old_scene_len = u32::from_le_bytes(good[96..100].try_into().unwrap()) as usize;
    let scene_bytes = solid.to_bytes();
    let mut bad = good[..104].to_vec();
    bad[96..100].copy_from_slice(&(scene_bytes.len() as u32).to_le_bytes());
    bad.extend(scene_bytes);
    bad.extend(&good[104 + old_scene_len..]);
    let size = bad.len() - 64;
    bad[24..32].copy_from_slice(&(size as u64).to_le_bytes());
    rehash(&mut bad);
    assert_eq!(
        world.restore_bytes(&bad),
        Err(SaveError::PhysicsUnsupported)
    );
    assert_eq!(world.save_bytes().unwrap(), good);
    assert_eq!(
        NativeWorld::new(solid).save_bytes(),
        Err(SaveError::PhysicsUnsupported)
    );
}
