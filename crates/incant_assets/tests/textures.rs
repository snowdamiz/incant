use image::{DynamicImage, ImageBuffer, ImageFormat, Rgba, RgbaImage};
use incant_assets::{
    TextureFormat, TextureUsage, cook_gltf, cook_texture, decode_ktx2, encode_ktx2, import_image,
    load_model, load_texture,
};
use serde_json::json;
use std::{fs, io::Cursor, path::Path};
fn encode(image: DynamicImage, format: ImageFormat) -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    image.write_to(&mut out, format).unwrap();
    out.into_inner()
}
fn png(w: u32, h: u32, pixels: Vec<u8>) -> Vec<u8> {
    encode(
        DynamicImage::ImageRgba8(RgbaImage::from_raw(w, h, pixels).unwrap()),
        ImageFormat::Png,
    )
}
fn floats(bytes: &[u8]) -> Vec<f32> {
    bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| f32::from_le_bytes(*b))
        .collect()
}
#[test]
fn color_mips_average_linear_light_while_numeric_maps_preserve_values() {
    let source = png(2, 1, vec![0, 0, 0, 255, 255, 255, 255, 255]);
    let color = import_image(&source, TextureUsage::Color).unwrap();
    let data = import_image(&source, TextureUsage::Linear).unwrap();
    assert_eq!(color.format, TextureFormat::Rgba8Srgb);
    assert_eq!(color.levels[1], [188, 188, 188, 255]);
    assert_eq!(data.format, TextureFormat::Rgba8Linear);
    assert_eq!(data.levels[1], [128, 128, 128, 255]);
    for t in [color, data] {
        assert_eq!(decode_ktx2(&encode_ktx2(&t).unwrap()).unwrap(), t);
    }
}
#[test]
fn transparent_color_does_not_bleed_and_odd_edges_are_not_dropped() {
    let texture = import_image(
        &png(2, 1, vec![255, 0, 0, 0, 0, 0, 255, 255]),
        TextureUsage::Color,
    )
    .unwrap();
    assert_eq!(texture.levels[1], [0, 0, 255, 128]);
    let texture = import_image(
        &png(3, 1, vec![0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255, 255]),
        TextureUsage::Linear,
    )
    .unwrap();
    assert_eq!(texture.levels[1], [85, 85, 85, 255]);
}
#[test]
fn normal_map_mips_renormalize_vectors() {
    let texture = import_image(
        &png(2, 1, vec![255, 128, 128, 255, 128, 255, 128, 255]),
        TextureUsage::Normal,
    )
    .unwrap();
    let normal: Vec<_> = texture.levels[1][..3]
        .iter()
        .map(|&v| v as f32 / 255. * 2. - 1.)
        .collect();
    assert!((normal.iter().map(|v| v * v).sum::<f32>().sqrt() - 1.).abs() < 0.01);
    assert!(normal[0] > 0.70 && normal[1] > 0.70);
}
#[test]
fn exr_and_sixteen_bit_png_keep_precision_and_hdr_range() {
    let img =
        ImageBuffer::<Rgba<f32>, Vec<f32>>::from_raw(2, 1, vec![4., 2., -0.5, 1., 2., 0., 0.5, 1.])
            .unwrap();
    let source = encode(DynamicImage::ImageRgba32F(img), ImageFormat::OpenExr);
    let texture = import_image(&source, TextureUsage::Color).unwrap();
    assert_eq!(texture.format, TextureFormat::Rgba32Float);
    assert_eq!(
        floats(&texture.levels[0]),
        [4., 2., -0.5, 1., 2., 0., 0.5, 1.]
    );
    assert_eq!(floats(&texture.levels[1]), [3., 1., 0., 1.]);
    assert_eq!(
        decode_ktx2(&encode_ktx2(&texture).unwrap()).unwrap(),
        texture
    );
    let img = ImageBuffer::<Rgba<u16>, Vec<u16>>::from_raw(1, 1, vec![12345, 32123, 65000, 65535])
        .unwrap();
    let texture = import_image(
        &encode(DynamicImage::ImageRgba16(img), ImageFormat::Png),
        TextureUsage::Linear,
    )
    .unwrap();
    assert_eq!(texture.format, TextureFormat::Rgba32Float);
    assert!((floats(&texture.levels[0])[0] - 12345. / 65535.).abs() < 1e-6);
}
#[test]
fn jpeg_cooks_and_invalid_images_and_containers_fail() {
    let rgb = image::RgbImage::from_pixel(2, 2, image::Rgb([64, 128, 192]));
    let texture = import_image(
        &encode(DynamicImage::ImageRgb8(rgb), ImageFormat::Jpeg),
        TextureUsage::Color,
    )
    .unwrap();
    assert_eq!((texture.width, texture.height), (2, 2));
    assert!((texture.levels[0][0] as i16 - 64).abs() < 5);
    assert!(import_image(b"not an image", TextureUsage::Color).is_err());
    let bytes = encode_ktx2(&texture).unwrap();
    for n in 0..bytes.len() {
        assert!(decode_ktx2(&bytes[..n]).is_err(), "truncated at {n}");
    }
    let mut bad = bytes.clone();
    bad[20..24].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(decode_ktx2(&bad).is_err());
    let mut bad = bytes;
    bad[40..44].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(decode_ktx2(&bad).is_err());
}
#[test]
fn standalone_texture_cache_survives_source_removal_and_repairs_corruption() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let cache = root.join("cache");
    let source = Path::new("texture.png");
    fs::write(
        root.join(source),
        png(2, 1, vec![0, 0, 0, 255, 255, 255, 255, 255]),
    )
    .unwrap();
    let a = cook_texture(root, source, &cache, TextureUsage::Color).unwrap();
    assert!(!a.cache_hit);
    assert!(
        cook_texture(root, source, &cache, TextureUsage::Color)
            .unwrap()
            .cache_hit
    );
    let linear = cook_texture(root, source, &cache, TextureUsage::Linear).unwrap();
    assert_ne!(linear.metadata.fingerprint, a.metadata.fingerprint);
    let path = cache.join(format!("{}.ktx2", a.metadata.content_sha256));
    fs::write(path, b"corrupt").unwrap();
    assert!(load_texture(&cache, &a.metadata.fingerprint).is_err());
    let repaired = cook_texture(root, source, &cache, TextureUsage::Color).unwrap();
    assert!(!repaired.cache_hit);
    fs::remove_file(root.join(source)).unwrap();
    assert_eq!(
        load_texture(&cache, &a.metadata.fingerprint)
            .unwrap()
            .texture,
        a.texture
    );
}
fn textured_fixture(root: &Path) {
    let positions = [0_f32, 0., 0., 1., 0., 0., 0., 1., 0.];
    let uvs = [0_f32, 0., 1., 0., 0., 1.];
    let bytes: Vec<_> = positions
        .into_iter()
        .chain(uvs)
        .flat_map(f32::to_le_bytes)
        .collect();
    fs::write(root.join("mesh.bin"), bytes).unwrap();
    fs::write(
        root.join("map.png"),
        png(2, 1, vec![255, 128, 128, 255, 128, 255, 128, 255]),
    )
    .unwrap();
    let doc = json!({"asset":{"version":"2.0"},"buffers":[{"byteLength":60,"uri":"mesh.bin"}],"bufferViews":[{"buffer":0,"byteLength":36},{"buffer":0,"byteOffset":36,"byteLength":24}],"accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","min":[0,0,0],"max":[1,1,0]},{"bufferView":1,"componentType":5126,"count":3,"type":"VEC2"}],"images":[{"uri":"map.png"}],"samplers":[{"magFilter":9728,"minFilter":9987,"wrapS":33071,"wrapT":33648}],"textures":[{"source":0,"sampler":0}],"materials":[{"pbrMetallicRoughness":{"baseColorTexture":{"index":0},"metallicRoughnessTexture":{"index":0}},"normalTexture":{"index":0},"occlusionTexture":{"index":0}}],"meshes":[{"primitives":[{"attributes":{"POSITION":0,"TEXCOORD_0":1},"material":0}]}],"nodes":[{"mesh":0}],"scenes":[{"nodes":[0]}],"scene":0});
    fs::write(root.join("model.gltf"), doc.to_string()).unwrap();
}
#[test]
fn model_textures_keep_bindings_variants_tangents_and_image_dependencies() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    textured_fixture(root);
    let cache = root.join("cache");
    let source = Path::new("model.gltf");
    let first = cook_gltf(root, source, &cache).unwrap();
    assert_eq!(first.images.len(), 3);
    assert_eq!(first.metadata.dependencies.len(), 3);
    let t = &first.metadata.textures[0];
    assert_eq!(t.wrap_s, 33071);
    assert_eq!(t.wrap_t, 33648);
    assert_eq!(t.mag_filter, Some(9728));
    assert_eq!(
        first.images[t.color.unwrap()].format,
        TextureFormat::Rgba8Srgb
    );
    assert_eq!(
        first.images[t.linear.unwrap()].format,
        TextureFormat::Rgba8Linear
    );
    assert_ne!(t.linear, t.normal);
    for vertex in &first.meshes[0].vertices {
        assert_eq!(vertex[8..12], [1., 0., 0., 1.]);
    }
    assert!(cook_gltf(root, source, &cache).unwrap().cache_hit);
    fs::write(root.join("map.png"), png(1, 1, vec![0, 0, 255, 255])).unwrap();
    let changed = cook_gltf(root, source, &cache).unwrap();
    assert_ne!(changed.metadata.fingerprint, first.metadata.fingerprint);
    fs::remove_file(root.join(source)).unwrap();
    fs::remove_file(root.join("mesh.bin")).unwrap();
    fs::remove_file(root.join("map.png")).unwrap();
    let loaded = load_model(&cache, &first.metadata.fingerprint).unwrap();
    assert_eq!(loaded.images, first.images);
}
#[test]
fn data_uri_and_buffer_view_images_are_imported_and_missing_uv_is_rejected() {
    use base64::{Engine, engine::general_purpose::STANDARD};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    textured_fixture(root);
    let path = root.join("model.gltf");
    let mut doc: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let image = fs::read(root.join("map.png")).unwrap();
    doc["images"][0] = json!({"uri":format!("data:image/png;base64,{}",STANDARD.encode(&image))});
    fs::write(&path, doc.to_string()).unwrap();
    let a = incant_assets::import_gltf(root, Path::new("model.gltf")).unwrap();
    let mut bytes = fs::read(root.join("mesh.bin")).unwrap();
    let offset = bytes.len();
    bytes.extend(&image);
    fs::write(root.join("mesh.bin"), &bytes).unwrap();
    doc["buffers"][0]["byteLength"] = json!(bytes.len());
    doc["bufferViews"]
        .as_array_mut()
        .unwrap()
        .push(json!({"buffer":0,"byteOffset":offset,"byteLength":image.len()}));
    doc["images"][0] = json!({"bufferView":2,"mimeType":"image/png"});
    fs::write(&path, doc.to_string()).unwrap();
    let b = incant_assets::import_gltf(root, Path::new("model.gltf")).unwrap();
    assert_eq!(a.images, b.images);
    doc["meshes"][0]["primitives"][0]["attributes"]
        .as_object_mut()
        .unwrap()
        .remove("TEXCOORD_0");
    fs::write(&path, doc.to_string()).unwrap();
    assert!(incant_assets::import_gltf(root, Path::new("model.gltf")).is_err());
}
