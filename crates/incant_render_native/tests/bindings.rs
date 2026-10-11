mod common;
use common::*;
use incant_assets_runtime::{RuntimeAssetData, TextureUsage};
use incant_render_native::{NativeRenderError, NativeRenderInput, VisualCaches};
use incant_runtime::{
    CookedScene, CookedSpatial, NativeEnvironmentLight, NativeMeshRenderer, NativeWorld,
    VisualAssetKind, VisualAssetRef,
};
use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};
struct Fixture {
    directory: tempfile::TempDir,
    models: PathBuf,
    textures: PathBuf,
    model: VisualAssetRef,
    texture: VisualAssetRef,
}
impl Fixture {
    fn new(usage: TextureUsage) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        let source = root.join("source");
        fs::create_dir(&source).unwrap();
        let models = root.join("models");
        let textures = root.join("textures");
        let vertices: [f32; 9] = [0., 0., 0., 1., 0., 0., 0., 1., 0.];
        fs::write(
            source.join("tri.bin"),
            vertices
                .into_iter()
                .flat_map(f32::to_le_bytes)
                .collect::<Vec<_>>(),
        )
        .unwrap();
        fs::write(source.join("tri.gltf"),json!({"asset":{"version":"2.0"},"buffers":[{"byteLength":36,"uri":"tri.bin"}],"bufferViews":[{"buffer":0,"byteLength":36}],"accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","min":[0,0,0],"max":[1,1,0]}],"meshes":[{"primitives":[{"attributes":{"POSITION":0}}]}],"nodes":[{"mesh":0}],"scenes":[{"nodes":[0]}],"scene":0}).to_string()).unwrap();
        let image = fs::File::create(source.join("image.png")).unwrap();
        let mut encoder = png::Encoder::new(image, 1, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&[128, 128, 255, 255])
            .unwrap();
        let model = incant_assets::cook_gltf(&source, Path::new("tri.gltf"), &models).unwrap();
        let texture =
            incant_assets::cook_texture(&source, Path::new("image.png"), &textures, usage).unwrap();
        let reference = |n, kind, key: &str| VisualAssetRef {
            id: id(n),
            kind,
            fingerprint: std::array::from_fn(|i| {
                u8::from_str_radix(&key[i * 2..i * 2 + 2], 16).unwrap()
            }),
        };
        let model = reference(90, VisualAssetKind::Model, &model.metadata.fingerprint);
        let texture = reference(91, VisualAssetKind::Texture, &texture.metadata.fingerprint);
        fs::remove_dir_all(source).unwrap(); // All subsequent reads are cooked-only.
        Self {
            directory,
            models,
            textures,
            model,
            texture,
        }
    }
    fn world(&self) -> NativeWorld {
        let mut mesh = declaration(3);
        mesh.mesh = Some(NativeMeshRenderer {
            mesh: self.model.id,
            cast_shadows: true,
        });
        let mut environment = CookedSpatial::empty(id(4));
        environment.environment = Some(NativeEnvironmentLight {
            texture: self.texture.id,
            intensity: 2.,
            rotation_degrees: 30.,
        });
        let scene = CookedScene::new(
            id(99),
            60,
            (1..=4).map(|n| entity(n, (n == 2).then_some(1))).collect(),
        )
        .unwrap()
        .with_spatial(
            vec![self.model, self.texture],
            vec![declaration(2), mesh, environment],
        )
        .unwrap();
        NativeWorld::from_bytes(&scene.to_bytes()).unwrap()
    }
    fn caches(&self) -> VisualCaches<'_> {
        VisualCaches {
            models: &self.models,
            textures: &self.textures,
        }
    }
}
#[test]
fn source_free_assets_are_sealed_retained_and_not_reloaded_on_motion_or_rebuild() {
    let fixture = Fixture::new(TextureUsage::Color);
    let mut w = fixture.world();
    let mut input =
        NativeRenderInput::load(&w.spatial_view().unwrap(), fixture.caches(), id(2)).unwrap();
    let source = Arc::clone(
        &input
            .input()
            .nodes
            .iter()
            .find_map(|n| n.mesh.as_ref())
            .unwrap()
            .source,
    );
    let RuntimeAssetData::Model(model) = source.data() else {
        panic!("model")
    };
    assert_eq!(model.meshes[0].indices.len(), 3);
    assert_eq!(input.input().environment.as_ref().unwrap().intensity, 2.);
    let expected = source.info().payload_bytes
        + input
            .input()
            .environment
            .as_ref()
            .unwrap()
            .source
            .info()
            .payload_bytes;
    assert_eq!(input.payload_bytes(), expected);
    assert!(expected > 0);
    let count = Arc::strong_count(&source);
    let nodes = input.input().nodes.as_ptr();
    fixture.directory.close().unwrap(); // Even cooked files are no longer needed.
    for _ in 0..8 {
        w.step().unwrap();
        input
            .try_sync(&w.spatial_view().unwrap(), |_, _, _| Ok::<_, ()>(()))
            .unwrap();
        assert_eq!(Arc::strong_count(&source), count);
        assert_eq!(input.input().nodes.as_ptr(), nodes);
        assert!(Arc::ptr_eq(
            &source,
            &input
                .input()
                .nodes
                .iter()
                .find_map(|n| n.mesh.as_ref())
                .unwrap()
                .source
        ));
    }
    w.apply(&mut queue([incant_runtime::StructuralCommand::SetParent {
        entity: id(3),
        parent: Some(id(2)),
    }]))
    .unwrap();
    input
        .try_sync(&w.spatial_view().unwrap(), |_, _, _| Ok::<_, ()>(()))
        .unwrap();
    assert_eq!(Arc::strong_count(&source), count);
    assert!(
        input
            .input()
            .nodes
            .iter()
            .filter_map(|n| n.mesh.as_ref())
            .any(|m| Arc::ptr_eq(&m.source, &source))
    );
}

#[test]
fn startup_rejects_missing_stale_corrupt_wrong_kind_and_unused_bad_bindings() {
    let fixture = Fixture::new(TextureUsage::Color);
    let w = fixture.world();
    let valid =
        NativeRenderInput::load(&w.spatial_view().unwrap(), fixture.caches(), id(2)).unwrap();
    {
        let mut reference = fixture.model;
        reference.fingerprint[0] ^= 1;
        let w = NativeWorld::new(
            scene()
                .with_spatial(vec![reference], vec![declaration(2), declaration(3)])
                .unwrap(),
        );
        assert!(
            matches!(NativeRenderInput::load(&w.spatial_view().unwrap(),fixture.caches(),id(2)),Err(NativeRenderError::Asset{asset,..}) if asset==id(90))
        );
    }
    let mut wrong = fixture.model;
    wrong.kind = VisualAssetKind::Texture;
    let w = NativeWorld::new(
        scene()
            .with_spatial(vec![wrong], vec![declaration(2), declaration(3)])
            .unwrap(),
    );
    assert!(matches!(
        NativeRenderInput::load(&w.spatial_view().unwrap(), fixture.caches(), id(2)),
        Err(NativeRenderError::Asset { .. })
    ));
    let path = fixture.models.join(format!(
        "{}.incmodel",
        valid
            .input()
            .nodes
            .iter()
            .find_map(|n| n.mesh.as_ref())
            .unwrap()
            .source
            .info()
            .fingerprint
    ));
    fs::write(&path, b"corrupt").unwrap();
    assert!(matches!(
        NativeRenderInput::load(
            &fixture.world().spatial_view().unwrap(),
            fixture.caches(),
            id(2)
        ),
        Err(NativeRenderError::Asset { .. })
    ));
    fs::remove_file(path).unwrap();
    assert!(matches!(
        NativeRenderInput::load(
            &fixture.world().spatial_view().unwrap(),
            fixture.caches(),
            id(2)
        ),
        Err(NativeRenderError::Asset { .. })
    ));
    let RuntimeAssetData::Model(model) = valid
        .input()
        .nodes
        .iter()
        .find_map(|n| n.mesh.as_ref())
        .unwrap()
        .source
        .data()
    else {
        unreachable!()
    };
    assert_eq!(model.meshes[0].indices.len(), 3);
}

#[test]
fn environment_usage_is_checked_at_startup_and_a_bad_rebind_keeps_prior_input() {
    let fixture = Fixture::new(TextureUsage::Normal);
    let w = fixture.world();
    assert!(matches!(
        NativeRenderInput::load(&w.spatial_view().unwrap(), fixture.caches(), id(2)),
        Err(NativeRenderError::EnvironmentUsage)
    ));
    let mut w = NativeWorld::new(
        scene()
            .with_spatial(vec![fixture.texture], vec![declaration(2), declaration(3)])
            .unwrap(),
    );
    let mut input =
        NativeRenderInput::load(&w.spatial_view().unwrap(), fixture.caches(), id(2)).unwrap();
    let mut env = declaration(3);
    env.environment = Some(NativeEnvironmentLight {
        texture: fixture.texture.id,
        intensity: 1.,
        rotation_degrees: 0.,
    });
    w.apply(&mut queue([
        incant_runtime::StructuralCommand::SetSpatial {
            entity: id(3),
            spatial: Some(env),
        },
    ]))
    .unwrap();
    assert!(
        input
            .try_sync(&w.spatial_view().unwrap(), |_, _, _| -> Result<(), ()> {
                panic!("bad binding consumer")
            })
            .is_err()
    );
    assert!(input.input().environment.is_none());
    assert_eq!(input.binding_generation(), 0);
}

#[test]
fn authoring_cook_to_binary_world_to_input_needs_no_project_or_source_at_runtime() {
    use incant_doc::{Asset, Entity, Project, Scene, new_id};
    let fixture = Fixture::new(TextureUsage::Color);
    let key = fixture
        .model
        .fingerprint
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let mut project = Project::empty("Cooked native render proof");
    let mut scene = Scene::new("Main");
    let asset = new_id();
    project.assets.insert(
        asset.clone(),
        Asset {
            id: asset.clone(),
            name: "Model".into(),
            path: "absent/tri.gltf".into(),
            kind: "model".into(),
            sha256: key,
            import_settings: None,
        },
    );
    let mut entity = Entity::new("Camera and model");
    let camera = incant_cook::stable_id(&entity.id).unwrap();
    entity.components.insert(
        "Camera".into(),
        serde_json::to_value(common::camera()).unwrap(),
    );
    entity.components.insert(
        "MeshRenderer".into(),
        json!({"mesh":asset,"materials":[],"cast_shadows":true}),
    );
    scene.entities.insert(entity.id.clone(), entity);
    let sid = scene.id.clone();
    project.scenes.insert(sid.clone(), scene);
    let bytes = incant_cook::cook_scene(&project, &sid).unwrap();
    drop(project);
    let world = NativeWorld::from_bytes(&bytes).unwrap();
    let input =
        NativeRenderInput::load(&world.spatial_view().unwrap(), fixture.caches(), camera).unwrap();
    assert_eq!(input.input().nodes.len(), 1);
    assert_eq!(input.camera_id(), camera);
    assert!(input.input().nodes[0].mesh.is_some());
    fixture.directory.close().unwrap();
    assert!(input.input().nodes[0].camera.is_some());
}

#[test]
fn native_sprite_binding_retains_color_texture_and_rejects_unavailable_geometry_atomically() {
    use incant_runtime::{
        FrameCommands, NativeSpriteRegion, NativeSpriteRenderer, SpriteMode, SpriteSampling,
        StructuralCommand,
    };
    let fixture = Fixture::new(TextureUsage::Color);
    let mut node = CookedSpatial::empty(id(3));
    node.sprite = Some(Box::new(NativeSpriteRenderer {
        texture: fixture.texture.id,
        region: NativeSpriteRegion::Pixels([0, 0, 1, 1]),
        size: [2., 2.],
        pivot: [0.5; 2],
        tint: [1.; 4],
        flip_x: false,
        flip_y: true,
        layer: -2,
        order: 9,
        sampling: SpriteSampling::Nearest,
        mode: SpriteMode::Simple,
    }));
    let mut world = NativeWorld::new(
        scene()
            .with_spatial(vec![fixture.texture], vec![declaration(2), node.clone()])
            .unwrap(),
    );
    let mut input =
        NativeRenderInput::load(&world.spatial_view().unwrap(), fixture.caches(), id(2)).unwrap();
    let before = Arc::clone(
        &input
            .input()
            .nodes
            .iter()
            .find_map(|n| n.sprite.as_ref())
            .unwrap()
            .source,
    );
    fs::remove_dir_all(&fixture.textures).unwrap();
    for _ in 0..4 {
        world.step().unwrap();
        input
            .try_sync(&world.spatial_view().unwrap(), |_, _, _| Ok::<_, ()>(()))
            .unwrap();
    }
    let s = input
        .input()
        .nodes
        .iter()
        .find_map(|n| n.sprite.as_ref())
        .unwrap();
    assert!(Arc::ptr_eq(&before, &s.source));
    assert!(s.sprite.flip_y);
    assert_eq!(s.sprite.layer, -2);
    for region in [
        NativeSpriteRegion::Pixels([0, 0, 2, 2]),
        NativeSpriteRegion::Atlas { frame: id(100) },
    ] {
        node.sprite.as_mut().unwrap().region = region;
        let mut commands = FrameCommands::default();
        commands
            .push(StructuralCommand::SetSpatial {
                entity: node.entity,
                spatial: Some(node.clone()),
            })
            .unwrap();
        world.apply(&mut commands).unwrap();
        let generation = input.binding_generation();
        assert!(
            input
                .try_sync(&world.spatial_view().unwrap(), |_, _, _| Ok::<_, ()>(()))
                .is_err()
        );
        assert_eq!(input.binding_generation(), generation);
        assert_eq!(
            input
                .input()
                .nodes
                .iter()
                .find_map(|n| n.sprite.as_ref())
                .unwrap()
                .sprite
                .region
                .pixels(),
            Some([0, 0, 1, 1])
        );
    }
}
