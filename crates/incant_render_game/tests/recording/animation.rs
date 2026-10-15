#[path = "../../../incant_assets/tests/skeletal/fixture.rs"]
mod fixture;
use incant_assets_runtime::RuntimeAsset;
use incant_render_data::{MeshInput, SceneInput, SceneNode, prewarm::MaterialRecorder};
use incant_types::{AnimationSample, Camera};
use std::{path::Path, sync::Arc};

fn input(rigid: bool, mixed_skin: bool) -> (SceneInput, tempfile::TempDir) {
    let temp = tempfile::tempdir().unwrap();
    let mut doc = fixture::fixture(temp.path());
    doc["scenes"][0]["nodes"] = serde_json::json!([0, 3]);
    if rigid {
        doc["nodes"][3].as_object_mut().unwrap().remove("skin");
        doc["animations"][2]["channels"][0]["target"]["node"] = 3.into();
    } else if mixed_skin {
        doc["animations"][2]["channels"][0]["target"]["node"] = 2.into();
    }
    fixture::write(temp.path(), &doc);
    let cache = temp.path().join("cooked");
    let cooked = incant_assets::cook_gltf(temp.path(), Path::new("model.gltf"), &cache).unwrap();
    let source = Arc::new(
        RuntimeAsset::load_model("model", 1, &cache, &cooked.metadata.fingerprint).unwrap(),
    );
    let node = |id: &str| SceneNode {
        id: id.into(),
        world: Default::default(),
        camera: None,
        light: None,
        mesh: None,
        sprite: None,
        tilemap: None,
        diagnostic: false,
    };
    let mut model = node("model");
    model.mesh = Some(MeshInput {
        source,
        cast_shadows: false,
        animation: None,
        animation_blend: None,
        animation_pose: None,
        animation_ik: None,
        animation_root: None,
    });
    let mut camera = node("camera");
    camera.camera = Some(Camera::default());
    (
        SceneInput {
            nodes: vec![model, camera],
            ..Default::default()
        },
        temp,
    )
}
#[test]
fn committed_rigid_and_skinned_orientation_use_the_same_parity_as_rendering() {
    for rigid in [false, true] {
        let (mut input, temp) = input(rigid, false);
        let mut recorder = MaterialRecorder::new(&input, "camera", 0, super::context()).unwrap();
        // All source and cooked files disappear. Only retained decoded bytes,
        // sealed receipts and committed samples remain available to observation.
        temp.close().unwrap();
        for (tick, reflected_pose, reflected_world) in [
            (1, true, false),
            (2, true, true),
            (3, false, true),
            (4, false, false),
        ] {
            input.nodes[0].world.x_axis.x = if reflected_world { -1. } else { 1. };
            input.nodes[0].mesh.as_mut().unwrap().animation = Some(if reflected_pose {
                AnimationSample {
                    clip: Some(2),
                    seconds: 2.,
                }
            } else {
                AnimationSample::REST
            });
            recorder.accept(&input, "camera", tick).unwrap();
        }
        let manifest = recorder.manifest().unwrap();
        assert_eq!(manifest.observed.len(), 2, "rigid={rigid}");
        assert!(!manifest.observed[0].key.mirrored());
        assert_eq!(manifest.observed[0].ticks, 3);
        assert_eq!(manifest.observed[0].last_tick, 4);
        assert!(manifest.observed[1].key.mirrored());
        assert_eq!(manifest.observed[1].ticks, 2);
        assert_eq!(
            (
                manifest.observed[1].first_tick,
                manifest.observed[1].last_tick
            ),
            (1, 3)
        );
    }
}
#[test]
fn mixed_skin_orientation_poisoning_cannot_export_a_partial_manifest() {
    let (mut input, _temp) = input(false, true);
    let mut recorder = MaterialRecorder::new(&input, "camera", 0, super::context()).unwrap();
    input.nodes[0].mesh.as_mut().unwrap().animation = Some(AnimationSample {
        clip: Some(2),
        seconds: 2.,
    });
    let error = recorder
        .accept(&input, "camera", 1)
        .unwrap_err()
        .to_string();
    assert!(error.contains("mixed skin orientation"), "{error}");
    assert!(recorder.manifest().is_err());
    input.nodes[0].mesh.as_mut().unwrap().animation = Some(AnimationSample::REST);
    assert!(recorder.accept(&input, "camera", 2).is_err());
}
