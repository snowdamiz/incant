//! Independent screen-space polygon oracle: no reconstruction shader math.
use glam::{DMat4, DVec3};
use incant_assets_runtime::RuntimeAsset;
use incant_render_data::{MeshInput, SceneNode};
use serde_json::json;
use std::{path::Path, sync::Arc};
pub(super) type Point = [f64; 2];
pub(super) const EDGE: [Point; 4] = [[6.25, 5.75], [25.375, 5.75], [36.625, 35.25], [6.25, 35.25]];
pub(super) const STRIP: [Point; 4] = [[6.25, 44.75], [54.25, 44.75], [54.25, 45.], [6.25, 45.]];

/// Signed distance to each convex polygon boundary. Only exact raster-edge
/// ties within 1e-5 output pixels are excluded from point-coverage assertions.
pub(super) fn contains(p: Point, polygon: &[Point]) -> Option<bool> {
    let mut nearest = f64::INFINITY;
    for (a, b) in polygon.iter().zip(polygon.iter().cycle().skip(1)) {
        let d = [b[0] - a[0], b[1] - a[1]];
        let side = (d[0] * (p[1] - a[1]) - d[1] * (p[0] - a[0])) / d[0].hypot(d[1]);
        nearest = nearest.min(side);
    }
    // Extended edge lines outside another boundary have a negative minimum.
    if nearest.abs() <= 1e-5 {
        None
    } else {
        Some(nearest > 0.)
    }
}

/// Exact polygon clipping to a pixel square, followed by shoelace area.
pub(super) fn area(polygon: &[Point], x: usize, y: usize) -> f64 {
    let mut p = polygon.to_vec();
    for (axis, boundary, sign) in [
        (0, x as f64, 1.),
        (0, x as f64 + 1., -1.),
        (1, y as f64, 1.),
        (1, y as f64 + 1., -1.),
    ] {
        let mut clipped = Vec::new();
        if p.is_empty() {
            return 0.;
        }
        for (&a, &b) in p.iter().zip(p.iter().cycle().skip(1)) {
            let da = sign * (a[axis] - boundary);
            let db = sign * (b[axis] - boundary);
            if da >= 0. {
                clipped.push(a);
            }
            if (da < 0.) != (db < 0.) {
                let t = da / (da - db);
                clipped.push([a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])]);
            }
        }
        p = clipped;
    }
    (p.iter()
        .zip(p.iter().cycle().skip(1))
        .map(|(a, b)| a[0] * b[1] - a[1] * b[0])
        .sum::<f64>()
        * 0.5)
        .abs()
}

pub(super) fn fixture() -> crate::SceneInput {
    let dir = tempfile::tempdir().unwrap();
    let positions: Vec<f32> = [EDGE, STRIP]
        .iter()
        .flat_map(|p| {
            // Screen Y points down; reverse winding for camera-facing +Z normals.
            [0, 2, 1, 0, 3, 2]
                .into_iter()
                .flat_map(move |i| [(p[i][0] - 32.) as f32 / 8., (32. - p[i][1]) as f32 / 8., 0.])
        })
        .collect();
    std::fs::write(
        dir.path().join("mesh.bin"),
        bytemuck::cast_slice(&positions),
    )
    .unwrap();
    let doc = json!({"asset":{"version":"2.0"},"buffers":[{"uri":"mesh.bin","byteLength":positions.len()*4}],
        "bufferViews":[{"buffer":0,"byteLength":positions.len()*4}],
        "accessors":[{"bufferView":0,"componentType":5126,"count":12,"type":"VEC3","min":[-4,-4,0],"max":[4,4,0]}],
        "materials":[{"pbrMetallicRoughness":{"baseColorFactor":[0,0,0,1],"metallicFactor":0,"roughnessFactor":1},"emissiveFactor":[1,1,1]}],
        "meshes":[{"primitives":[{"attributes":{"POSITION":0},"material":0}]}],"nodes":[{"mesh":0}],"scenes":[{"nodes":[0]}],"scene":0});
    std::fs::write(
        dir.path().join("mesh.gltf"),
        serde_json::to_vec(&doc).unwrap(),
    )
    .unwrap();
    let cache = dir.path().join("cache");
    let cooked = incant_assets::cook_gltf(dir.path(), Path::new("mesh.gltf"), &cache).unwrap();
    let source = Arc::new(
        RuntimeAsset::load_model("edge-strip", 1, &cache, &cooked.metadata.fingerprint).unwrap(),
    );
    let mesh = SceneNode {
        id: "mesh".into(),
        world: DMat4::IDENTITY,
        camera: None,
        mesh: Some(MeshInput {
            source,
            cast_shadows: false,
            animation: None,
            animation_blend: None,
            animation_pose: None,
            animation_ik: None,
            animation_root: None,
        }),
        light: None,
        sprite: None,
        tilemap: None,
        diagnostic: false,
    };
    let camera = SceneNode {
        id: "camera".into(),
        world: DMat4::from_translation(DVec3::new(0., 0., 10.)),
        camera: Some(incant_types::Camera {
            fov_degrees: 60.,
            near: 0.1,
            far: 50.,
            projection: incant_types::CameraProjection::Orthographic { vertical_size: 8. },
        }),
        mesh: None,
        light: None,
        sprite: None,
        tilemap: None,
        diagnostic: false,
    };
    dir.close().unwrap();
    crate::SceneInput {
        nodes: vec![mesh, camera],
        ..Default::default()
    }
}

#[test]
fn source_deleted_cooked_fixture_preserves_opaque_projected_polygons() {
    let input = fixture();
    let incant_assets_runtime::RuntimeAssetData::Model(model) =
        input.nodes[0].mesh.as_ref().unwrap().source.data()
    else {
        panic!("model fixture")
    };
    assert_eq!(model.meshes.len(), 1);
    assert_eq!(model.meshes[0].vertices.len(), 12);
    assert_eq!(
        model.materials[0].alpha_mode,
        incant_assets_runtime::AlphaMode::Opaque
    );
    let corners: Vec<_> = [EDGE, STRIP].into_iter().flatten().collect();
    for v in &model.meshes[0].vertices {
        assert_eq!(v[2], 0.);
        let p = [f64::from(v[0]) * 8. + 32., 32. - f64::from(v[1]) * 8.];
        assert!(corners.contains(&p), "unexpected cooked position {p:?}");
    }
}
#[test]
fn polygon_oracle_measures_area_and_only_exempts_edge_ties() {
    let square = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
    assert_eq!(area(&square, 0, 0), 1.);
    assert_eq!(area(&square, 1, 0), 0.);
    assert_eq!(contains([0.5, 0.5], &square), Some(true));
    assert_eq!(contains([0., 0.5], &square), None);
    assert_eq!(contains([0., 2.], &square), Some(false));
    let half = [[0., 0.], [1., 0.], [0., 1.]];
    assert_eq!(area(&half, 0, 0), 0.5);
    let sum = (0..64)
        .flat_map(|y| (0..64).map(move |x| area(&STRIP, x, y)))
        .sum::<f64>();
    assert_eq!(sum, 12.);
}
