//! Real occlusion checks with analytical receiver points, not visual approval.
use super::materials::Fixture;
use incant_doc::{Entity, Transform};
use incant_render::Renderer;
use serde_json::json;
mod coverage;
mod queued;
fn insert(f: &mut Fixture, e: Entity) -> String {
    let id = e.id.clone();
    f.project
        .scenes
        .values_mut()
        .next()
        .unwrap()
        .entities
        .insert(id.clone(), e);
    id
}
fn set(f: &mut Fixture, id: &str, kind: &str, value: serde_json::Value) {
    f.project
        .scenes
        .values_mut()
        .next()
        .unwrap()
        .entities
        .get_mut(id)
        .unwrap()
        .components
        .insert(kind.into(), value);
}
fn fixture() -> (Fixture, String, String, String) {
    let mut f =
        Fixture::new(json!({"pbrMetallicRoughness":{"metallicFactor":0,"roughnessFactor":1}}));
    f.environment(
        &[0, 0, 0, 255].repeat(2),
        2,
        1,
        incant_doc::TextureUsage::Linear,
    );
    let receiver = f
        .project
        .scenes
        .values()
        .next()
        .unwrap()
        .entities
        .values()
        .find(|e| e.components.contains_key("MeshRenderer"))
        .unwrap()
        .id
        .clone();
    set(
        &mut f,
        &receiver,
        "Transform",
        json!(Transform {
            scale: [1.5, 1.5, 1.],
            ..Default::default()
        }),
    );
    f.project
        .scenes
        .values_mut()
        .next()
        .unwrap()
        .entities
        .get_mut(&receiver)
        .unwrap()
        .components
        .get_mut("MeshRenderer")
        .unwrap()["cast_shadows"] = json!(false);
    let mut caster = Entity::new("Occluder");
    caster.components.insert(
        "MeshRenderer".into(),
        json!({"mesh":f.asset_id,"materials":[],"cast_shadows":true}),
    );
    caster.components.insert(
        "Transform".into(),
        json!(Transform {
            translation: [0., 0., 2.],
            scale: [0.25; 3],
            ..Default::default()
        }),
    );
    let caster = insert(&mut f, caster);
    let mut camera = Entity::new("Shadow camera");
    camera.components.insert(
        "Camera".into(),
        json!({"fov_degrees":60,"near":0.1,"far":100}),
    );
    camera.components.insert(
        "Transform".into(),
        json!(Transform {
            translation: [0., 0., 8.],
            ..Default::default()
        }),
    );
    let camera = insert(&mut f, camera);
    let mut light = Entity::new("Shadow sun");
    light.components.insert(
        "DirectionalLight".into(),
        json!({"color":[1,1,1],"intensity":2,"shadows":{"distance":40}}),
    );
    light.components.insert(
        "Transform".into(),
        json!(Transform {
            rotation: glam::DQuat::from_rotation_y(std::f64::consts::FRAC_PI_4).to_array(),
            ..Default::default()
        }),
    );
    let light = insert(&mut f, light);
    (f, camera, caster, light)
}
fn capture(f: &Fixture, r: &Renderer, camera: &str, name: &str) -> Vec<u8> {
    let scene = f.scene(r).with_camera(camera).unwrap();
    let png = r.screenshot_scene_png(&scene, 640, 480).unwrap();
    if let Some(dir) = std::env::var_os("INCANT_SHADOW_EVIDENCE") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(std::path::Path::new(&dir).join(format!("{name}.png")), &png).unwrap();
    }
    png
}
fn pixel(png: &[u8], world: [f32; 2]) -> [u8; 4] {
    // Receiver plane z=0 lies 8m in front of the 60-degree camera.
    let x = ((0.5 + world[0] / (2. * 8. * 30_f32.to_radians().tan() * 640. / 480.)) * 640.).floor()
        as usize;
    let y = ((0.5 - world[1] / (2. * 8. * 30_f32.to_radians().tan())) * 480.).floor() as usize;
    let mut reader = png::Decoder::new(std::io::Cursor::new(png))
        .read_info()
        .unwrap();
    let mut bytes = vec![0; reader.output_buffer_size().unwrap()];
    reader.next_frame(&mut bytes).unwrap();
    bytes[(y * 640 + x) * 4..(y * 640 + x) * 4 + 4]
        .try_into()
        .unwrap()
}
#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn directional_shadow_occlusion_respects_flags_range_and_retained_scene_versions() {
    let r = Renderer::headless().unwrap();
    let (mut f, camera, caster, light) = fixture();
    let prepared = f.scene(&r).with_camera(&camera).unwrap();
    let shadow = capture(&f, &r, &camera, "shadow-enabled");
    set(
        &mut f,
        &light,
        "DirectionalLight",
        json!({"color":[1,1,1],"intensity":2}),
    );
    let unshadowed = capture(&f, &r, &camera, "shadow-disabled");
    let dark = pixel(&shadow, [-2., 0.]);
    let lit = pixel(&unshadowed, [-2., 0.]);
    assert!(
        dark[0] < 30 && lit[0] > 120,
        "shadow {dark:?}, reference {lit:?}"
    );
    assert_eq!(pixel(&shadow, [2., 0.]), pixel(&unshadowed, [2., 0.]));
    set(
        &mut f,
        &light,
        "DirectionalLight",
        json!({"color":[1,1,1],"intensity":2,"shadows":{"distance":40}}),
    );
    f.project
        .scenes
        .values_mut()
        .next()
        .unwrap()
        .entities
        .get_mut(&caster)
        .unwrap()
        .components
        .get_mut("MeshRenderer")
        .unwrap()["cast_shadows"] = json!(false);
    assert_eq!(
        capture(&f, &r, &camera, "shadow-caster-disabled"),
        unshadowed
    );
    f.project
        .scenes
        .values_mut()
        .next()
        .unwrap()
        .entities
        .get_mut(&caster)
        .unwrap()
        .components
        .get_mut("MeshRenderer")
        .unwrap()["cast_shadows"] = json!(true);
    assert_eq!(capture(&f, &r, &camera, "shadow-caster-restored"), shadow);
    let receiver = f
        .project
        .scenes
        .values_mut()
        .next()
        .unwrap()
        .entities
        .values_mut()
        .find(|e| {
            e.components
                .get("MeshRenderer")
                .is_some_and(|m| m["cast_shadows"] == false)
        })
        .unwrap();
    receiver.components.get_mut("MeshRenderer").unwrap()["cast_shadows"] = json!(true);
    assert_eq!(
        capture(&f, &r, &camera, "shadow-self-casting-receiver"),
        shadow
    );

    set(
        &mut f,
        &light,
        "DirectionalLight",
        json!({"color":[1,1,1],"intensity":2,"shadows":{"distance":5}}),
    );
    assert_eq!(
        capture(&f, &r, &camera, "shadow-beyond-distance"),
        unshadowed
    );
    set(
        &mut f,
        &light,
        "DirectionalLight",
        json!({"color":[1,1,1],"intensity":2,"shadows":{"distance":40}}),
    );
    set(
        &mut f,
        &caster,
        "Transform",
        json!(Transform {
            translation: [1., 0., 2.],
            scale: [0.25; 3],
            ..Default::default()
        }),
    );
    let moved = capture(&f, &r, &camera, "shadow-moved-caster");
    assert_ne!(moved, shadow);
    // Reimport changes both geometry and caster bounds. A previously prepared
    // scene must retain its original shadow geometry after the cache advances.
    let path = f.root.path().join("quad.bin");
    let mut bytes = std::fs::read(&path).unwrap();
    for vertex in bytes[..72].chunks_exact_mut(12) {
        let x = f32::from_le_bytes(vertex[..4].try_into().unwrap()) * 2.;
        vertex[..4].copy_from_slice(&x.to_le_bytes());
    }
    std::fs::write(path, bytes).unwrap();
    f.edit(|g| {
        g["accessors"][0]["min"][0] = json!(-6.);
        g["accessors"][0]["max"][0] = json!(6.);
    });
    assert_ne!(capture(&f, &r, &camera, "shadow-reimported-width"), moved);
    assert_eq!(r.screenshot_scene_png(&prepared, 640, 480).unwrap(), shadow);
}

#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn offscreen_casters_and_multiple_directional_shadow_slots_remain_visible() {
    let r = Renderer::headless().unwrap();
    let (mut f, camera, caster, light) = fixture();
    // Caster is outside the camera frustum, but its ray reaches receiver origin.
    set(
        &mut f,
        &caster,
        "Transform",
        json!(Transform {
            translation: [8., 0., 4.],
            scale: [0.25; 3],
            ..Default::default()
        }),
    );
    set(
        &mut f,
        &light,
        "Transform",
        json!(Transform {
            rotation: glam::DQuat::from_rotation_y(2_f64.atan()).to_array(),
            ..Default::default()
        }),
    );
    let shadow = capture(&f, &r, &camera, "shadow-offscreen-caster");
    set(
        &mut f,
        &light,
        "DirectionalLight",
        json!({"color":[1,1,1],"intensity":2}),
    );
    let reference = capture(&f, &r, &camera, "shadow-offscreen-reference");
    assert!(pixel(&shadow, [0., 0.])[0] < 30);
    assert!(pixel(&reference, [0., 0.])[0] > 100);
    // Four enabled, differently colored suns exercise every atlas slot. The
    // co-located directions cast the same analytically located shadow.
    set(
        &mut f,
        &light,
        "DirectionalLight",
        json!({"color":[1,0.2,0.3],"intensity":1,"shadows":{"distance":40}}),
    );
    for i in 0..3 {
        let mut extra = Entity::new("Additional shadow sun");
        extra.components.insert(
            "DirectionalLight".into(),
            json!({"color":[0.2,0.3+0.2*i as f64,1],"intensity":1,"shadows":{"distance":40}}),
        );
        extra.components.insert(
            "Transform".into(),
            json!(Transform {
                rotation: glam::DQuat::from_rotation_y(2_f64.atan()).to_array(),
                ..Default::default()
            }),
        );
        insert(&mut f, extra);
    }
    let four = capture(&f, &r, &camera, "shadow-four-suns");
    assert!(pixel(&four, [0., 0.])[..3].iter().all(|v| *v < 30));
    assert!(pixel(&four, [2., 0.])[0] > 100);
    let mut extra = Entity::new("Over budget sun");
    extra.components.insert(
        "DirectionalLight".into(),
        json!({"color":[1,1,1],"intensity":1,"shadows":{"distance":40}}),
    );
    insert(&mut f, extra);
    let error = r.prepare_scene(&f.project, &f.assets).err().unwrap();
    assert!(matches!(
        error.downcast_ref::<incant_render::SceneError>(),
        Some(incant_render::SceneError::ShadowLightLimit)
    ));
}

fn separate_receiver_asset(f: &mut Fixture) {
    let source = std::fs::read(f.root.path().join("quad.gltf")).unwrap();
    std::fs::write(f.root.path().join("receiver.gltf"), source).unwrap();
    let cooked = incant_assets::cook_gltf(
        f.root.path(),
        std::path::Path::new("receiver.gltf"),
        &f.root.path().join(".incant/cache/models"),
    )
    .unwrap();
    let id = incant_doc::new_id();
    f.project.assets.insert(
        id.clone(),
        incant_doc::Asset {
            id: id.clone(),
            name: "Receiver".into(),
            path: "receiver.gltf".into(),
            kind: "model".into(),
            sha256: cooked.metadata.fingerprint,
            import_settings: None,
        },
    );
    let receiver = f
        .project
        .scenes
        .values_mut()
        .next()
        .unwrap()
        .entities
        .values_mut()
        .find(|e| {
            e.components
                .get("MeshRenderer")
                .is_some_and(|m| m["cast_shadows"] == false)
        })
        .unwrap();
    receiver.components.get_mut("MeshRenderer").unwrap()["mesh"] = json!(id);
    f.assets.sync_project(&f.project, f.root.path()).unwrap();
}
#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn alpha_masked_and_reflected_casters_preserve_holes_and_blend_casters_fail_explicitly() {
    let r = Renderer::headless().unwrap();
    let (mut f, camera, caster, _light) = fixture();
    separate_receiver_asset(&mut f);
    // Left half opaque, right half transparent; magnification is nearest.
    f.texture(&[255, 255, 255, 255, 255, 255, 255, 0], 2, 1, |g| {
        g["materials"][0]["alphaMode"] = json!("MASK");
        g["materials"][0]["alphaCutoff"] = json!(0.5);
        g["materials"][0]["pbrMetallicRoughness"]["baseColorTexture"] = json!({"index":0});
    });
    let mask = capture(&f, &r, &camera, "shadow-alpha-mask");
    assert!(pixel(&mask, [-2.4, 0.])[0] < 30);
    assert!(pixel(&mask, [-1.6, 0.])[0] > 120);
    set(
        &mut f,
        &caster,
        "Transform",
        json!(Transform {
            translation: [0., 0., 2.],
            scale: [-0.25, 0.25, 0.25],
            ..Default::default()
        }),
    );
    let reflected = capture(&f, &r, &camera, "shadow-reflected-mask");
    assert!(pixel(&reflected, [-2.4, 0.])[0] > 120);
    assert!(pixel(&reflected, [-1.6, 0.])[0] < 30);
    set(
        &mut f,
        &caster,
        "Transform",
        json!(Transform {
            translation: [0., 0., 2.],
            scale: [0.25; 3],
            rotation: glam::DQuat::from_rotation_y(std::f64::consts::PI).to_array(),
        }),
    );
    let single = capture(&f, &r, &camera, "shadow-single-sided-back");
    assert!(pixel(&single, [-1.6, 0.])[0] > 120 && pixel(&single, [-2.4, 0.])[0] > 120);
    f.edit(|g| g["materials"][0]["doubleSided"] = json!(true));
    let double = capture(&f, &r, &camera, "shadow-double-sided-back");
    assert!(pixel(&double, [-1.6, 0.])[0] < 30 && pixel(&double, [-2.4, 0.])[0] > 120);
    f.edit(|g| g["materials"][0]["alphaMode"] = json!("BLEND"));
    let error = r.prepare_scene(&f.project, &f.assets).err().unwrap();
    assert!(matches!(
        error.downcast_ref::<incant_render::ResourceError>(),
        Some(incant_render::ResourceError::TransparentShadowCaster)
    ));
    f.project
        .scenes
        .values_mut()
        .next()
        .unwrap()
        .entities
        .get_mut(&caster)
        .unwrap()
        .components
        .get_mut("MeshRenderer")
        .unwrap()["cast_shadows"] = json!(false);
    capture(&f, &r, &camera, "shadow-blended-noncaster");
}
