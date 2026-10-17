use super::*;
use incant_assets_runtime::{CookedModel, load_model};

#[derive(Clone, Copy)]
pub(super) enum Kind {
    Geometry,
    Tangents,
}
impl Kind {
    pub(super) fn model(self, root: &Path, replacement: bool) -> VisualAssetRef {
        let change = match (self, replacement) {
            (_, false) => Change::Original,
            (Self::Geometry, true) => Change::Geometry,
            (Self::Tangents, true) => Change::GeneratedTangents,
        };
        self.cook(root, change)
    }
    pub(super) fn lit(self) -> bool {
        matches!(self, Self::Tangents)
    }
    pub(super) fn invalid(self, root: &Path) -> Option<VisualAssetRef> {
        matches!(self, Self::Geometry).then(|| self.cook(root, Change::Topology))
    }
    fn cook(self, root: &Path, change: Change) -> VisualAssetRef {
        match self {
            // Authored normals retain indexed vertices: Topology changes only
            // indices, rather than flat-normal expansion and skin correspondence.
            Self::Geometry => fixture::indexed_model(root, change),
            Self::Tangents => fixture::model(root, change),
        };
        let mut doc: serde_json::Value =
            serde_json::from_slice(&fs::read(root.join("model.gltf")).unwrap()).unwrap();
        doc["scenes"][0]["nodes"] = json!([0, 3]);
        doc["materials"][0]["doubleSided"] = json!(true);
        if self.lit() {
            // Both assets use precisely the same normal map and material. The
            // only changed decoded vertex lanes are the generated tangent basis.
            doc["materials"][0]["normalTexture"] = json!({"index":0});
            doc["materials"][0]["pbrMetallicRoughness"] = json!({
                "baseColorFactor":[0.5,0.5,0.5,1],"metallicFactor":0,"roughnessFactor":1
            });
            let mut png = png::Encoder::new(fs::File::create(root.join("rig.png")).unwrap(), 1, 1);
            png.set_color(png::ColorType::Rgba);
            png.set_depth(png::BitDepth::Eight);
            png.write_header()
                .unwrap()
                .write_image_data(&[128, 218, 218, 255])
                .unwrap();
        } else {
            doc["materials"][0]["emissiveTexture"] = json!({"index":0});
            doc["materials"][0]["emissiveFactor"] = json!([1, 1, 1]);
        }
        fs::write(root.join("model.gltf"), doc.to_string()).unwrap();
        let cooked = incant_assets::cook_gltf(
            root,
            Path::new("model.gltf"),
            &root.join(".incant/cache/models"),
        )
        .unwrap();
        reference(90, VisualAssetKind::Model, &cooked.metadata.fingerprint)
    }
    pub(super) fn check(self, root: &Path, old: VisualAssetRef, next: VisualAssetRef) {
        let before = loaded(root, old);
        let after = loaded(root, next);
        assert_ne!(old.fingerprint, next.fingerprint);
        assert_eq!(before.materials, after.materials);
        assert_eq!(before.images, after.images);
        assert_eq!(before.animation, after.animation);
        assert_eq!(before.meshes.len(), 1);
        assert_eq!(after.meshes.len(), 1);
        assert_eq!(before.meshes[0].indices, after.meshes[0].indices);
        assert_eq!(before.meshes[0].vertices.len(), 3);
        assert_eq!(after.meshes[0].vertices.len(), 3);
        for (index, (a, b)) in before.meshes[0]
            .vertices
            .iter()
            .zip(&after.meshes[0].vertices)
            .enumerate()
        {
            match self {
                Self::Geometry => {
                    assert_eq!(&a[1..], &b[1..]);
                    // Indexed fixture keeps source vertex 0 at cooked slot 0.
                    assert_eq!(b[0] - a[0], if index == 0 { 0.25 } else { 0. });
                }
                Self::Tangents => {
                    assert_eq!(&a[..8], &b[..8], "position, normal and UV are fixed");
                    assert_eq!(&a[8..], &[1., 0., 0., 1.]);
                    assert_eq!(&b[8..], &[1., 0., 0., -1.]);
                }
            }
        }
    }
}
pub(super) fn loaded(root: &Path, reference: VisualAssetRef) -> CookedModel {
    load_model(&root.join(".incant/cache/models"), &key(reference)).unwrap()
}

#[test]
fn indexed_geometry_and_topology_controls_change_only_the_intended_vertex_or_indices() {
    let root = tempfile::tempdir().unwrap();
    let old = Kind::Geometry.model(root.path(), false);
    let next = Kind::Geometry.model(root.path(), true);
    Kind::Geometry.check(root.path(), old, next);
    let invalid = Kind::Geometry.invalid(root.path()).unwrap();
    let a = loaded(root.path(), old);
    let b = loaded(root.path(), invalid);
    assert_eq!(a.meshes[0].vertices, b.meshes[0].vertices);
    assert_eq!(a.animation, b.animation);
    assert_eq!(a.materials, b.materials);
    assert_eq!(a.images, b.images);
    assert_ne!(a.meshes[0].indices, b.meshes[0].indices);
}

#[test]
fn generated_tangent_control_keeps_geometry_animation_material_and_image_exact() {
    let root = tempfile::tempdir().unwrap();
    let old = Kind::Tangents.model(root.path(), false);
    let next = Kind::Tangents.model(root.path(), true);
    Kind::Tangents.check(root.path(), old, next);
}
