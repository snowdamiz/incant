#![cfg(feature = "visual")]

use incant_assets_runtime::{
    AssetError, ModelNode, Texture, TextureFormat, decode_ktx2, decode_mesh, decode_model,
};

#[test]
fn empty_and_nonformat_buffers_fail_without_source_or_import_dependencies() {
    for bytes in [b"".as_slice(), b"not a cooked asset".as_slice()] {
        assert!(decode_mesh(bytes).is_err());
        assert!(decode_ktx2(bytes).is_err());
        assert!(decode_model(bytes, &"0".repeat(64)).is_err());
    }
}
#[test]
fn shared_value_validation_retains_dimension_mip_float_and_hierarchy_bounds() {
    let mut texture = Texture {
        width: 1,
        height: 1,
        format: TextureFormat::Rgba32Float,
        levels: vec![vec![0; 16]],
    };
    texture.validate().unwrap();
    texture.levels[0][..4].copy_from_slice(&f32::NAN.to_le_bytes());
    assert!(matches!(texture.validate(), Err(AssetError::Invalid(_))));
    texture.width = 8193;
    assert!(matches!(
        texture.validate(),
        Err(AssetError::Limit("texture dimensions"))
    ));
    texture.width = 1;
    texture.levels.clear();
    assert!(matches!(texture.validate(), Err(AssetError::Invalid(_))));
    let mut node = ModelNode {
        name: None,
        children: vec![1],
        meshes: vec![],
        transform: [[0.; 4]; 4],
    };
    assert!(incant_assets_runtime::model_node::validate_nodes(&[node.clone()]).is_err());
    node.children = vec![0];
    assert!(incant_assets_runtime::model_node::validate_nodes(&[node.clone()]).is_err());
    node.children.clear();
    incant_assets_runtime::model_node::validate_nodes(&[node]).unwrap();
}
