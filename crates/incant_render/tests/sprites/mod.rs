//! Numerical sprite contracts; these fixtures do not approve appearance.
mod ties;
use incant_assets::{AssetStore, cook_texture};
use incant_doc::{
    Asset, AssetImportSettings, Camera, CameraProjection, Entity, Project, Scene, SpriteMode,
    SpriteRenderer, SpriteSampling, TextureUsage, Transform,
};
use incant_render::{RenderScene, Renderer};
use serde_json::json;
use std::path::Path;
struct Fixture {
    root: tempfile::TempDir,
    project: Project,
    assets: AssetStore,
    scene: String,
    camera: String,
    sprite: String,
    texture: String,
}
impl Fixture {
    fn new(pixels: &[u8], w: u32, h: u32) -> Self {
        let root = tempfile::tempdir().unwrap();
        let mut project = Project::empty("Sprite numerical fixture");
        let mut scene = Scene::new("Main");
        let mut camera = Entity::new("Camera");
        camera.components.insert(
            "Transform".into(),
            json!(Transform {
                translation: [0., 0., 8.],
                ..Default::default()
            }),
        );
        camera.components.insert(
            "Camera".into(),
            json!(Camera {
                fov_degrees: 60.,
                near: 0.1,
                far: 100.,
                projection: CameraProjection::PixelPerfect {
                    pixels_per_unit: 8.,
                    reference_resolution: [64, 64]
                }
            }),
        );
        let camera_id = camera.id.clone();
        scene.entities.insert(camera.id.clone(), camera);
        let texture = incant_doc::new_id();
        let mut sprite = Entity::new("Sprite");
        sprite
            .components
            .insert("Transform".into(), json!(Transform::default()));
        sprite.components.insert(
            "SpriteRenderer".into(),
            json!(SpriteRenderer {
                texture: texture.clone(),
                region: [0, 0, w, h],
                size: [4., 4.],
                pivot: [0.5, 0.5],
                tint: [1.; 4],
                flip_x: false,
                flip_y: false,
                layer: 0,
                order: 0,
                sampling: SpriteSampling::Nearest,
                mode: SpriteMode::Simple
            }),
        );
        let sprite_id = sprite.id.clone();
        scene.entities.insert(sprite.id.clone(), sprite);
        let sid = scene.id.clone();
        project.scenes.insert(sid.clone(), scene);
        let mut f = Self {
            root,
            project,
            assets: AssetStore::default(),
            scene: sid,
            camera: camera_id,
            sprite: sprite_id,
            texture,
        };
        f.cook(pixels, w, h);
        f
    }
    fn cook(&mut self, pixels: &[u8], w: u32, h: u32) {
        crate::materials::write_png(&self.root.path().join("sprite.png"), pixels, w, h);
        let cooked = cook_texture(
            self.root.path(),
            Path::new("sprite.png"),
            &self.root.path().join(".incant/cache/textures"),
            TextureUsage::Color,
        )
        .unwrap();
        self.project.assets.insert(
            self.texture.clone(),
            Asset {
                id: self.texture.clone(),
                name: "Sprite".into(),
                path: "sprite.png".into(),
                kind: "texture".into(),
                sha256: cooked.metadata.fingerprint,
                import_settings: Some(AssetImportSettings::Texture {
                    usage: TextureUsage::Color,
                }),
            },
        );
        self.assets
            .sync_project(&self.project, self.root.path())
            .unwrap();
    }
    fn edit(&mut self, key: &str, value: serde_json::Value) {
        self.project
            .scenes
            .get_mut(&self.scene)
            .unwrap()
            .entities
            .get_mut(&self.sprite)
            .unwrap()
            .components
            .get_mut("SpriteRenderer")
            .unwrap()[key] = value;
    }
    fn scene(&self, r: &Renderer) -> RenderScene {
        r.prepare_scene(&self.project, &self.assets)
            .unwrap()
            .with_camera(&self.camera)
            .unwrap()
    }
    fn pixels(&self, r: &Renderer) -> Vec<[u8; 4]> {
        decode(&r.screenshot_scene_png(&self.scene(r), 64, 64).unwrap())
    }
}
fn decode(png: &[u8]) -> Vec<[u8; 4]> {
    let mut reader = png::Decoder::new(std::io::Cursor::new(png))
        .read_info()
        .unwrap();
    let mut bytes = vec![0; reader.output_buffer_size().unwrap()];
    reader.next_frame(&mut bytes).unwrap();
    bytes.as_chunks::<4>().0.to_vec()
}
fn is_red(p: [u8; 4]) -> bool {
    p[0] > 200 && p[1] < 50 && p[2] < 50
}
fn is_green(p: [u8; 4]) -> bool {
    p[1] > 200 && p[0] < 50 && p[2] < 50
}
fn is_blue(p: [u8; 4]) -> bool {
    p[2] > 200 && p[0] < 50 && p[1] < 50
}

#[test]
#[ignore = "requires a native GPU; desktop workflows execute this"]
fn sprite_subregions_flips_and_retained_reimports_preserve_exact_pixels() {
    let r = Renderer::headless().unwrap();
    let mut f = Fixture::new(
        &[
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
        ],
        2,
        2,
    );
    let retained = f.scene(&r);
    assert_eq!(retained.stats().sprite_entities, 1);
    assert_eq!(retained.stats().diagnostic_entities, 0);
    let base = f.pixels(&r);
    assert!(is_red(base[20 * 64 + 20]));
    assert!(is_green(base[20 * 64 + 44]));
    assert!(is_blue(base[44 * 64 + 20]));
    f.edit("flip_x", json!(true));
    let horizontal = f.pixels(&r);
    for y in 0..64 {
        for x in 0..64 {
            assert_eq!(horizontal[y * 64 + x], base[y * 64 + 63 - x]);
        }
    }
    f.edit("flip_y", json!(true));
    let both = f.pixels(&r);
    for y in 0..64 {
        for x in 0..64 {
            assert_eq!(both[y * 64 + x], base[(63 - y) * 64 + 63 - x]);
        }
    }
    f.edit("region", json!([1, 0, 1, 1]));
    let crop = f.pixels(&r);
    assert!(is_green(crop[32 * 64 + 32]));
    assert!(crop.iter().all(|p| is_green(*p) || *p == [20, 21, 25, 255]));
    f.edit("region", json!([0, 0, 2, 2]));
    f.edit("flip_x", json!(false));
    f.edit("flip_y", json!(false));
    f.cook(&[0, 0, 255, 255].repeat(4), 2, 2);
    let updated = f.pixels(&r);
    assert!(is_blue(updated[32 * 64 + 32]));
    assert_eq!(
        decode(&r.screenshot_scene_png(&retained, 64, 64).unwrap()),
        base
    );
    std::fs::remove_file(f.root.path().join("sprite.png")).unwrap();
    let mut reload = AssetStore::default();
    reload.sync_project(&f.project, f.root.path()).unwrap();
    let from_cache = r
        .prepare_scene(&f.project, &reload)
        .unwrap()
        .with_camera(&f.camera)
        .unwrap();
    assert_eq!(
        decode(&r.screenshot_scene_png(&from_cache, 64, 64).unwrap()),
        updated
    );
    f.edit("region", json!([1, 0, 2, 2]));
    assert!(
        r.prepare_scene(&f.project, &f.assets)
            .err()
            .unwrap()
            .to_string()
            .contains("source region")
    );
    assert_eq!(
        decode(&r.screenshot_scene_png(&retained, 64, 64).unwrap()),
        base
    );
}
#[test]
#[ignore = "requires a native GPU; desktop workflows execute this"]
fn sprite_layers_order_and_alpha_follow_painter_semantics() {
    let r = Renderer::headless().unwrap();
    let mut f = Fixture::new(&[255; 4], 1, 1);
    f.edit("tint", json!([1, 0, 0, 1]));
    let mut top = f.project.scenes[&f.scene].entities[&f.sprite].clone();
    top.id = incant_doc::new_id();
    top.components.get_mut("SpriteRenderer").unwrap()["tint"] = json!([0, 0, 1, 1]);
    top.components.get_mut("SpriteRenderer").unwrap()["layer"] = json!(1);
    let top_id = top.id.clone();
    f.project
        .scenes
        .get_mut(&f.scene)
        .unwrap()
        .entities
        .insert(top.id.clone(), top);
    assert!(is_blue(f.pixels(&r)[32 * 64 + 32]));
    f.edit("layer", json!(2));
    assert!(is_red(f.pixels(&r)[32 * 64 + 32]));
    f.edit("layer", json!(1));
    f.edit("order", json!(1));
    assert!(is_red(f.pixels(&r)[32 * 64 + 32]));
    f.edit("tint", json!([1, 0, 0, 0]));
    assert!(is_blue(f.pixels(&r)[32 * 64 + 32]));
    f.edit("order", json!(0));
    f.edit("tint", json!([1, 0, 0, 1]));
    let p = f.pixels(&r)[32 * 64 + 32];
    assert_eq!(is_red(p), f.sprite > top_id);
    let stable = f.pixels(&r);
    assert_eq!(stable, f.pixels(&r));
}
#[test]
#[ignore = "requires a native GPU; desktop workflows execute this"]
fn nine_slice_preserves_pixel_borders_and_linear_sampling_has_no_hidden_color_fringe() {
    let r = Renderer::headless().unwrap();
    let mut pixels = Vec::new();
    for y in 0..3 {
        for x in 0..3 {
            pixels.extend(if x == 1 && y == 1 {
                [0, 255, 0, 255]
            } else {
                [255, 0, 0, 255]
            });
        }
    }
    let mut f = Fixture::new(&pixels, 3, 3);
    f.edit(
        "mode",
        json!({"kind":"nine_slice","border":[1,1,1,1],"pixels_per_unit":8}),
    );
    let image = f.pixels(&r);
    for y in 16..48 {
        for x in 16..48 {
            let p = image[y * 64 + x];
            if x == 16 || x == 47 || y == 16 || y == 47 {
                assert!(is_red(p), "{x},{y}: {p:?}");
            } else {
                assert!(is_green(p), "{x},{y}: {p:?}");
            }
        }
    }
    f.edit("mode", json!({"kind":"simple"}));
    f.edit("region", json!([0, 0, 2, 1]));
    f.edit("sampling", json!("linear"));
    f.cook(&[255, 0, 0, 0, 0, 0, 255, 255], 2, 1);
    let image = f.pixels(&r);
    assert!(
        image.iter().all(|p| p[0] <= 31),
        "hidden red leaked into filtered alpha"
    );
    assert!(is_blue(image[32 * 64 + 46]));
}

#[test]
#[ignore = "requires a native GPU; desktop workflows execute this"]
fn inherited_sprite_transforms_and_queued_reimports_survive_scene_drop() {
    let r = Renderer::headless().unwrap();
    let mut f = Fixture::new(&[255, 0, 0, 255], 1, 1);
    let baseline = f.pixels(&r);
    let mut parent = Entity::new("Parent");
    parent.components.insert(
        "Transform".into(),
        json!(Transform {
            translation: [1., 0., 0.],
            scale: [-1., 1., 1.],
            ..Default::default()
        }),
    );
    f.project
        .scenes
        .get_mut(&f.scene)
        .unwrap()
        .entities
        .get_mut(&f.sprite)
        .unwrap()
        .parent = Some(parent.id.clone());
    f.project
        .scenes
        .get_mut(&f.scene)
        .unwrap()
        .entities
        .insert(parent.id.clone(), parent);
    let inherited = f.pixels(&r);
    for y in 0..64 {
        for x in 8..64 {
            assert_eq!(inherited[y * 64 + x], baseline[y * 64 + x - 8]);
        }
    }
    let format = incant_render::wgpu::TextureFormat::Rgba8UnormSrgb;
    let mut jobs = Vec::new();
    for (i, color) in [[255, 0, 0, 255], [0, 0, 255, 255], [0, 255, 0, 255]]
        .into_iter()
        .enumerate()
    {
        f.cook(&color, 1, 1);
        let scene = f.scene(&r);
        let width = 64 * (i as u32 + 1);
        let expected = decode(&r.screenshot_scene_png(&scene, width, 64).unwrap());
        let target = r
            .device
            .create_texture(&incant_render::wgpu::TextureDescriptor {
                label: Some("Queued sprite output"),
                size: incant_render::wgpu::Extent3d {
                    width,
                    height: 64,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: incant_render::wgpu::TextureDimension::D2,
                format,
                usage: incant_render::wgpu::TextureUsages::RENDER_ATTACHMENT
                    | incant_render::wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
        let command = r
            .draw_scene(
                &scene,
                &target.create_view(&Default::default()),
                format,
                width,
                64,
                None,
            )
            .unwrap();
        jobs.push((target, command, width, expected));
    }
    drop(f);
    for (target, command, width, expected) in jobs.into_iter().rev() {
        r.queue.submit([command]);
        let buffer = r
            .device
            .create_buffer(&incant_render::wgpu::BufferDescriptor {
                label: Some("Sprite output readback"),
                size: u64::from(width) * 64 * 4,
                usage: incant_render::wgpu::BufferUsages::COPY_DST
                    | incant_render::wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
        let mut encoder = r.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            target.as_image_copy(),
            incant_render::wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: incant_render::wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(width * 4),
                    rows_per_image: Some(64),
                },
            },
            target.size(),
        );
        r.queue.submit([encoder.finish()]);
        let (send, receive) = std::sync::mpsc::channel();
        buffer.map_async(incant_render::wgpu::MapMode::Read, .., move |result| {
            send.send(result).unwrap()
        });
        r.device
            .poll(incant_render::wgpu::PollType::wait_indefinitely())
            .unwrap();
        receive.recv().unwrap().unwrap();
        let mapped = buffer.get_mapped_range(..);
        assert_eq!(mapped.as_chunks::<4>().0, expected);
    }
}
