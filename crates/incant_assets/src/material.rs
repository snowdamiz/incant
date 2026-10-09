//! Typed core glTF metallic-roughness semantics. Raw metadata remains on disk for
//! round trips; consumers receive only checked factors and resolved image roles.
use crate::{AssetError, ModelTexture, Result, Texture, TextureFormat, TextureUsage, invalid};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum AlphaMode {
    #[default]
    Opaque,
    Mask,
    Blend,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaterialTexture {
    pub texture: usize,
    pub image: usize,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    pub base_color: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
    pub emissive: [f32; 3],
    pub normal_scale: f32,
    pub occlusion_strength: f32,
    pub alpha_mode: AlphaMode,
    pub alpha_cutoff: f32,
    pub double_sided: bool,
    /// Base color, metallic/roughness, normal, occlusion, emissive, in that order.
    pub maps: [Option<MaterialTexture>; 5],
}
impl Default for Material {
    fn default() -> Self {
        Self {
            base_color: [1.; 4],
            metallic: 1.,
            roughness: 1.,
            emissive: [0.; 3],
            normal_scale: 1.,
            occlusion_strength: 1.,
            alpha_mode: AlphaMode::Opaque,
            alpha_cutoff: 0.5,
            double_sided: false,
            maps: [None; 5],
        }
    }
}

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
struct RawMaterial {
    name: Option<String>,
    extras: Option<Value>,
    extensions: BTreeMap<String, Value>,
    pbr_metallic_roughness: RawPbr,
    normal_texture: Option<RawTexture>,
    occlusion_texture: Option<RawTexture>,
    emissive_texture: Option<RawTexture>,
    emissive_factor: [f32; 3],
    alpha_mode: Option<AlphaMode>,
    alpha_cutoff: Option<f32>,
    double_sided: bool,
}
#[derive(Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
struct RawPbr {
    extras: Option<Value>,
    extensions: BTreeMap<String, Value>,
    base_color_factor: [f32; 4],
    base_color_texture: Option<RawTexture>,
    metallic_factor: f32,
    roughness_factor: f32,
    metallic_roughness_texture: Option<RawTexture>,
}
impl Default for RawPbr {
    fn default() -> Self {
        Self {
            extras: None,
            extensions: BTreeMap::new(),
            base_color_factor: [1.; 4],
            base_color_texture: None,
            metallic_factor: 1.,
            roughness_factor: 1.,
            metallic_roughness_texture: None,
        }
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RawTexture {
    index: usize,
    #[serde(default)]
    tex_coord: u32,
    #[serde(default)]
    scale: Option<f32>,
    #[serde(default)]
    strength: Option<f32>,
    #[serde(default)]
    extensions: BTreeMap<String, Value>,
    #[serde(default, rename = "extras")]
    _extras: Option<Value>,
}
fn extensions(map: &BTreeMap<String, Value>) -> Result<()> {
    if let Some(name) = map.keys().next() {
        Err(AssetError::Unsupported(format!(
            "material extension {name}"
        )))
    } else {
        Ok(())
    }
}
fn unit(value: f32) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

pub(crate) fn resolve(
    values: &[Value],
    textures: &[ModelTexture],
    images: &[Texture],
) -> Result<Vec<Material>> {
    if values.len() > 10_000 {
        return Err(AssetError::Limit("model materials"));
    }
    values
        .iter()
        .map(|value| {
            let raw: RawMaterial = serde_json::from_value(value.clone())?;
            // Names and application extras carry no executable/material semantics.
            let _ = (&raw.name, &raw.extras, &raw.pbr_metallic_roughness.extras);
            extensions(&raw.extensions)?;
            let pbr = &raw.pbr_metallic_roughness;
            extensions(&pbr.extensions)?;
            let normal_scale = raw
                .normal_texture
                .as_ref()
                .and_then(|t| t.scale)
                .unwrap_or(1.);
            let occlusion_strength = raw
                .occlusion_texture
                .as_ref()
                .and_then(|t| t.strength)
                .unwrap_or(1.);
            let alpha_cutoff = raw.alpha_cutoff.unwrap_or(0.5);
            if !pbr.base_color_factor.into_iter().all(unit)
                || !unit(pbr.metallic_factor)
                || !unit(pbr.roughness_factor)
                || !raw.emissive_factor.into_iter().all(unit)
                || !normal_scale.is_finite()
                || !unit(occlusion_strength)
                || !alpha_cutoff.is_finite()
                || alpha_cutoff < 0.
                || (raw.alpha_cutoff.is_some() && raw.alpha_mode.is_none())
            {
                return Err(invalid("material factor outside glTF range"));
            }
            let mut maps = [None; 5];
            let slots = [
                (&pbr.base_color_texture, TextureUsage::Color),
                (&pbr.metallic_roughness_texture, TextureUsage::Linear),
                (&raw.normal_texture, TextureUsage::Normal),
                (&raw.occlusion_texture, TextureUsage::Linear),
                (&raw.emissive_texture, TextureUsage::Color),
            ];
            for (slot, (info, usage)) in slots.into_iter().enumerate() {
                let Some(info) = info else { continue };
                extensions(&info.extensions)?;
                if info.tex_coord != 0 {
                    return Err(AssetError::Unsupported(
                        "material texture coordinates beyond UV0".into(),
                    ));
                }
                if (info.scale.is_some() && slot != 2) || (info.strength.is_some() && slot != 3) {
                    return Err(invalid(
                        "texture parameter does not apply to this material role",
                    ));
                }
                let binding = textures
                    .get(info.index)
                    .ok_or_else(|| invalid("material texture index outside model"))?;
                let image = match usage {
                    TextureUsage::Color => binding.color,
                    TextureUsage::Linear => binding.linear,
                    TextureUsage::Normal => binding.normal,
                }
                .ok_or_else(|| invalid("material texture has no cooked variant for its role"))?;
                let texture = images
                    .get(image)
                    .ok_or_else(|| invalid("material image index outside model"))?;
                let format_ok = match usage {
                    TextureUsage::Color => matches!(
                        texture.format,
                        TextureFormat::Rgba8Srgb | TextureFormat::Rgba32Float
                    ),
                    TextureUsage::Linear | TextureUsage::Normal => matches!(
                        texture.format,
                        TextureFormat::Rgba8Linear | TextureFormat::Rgba32Float
                    ),
                };
                if !format_ok {
                    return Err(invalid(
                        "material texture role does not match its cooked color space",
                    ));
                }
                maps[slot] = Some(MaterialTexture {
                    texture: info.index,
                    image,
                });
            }
            Ok(Material {
                base_color: pbr.base_color_factor,
                metallic: pbr.metallic_factor,
                roughness: pbr.roughness_factor,
                emissive: raw.emissive_factor,
                normal_scale,
                occlusion_strength,
                alpha_mode: raw.alpha_mode.unwrap_or_default(),
                alpha_cutoff,
                double_sided: raw.double_sided,
                maps,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn defaults_and_material_roles_keep_color_spaces_and_sampler_identity() {
        assert_eq!(
            resolve(&[json!({})], &[], &[]).unwrap(),
            [Material::default()]
        );
        let textures = [ModelTexture {
            color: Some(0),
            linear: Some(1),
            normal: Some(2),
            mag_filter: None,
            min_filter: None,
            wrap_s: 10497,
            wrap_t: 10497,
        }];
        let images: Vec<_> = [
            TextureFormat::Rgba8Srgb,
            TextureFormat::Rgba8Linear,
            TextureFormat::Rgba8Linear,
        ]
        .into_iter()
        .map(|format| Texture {
            width: 1,
            height: 1,
            format,
            levels: vec![vec![255; 4]],
        })
        .collect();
        let raw = json!({"pbrMetallicRoughness":{"baseColorFactor":[0.1,0.2,0.3,0.4],"metallicFactor":0.2,"roughnessFactor":0.7,"baseColorTexture":{"index":0},"metallicRoughnessTexture":{"index":0}},"normalTexture":{"index":0,"scale":-2},"occlusionTexture":{"index":0,"strength":0.5},"emissiveTexture":{"index":0},"emissiveFactor":[0.1,0.2,0.3],"alphaMode":"MASK","alphaCutoff":1.2,"doubleSided":true});
        let material = resolve(&[raw], &textures, &images).unwrap().remove(0);
        assert_eq!(material.maps.map(|m| m.unwrap().image), [0, 1, 2, 1, 0]);
        assert!(material.maps.into_iter().all(|m| m.unwrap().texture == 0));
        assert_eq!(material.base_color, [0.1, 0.2, 0.3, 0.4]);
        assert_eq!(material.normal_scale, -2.);
        assert_eq!(material.alpha_cutoff, 1.2);
        assert_eq!(material.alpha_mode, AlphaMode::Mask);
        assert!(material.double_sided);
    }
    #[test]
    fn malformed_or_unresolved_materials_fail_before_gpu_upload() {
        for bad in [
            json!({"pbrMetallicRoughness":{"metallicFactor":2}}),
            json!({"emissiveFactor":[2,0,0]}),
            json!({"alphaCutoff":0.5}),
            json!({"alphaMode":"UNKNOWN"}),
            json!({"extensions":{"KHR_materials_unlit":{}}}),
            json!({"normalTexture":{"index":0}}),
            json!({"pbrMetallicRoughness":{"baseColorTexture":{"index":0,"texCoord":1}}}),
        ] {
            assert!(resolve(&[bad], &[], &[]).is_err());
        }
        let texture = ModelTexture {
            color: Some(0),
            linear: None,
            normal: None,
            mag_filter: None,
            min_filter: None,
            wrap_s: 10497,
            wrap_t: 10497,
        };
        let raw = json!({"pbrMetallicRoughness":{"baseColorTexture":{"index":0}}});
        let image = Texture {
            width: 1,
            height: 1,
            format: TextureFormat::Rgba8Linear,
            levels: vec![vec![255; 4]],
        };
        assert!(resolve(&[raw], &[texture], &[image]).is_err());
    }
}
