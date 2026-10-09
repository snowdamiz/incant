//! Numerical renderer fixtures, not game art or visual approval.
use incant_assets::{AssetStore, cook_gltf};
use incant_doc::{Asset, Entity, MeshRenderer, Project, Scene, Transform, new_id};
use incant_render::{RenderScene, Renderer};
use serde_json::{Value, json};
use std::{fs, path::Path};

pub struct Fixture {
    pub root: tempfile::TempDir,
    pub project: Project,
    pub assets: AssetStore,
    pub asset_id: String,
}
impl Fixture {
    pub fn new(material: Value) -> Self {
        let root = tempfile::tempdir().unwrap();
        // A quad centered on the camera target, with consistent +Z winding/UVs.
        let positions: [f32; 18] = [
            -3., -3., 0., 3., -3., 0., 3., 3., 0., -3., -3., 0., 3., 3., 0., -3., 3., 0.,
        ];
        let uv: [f32; 12] = [0., 1., 1., 1., 1., 0., 0., 1., 1., 0., 0., 0.];
        let buffer: Vec<_> = positions
            .into_iter()
            .chain(uv)
            .flat_map(f32::to_le_bytes)
            .collect();
        fs::write(root.path().join("quad.bin"), buffer).unwrap();
        let document = json!({
            "asset":{"version":"2.0"}, "buffers":[{"uri":"quad.bin","byteLength":120}],
            "bufferViews":[{"buffer":0,"byteLength":72},{"buffer":0,"byteOffset":72,"byteLength":48}],
            "accessors":[{"bufferView":0,"componentType":5126,"count":6,"type":"VEC3","min":[-3,-3,0],"max":[3,3,0]},
                {"bufferView":1,"componentType":5126,"count":6,"type":"VEC2"}],
            "meshes":[{"primitives":[{"attributes":{"POSITION":0,"TEXCOORD_0":1},"material":0}]}],
            "materials":[material], "nodes":[{"mesh":0}], "scenes":[{"nodes":[0]}],"scene":0
        });
        fs::write(
            root.path().join("quad.gltf"),
            serde_json::to_vec(&document).unwrap(),
        )
        .unwrap();
        let mut fixture = Self {
            root,
            project: Project::empty("Materials"),
            assets: AssetStore::default(),
            asset_id: new_id(),
        };
        let mut scene = Scene::new("Scene");
        let mut entity = Entity::new("Quad");
        entity
            .components
            .insert("Transform".into(), json!(Transform::default()));
        entity.components.insert(
            "MeshRenderer".into(),
            json!(MeshRenderer {
                mesh: fixture.asset_id.clone(),
                materials: vec![],
                cast_shadows: true
            }),
        );
        scene.entities.insert(entity.id.clone(), entity);
        fixture.project.scenes.insert(scene.id.clone(), scene);
        fixture.cook();
        fixture
    }
    pub fn edit(&mut self, change: impl FnOnce(&mut Value)) {
        let path = self.root.path().join("quad.gltf");
        let mut value: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        change(&mut value);
        fs::write(path, serde_json::to_vec(&value).unwrap()).unwrap();
        self.cook();
    }
    pub fn normal(&mut self, normal: glam::Vec3) {
        let path = self.root.path().join("quad.bin");
        let mut bytes = fs::read(&path).unwrap();
        bytes.truncate(120);
        for _ in 0..6 {
            for value in normal.normalize().to_array() {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
        fs::write(path, bytes).unwrap();
        self.edit(|g| {
            g["buffers"][0]["byteLength"] = json!(192);
            if g["bufferViews"].as_array().unwrap().len() == 2 {
                g["bufferViews"].as_array_mut().unwrap().push(Value::Null);
            }
            if g["accessors"].as_array().unwrap().len() == 2 {
                g["accessors"].as_array_mut().unwrap().push(Value::Null);
            }
            g["bufferViews"][2] = json!({"buffer":0,"byteOffset":120,"byteLength":72});
            g["accessors"][2] =
                json!({"bufferView":2,"componentType":5126,"count":6,"type":"VEC3"});
            g["meshes"][0]["primitives"][0]["attributes"]["NORMAL"] = json!(2);
        });
    }
    pub fn texture(
        &mut self,
        pixels: &[u8],
        width: u32,
        height: u32,
        change: impl FnOnce(&mut Value),
    ) {
        write_png(&self.root.path().join("map.png"), pixels, width, height);
        self.edit(|g| {
            g["images"] = json!([{"uri":"map.png"}]);
            g["textures"] = json!([{"source":0,"sampler":0}]);
            g["samplers"] =
                json!([{"magFilter":9728,"minFilter":9728,"wrapS":33071,"wrapT":33071}]);
            change(g);
        });
    }
    pub fn cook(&mut self) {
        let cooked = cook_gltf(
            self.root.path(),
            Path::new("quad.gltf"),
            &self.root.path().join(".incant/cache/models"),
        )
        .unwrap();
        self.project.assets.insert(
            self.asset_id.clone(),
            Asset {
                id: self.asset_id.clone(),
                name: "Quad".into(),
                path: "quad.gltf".into(),
                kind: "model".into(),
                sha256: cooked.metadata.fingerprint,
                import_settings: None,
            },
        );
        self.assets
            .sync_project(&self.project, self.root.path())
            .unwrap();
    }
    pub fn scene(&self, renderer: &Renderer) -> RenderScene {
        renderer.prepare_scene(&self.project, &self.assets).unwrap()
    }
    pub fn pixel(&self, renderer: &Renderer) -> [u8; 4] {
        center(
            &renderer
                .screenshot_scene_png(&self.scene(renderer), 320, 180)
                .unwrap(),
        )
    }
}
pub fn write_png(path: &Path, pixels: &[u8], width: u32, height: u32) {
    let mut encoder = png::Encoder::new(fs::File::create(path).unwrap(), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(pixels)
        .unwrap();
}
pub fn center(bytes: &[u8]) -> [u8; 4] {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .unwrap();
    let mut data = vec![0; decoder.output_buffer_size().unwrap()];
    let info = decoder.next_frame(&mut data).unwrap();
    let offset = ((info.height / 2 * info.width + info.width / 2) * 4) as usize;
    data[offset..offset + 4].try_into().unwrap()
}
pub fn near(actual: [u8; 4], expected: [u8; 4], tolerance: u8) {
    assert!(
        actual
            .into_iter()
            .zip(expected)
            .all(|(a, b)| a.abs_diff(b) <= tolerance),
        "{actual:?} != {expected:?} within {tolerance}"
    );
}
pub fn emissive(color: [f32; 3], alpha: f32, mode: &str) -> Value {
    json!({"pbrMetallicRoughness":{"baseColorFactor":[0,0,0,alpha],"metallicFactor":1},"emissiveFactor":color,"alphaMode":mode})
}
