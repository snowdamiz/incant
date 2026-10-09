//! Geometric coverage fixtures requested by Claude, without an art-direction claim.
use super::*;

fn save(name: &str, png: &[u8]) {
    if let Some(directory) = std::env::var_os("INCANT_LIGHT_EVIDENCE") {
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            std::path::Path::new(&directory).join(format!("{name}.png")),
            png,
        )
        .unwrap();
    }
}
fn oracle(f: &mut Fixture, r: &Renderer, name: &str) {
    let reference = f.scene(r);
    // These zero-energy lights force every cluster into the all-local-light
    // fallback. The oracle consequently bypasses all spatial selection.
    for _ in 0..65 {
        light(
            f,
            "PointLight",
            json!({"color":[0,0,0],"intensity":0,"range":10000}),
            [0.; 3],
        );
    }
    let brute_force = f.scene(r);
    for (w, h) in [(640, 480), (513, 385)] {
        let a = r.screenshot_scene_png(&reference, w, h).unwrap();
        let b = r.screenshot_scene_png(&brute_force, w, h).unwrap();
        assert_eq!(a, b, "{name} at {w}x{h}");
        save(&format!("{name}-{w}x{h}"), &a);
        save(&format!("{name}-{w}x{h}-oracle"), &b);
    }
}

#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn distinct_colored_lights_preserve_energy_across_cluster_overflow() {
    let r = Renderer::headless().unwrap();
    let mut f = fixture();
    for z in 0..3 {
        for y in 0..4 {
            for x in 0..8 {
                light(
                    &mut f,
                    "PointLight",
                    json!({
                        "color":[0.2+0.1*x as f64,0.2+0.2*y as f64,0.2+0.3*z as f64],
                        "intensity":0.15,"range":4
                    }),
                    [
                        (x as f64 - 3.5) * 0.2,
                        (y as f64 - 1.5) * 0.4,
                        1.4 + z as f64 * 0.2,
                    ],
                );
            }
        }
    }
    oracle(&mut f, &r, "spatial-overflow-96");
}

#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn visible_depth_boundary_patches_match_all_light_oracle() {
    let r = Renderer::headless().unwrap();
    let mut f = fixture();
    let view = glam::Mat4::look_at_rh(glam::Vec3::new(6., 5., 9.), glam::Vec3::ZERO, glam::Vec3::Y)
        .inverse();
    let tangent = 25_f32.to_radians().tan();
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    for slice in 0..24 {
        // Four columns and six rows, left to right / top to bottom. Each patch
        // lies just to one side of a logarithmic boundary. Perspective-sized
        // patches make distant slices visible instead of subpixel at a horizon.
        let depth = 0.1_f32
            * 10000_f32.powf(slice as f32 / 24.)
            * if slice % 2 == 0 { 1.0005 } else { 0.9995 };
        let x = -0.75 + (slice % 4) as f32 * 0.5;
        let y = 1. - ((slice / 4) as f32 + 0.5) / 3.;
        let world = |nx: f32, ny: f32| {
            view.transform_point3(glam::Vec3::new(
                nx * tangent * 4. / 3. * depth,
                ny * tangent * depth,
                -depth,
            ))
        };
        let corners = [
            world(x - 0.20, y - 0.12),
            world(x + 0.20, y - 0.12),
            world(x + 0.20, y + 0.12),
            world(x - 0.20, y + 0.12),
        ];
        let normal = view.transform_vector3(glam::Vec3::Z).normalize();
        for index in [0, 1, 2, 0, 2, 3] {
            positions.push(corners[index]);
            normals.push(normal);
        }
        let center = world(x, y);
        light(
            &mut f,
            "PointLight",
            json!({
                "color":[0.3+0.15*(slice%4) as f64,0.3+0.1*(slice/4) as f64,0.6],
                "intensity":0.003*depth*depth,"range":0.16*depth
            }),
            (center + normal * (0.05 * depth)).as_dvec3().to_array(),
        );
    }
    let low = positions
        .iter()
        .fold(glam::Vec3::splat(f32::INFINITY), |a, p| a.min(*p));
    let high = positions
        .iter()
        .fold(glam::Vec3::splat(f32::NEG_INFINITY), |a, p| a.max(*p));
    let bytes: Vec<_> = positions
        .iter()
        .chain(normals.iter())
        .flat_map(|p| p.to_array())
        .flat_map(f32::to_le_bytes)
        .collect();
    std::fs::write(f.root.path().join("quad.bin"), &bytes).unwrap();
    let count = positions.len();
    f.edit(|g|{
        g["buffers"][0]["byteLength"] = json!(bytes.len());
        g["bufferViews"] = json!([{"buffer":0,"byteLength":count*12},{"buffer":0,"byteOffset":count*12,"byteLength":count*12}]);
        g["accessors"] = json!([
            {"bufferView":0,"componentType":5126,"count":count,"type":"VEC3","min":low.to_array(),"max":high.to_array()},
            {"bufferView":1,"componentType":5126,"count":count,"type":"VEC3"}]);
        g["meshes"][0]["primitives"][0]["attributes"] = json!({"POSITION":0,"NORMAL":1});
    });
    oracle(&mut f, &r, "depth-boundary-patches");
}

#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn local_range_edges_and_hdr_specular_materials_render_repeatably() {
    let r = Renderer::headless().unwrap();
    let mut f = fixture();
    let id = light(
        &mut f,
        "PointLight",
        json!({"color":[1,0.7,0.3],"intensity":5,"range":1.8}),
        [0., 0., 0.8],
    );
    for (w, h) in [(640, 480), (513, 385)] {
        save(
            &format!("visible-range-edge-{w}x{h}"),
            &r.screenshot_scene_png(&f.scene(&r), w, h).unwrap(),
        );
    }
    // A sphere continuously spans normal and view angles, including grazing
    // angles; the same material sweep receives a high-intensity authored point.
    crate::environments::sphere(&mut f);
    component(
        &mut f,
        &id,
        "PointLight",
        json!({"color":[1,0.8,0.5],"intensity":1000,"range":100}),
    );
    component(
        &mut f,
        &id,
        "Transform",
        json!(Transform {
            translation: [4., 5., 6.],
            ..Default::default()
        }),
    );
    let mut outputs = Vec::new();
    for (label, metallic) in [("dielectric", 0.), ("metal", 1.)] {
        for roughness in [0.045, 0.3, 0.7, 1.] {
            f.edit(|g|g["materials"][0]=json!({"pbrMetallicRoughness":{
                "baseColorFactor":[0.5,0.5,0.5,1],"metallicFactor":metallic,"roughnessFactor":roughness
            }}));
            let scene = f.scene(&r);
            let a = r.screenshot_scene_png(&scene, 640, 480).unwrap();
            assert_eq!(a, r.screenshot_scene_png(&scene, 640, 480).unwrap());
            assert!(
                !outputs.contains(&a),
                "material changes must affect punctual illumination"
            );
            save(&format!("punctual-hdr-{label}-{roughness}"), &a);
            outputs.push(a);
        }
    }
    component(
        &mut f,
        &id,
        "PointLight",
        json!({"color":[1,0.8,0.5],"intensity":50,"range":100}),
    );
    for roughness in [0.045, 0.3, 0.7, 1.] {
        f.edit(|g| {
            g["materials"][0] = json!({"pbrMetallicRoughness":{
                "baseColorFactor":[0.5,0.5,0.5,1],"metallicFactor":0,"roughnessFactor":roughness
            }})
        });
        save(
            &format!("punctual-low-dielectric-{roughness}"),
            &r.screenshot_scene_png(&f.scene(&r), 640, 480).unwrap(),
        );
    }
}
