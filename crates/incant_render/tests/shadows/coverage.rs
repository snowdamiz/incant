//! Receivers straddle each cascade split and the configured shadow distance.
use super::*;
#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn cascade_transitions_preserve_occlusion_and_fade_at_the_declared_distance() {
    check_cascade_coverage(false);
}
#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn orthographic_cascade_transitions_preserve_occlusion_and_fade_at_the_declared_distance() {
    check_cascade_coverage(true);
}
fn check_cascade_coverage(orthographic: bool) {
    let r = Renderer::headless().unwrap();
    let (mut f, camera, _caster, light) = fixture();
    f.project
        .scenes
        .values_mut()
        .next()
        .unwrap()
        .entities
        .retain(|_, e| !e.components.contains_key("MeshRenderer"));
    set(&mut f, &camera, "Transform", json!(Transform::default()));
    if orthographic {
        set(
            &mut f,
            &camera,
            "Camera",
            json!({"fov_degrees":60,"near":0.1,"far":100,"projection":{"kind":"orthographic","vertical_size":16}}),
        );
    }
    let split = |i: f64| 0.5 * (0.1 * 400_f64.powf(i / 4.) + 0.1 + 39.9 * i / 4.);
    let depths = [
        split(1.) * 0.995,
        split(1.) * 1.005,
        split(2.) * 0.995,
        split(2.) * 1.005,
        split(3.) * 0.995,
        split(3.) * 1.005,
        39.,
        41.,
    ];
    let mut samples = Vec::new();
    for (i, depth) in depths.into_iter().enumerate() {
        let nx = -0.6 + (i % 4) as f64 * 0.4;
        let ny = if i < 4 { 0.3 } else { -0.3 };
        let half_height = if orthographic {
            8.
        } else {
            depth * 30_f64.to_radians().tan()
        };
        let x = nx * half_height * 4. / 3.;
        let y = ny * half_height;
        let mut receiver = Entity::new(format!("Receiver {i}"));
        receiver.components.insert(
            "MeshRenderer".into(),
            json!({"mesh":f.asset_id,"materials":[],"cast_shadows":false}),
        );
        receiver.components.insert(
            "Transform".into(),
            json!(Transform {
                translation: [x, y, -depth],
                scale: [0.12 * half_height * 4. / 9.; 3],
                ..Default::default()
            }),
        );
        insert(&mut f, receiver);
        let mut caster = Entity::new(format!("Caster {i}"));
        caster.components.insert(
            "MeshRenderer".into(),
            json!({"mesh":f.asset_id,"materials":[],"cast_shadows":true}),
        );
        caster.components.insert(
            "Transform".into(),
            json!(Transform {
                translation: [x + 0.1 * depth, y, -depth * 0.9],
                scale: [0.025 * half_height * 4. / 9.; 3],
                ..Default::default()
            }),
        );
        insert(&mut f, caster);
        samples.push(
            (((0.5 - ny * 0.5) * 480.).floor() as usize * 640
                + ((0.5 + nx * 0.5) * 640.).floor() as usize)
                * 4,
        );
    }
    let shadow = capture(
        &f,
        &r,
        &camera,
        if orthographic {
            "orthographic-shadow-cascade-boundaries"
        } else {
            "shadow-cascade-boundaries"
        },
    );
    set(
        &mut f,
        &light,
        "DirectionalLight",
        json!({"color":[1,1,1],"intensity":2}),
    );
    let reference = capture(
        &f,
        &r,
        &camera,
        if orthographic {
            "orthographic-shadow-cascade-boundaries-reference"
        } else {
            "shadow-cascade-boundaries-reference"
        },
    );
    let decode = |png: Vec<u8>| {
        let mut reader = png::Decoder::new(std::io::Cursor::new(png))
            .read_info()
            .unwrap();
        let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
        reader.next_frame(&mut pixels).unwrap();
        pixels
    };
    let shadow = decode(shadow);
    let reference = decode(reference);
    for (i, offset) in samples.into_iter().enumerate() {
        let shaded = shadow[offset];
        let lit = reference[offset];
        assert!(lit > 120, "receiver {i} must be visible and lit: {lit}");
        if i < 6 {
            assert!(shaded < 30, "split receiver {i} lost shadow: {shaded}");
        } else if i == 6 {
            assert!(
                shaded > 30 && shaded + 10 < lit,
                "last cascade should fade: {shaded} / {lit}"
            );
        } else {
            assert_eq!(
                shaded, lit,
                "past requested distance must remain unshadowed"
            );
        }
    }
}
