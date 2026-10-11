use super::unsupported;
use crate::{
    AssetError, MAX_SOURCE_BYTES, ModelTexture, Result, SourceSet, Texture, TextureUsage,
    import_image, invalid,
};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
use ufbx::{Material, MaterialMap, ShaderType};

pub(super) struct Materials<'a> {
    sources: &'a mut SourceSet,
    source: &'a Path,
    textures: Vec<ModelTexture>,
    images: Vec<Texture>,
    variants: BTreeMap<(u32, TextureUsage), usize>,
    decoded_bytes: usize,
}
impl<'a> Materials<'a> {
    pub fn new(sources: &'a mut SourceSet, source: &'a Path) -> Self {
        Self {
            sources,
            source,
            textures: Vec::new(),
            images: Vec::new(),
            variants: BTreeMap::new(),
            decoded_bytes: 0,
        }
    }
    pub fn finish(self) -> (Vec<ModelTexture>, Vec<Texture>) {
        (self.textures, self.images)
    }
    pub fn import(&mut self, m: &Material) -> Result<Value> {
        if m.element.name.len() > 4096 {
            return Err(AssetError::Limit("FBX material name"));
        }
        let f = &m.fbx;
        let p = &m.pbr;
        let (base, base_factor, emission, emission_factor, normal, metallic, roughness, alpha) =
            match m.shader_type {
                ShaderType::FbxLambert => {
                    for (color, factor, name) in [
                        (
                            &f.ambient_color,
                            &f.ambient_factor,
                            "ambient material color",
                        ),
                        (
                            &f.reflection_color,
                            &f.reflection_factor,
                            "reflection material color",
                        ),
                    ] {
                        if nonzero_color(color, factor) {
                            return Err(unsupported(name));
                        }
                    }
                    if f.bump.texture.is_some()
                        || f.displacement.texture.is_some()
                        || f.vector_displacement.texture.is_some()
                    {
                        return Err(unsupported("bump/displacement maps"));
                    }
                    let transparency =
                        color(&f.transparency_color, scalar(&f.transparency_factor))?;
                    if transparency[0] != transparency[1] || transparency[1] != transparency[2] {
                        return Err(unsupported("colored transparency"));
                    }
                    (
                        &f.diffuse_color,
                        &f.diffuse_factor,
                        &f.emission_color,
                        &f.emission_factor,
                        &f.normal_map,
                        0.,
                        1.,
                        1. - transparency[0],
                    )
                }
                // ufbx's explicitly requested Blender exporter interpretation restores
                // the source metallic/roughness factors from its FBX property encoding.
                ShaderType::BlenderPhong => (
                    &p.base_color,
                    &p.base_factor,
                    &p.emission_color,
                    &p.emission_factor,
                    &p.normal_map,
                    scalar(&p.metalness),
                    scalar(&p.roughness),
                    scalar(&p.opacity),
                ),
                _ => {
                    return Err(unsupported(format!(
                        "material {:?} ({:?}); use a supported Lambert or Blender metallic/roughness material",
                        m.element.name, m.shader_type
                    )));
                }
            };
        let advanced = &m.features;
        if [
            m.shader_type != ShaderType::FbxLambert && advanced.transmission.enabled,
            advanced.coat.enabled,
            advanced.sheen.enabled,
            advanced.matte.enabled,
            advanced.unlit.enabled,
            advanced.diffuse_roughness.enabled,
            advanced.thin_walled.enabled,
            advanced.caustics.enabled,
            advanced.exit_to_background.enabled,
        ]
        .into_iter()
        .any(|v| v)
        {
            return Err(unsupported("advanced material lobes"));
        }
        let allowed: [(&MaterialMap, TextureUsage); 3] = [
            (base, TextureUsage::Color),
            (emission, TextureUsage::Color),
            (normal, TextureUsage::Normal),
        ];
        let ids: BTreeSet<_> = allowed
            .iter()
            .filter_map(|(v, _)| v.texture.as_ref().map(|t| t.element.typed_id))
            .collect();
        // Includes disabled and unused bindings: none are silently discarded.
        for binding in &m.textures {
            if !ids.contains(&binding.texture.element.typed_id)
                || !matches!(
                    binding.material_prop.as_ref(),
                    "DiffuseColor" | "EmissiveColor" | "NormalMap"
                )
            {
                return Err(unsupported(format!(
                    "material texture channel {}",
                    binding.material_prop
                )));
            }
        }
        for factor in [
            base_factor,
            emission_factor,
            &p.metalness,
            &p.roughness,
            &p.opacity,
            &f.transparency_factor,
            &f.transparency_color,
        ] {
            if factor.texture.is_some() {
                return Err(unsupported(
                    "factor/transparency/metallic/roughness textures",
                ));
            }
        }
        let base_color = color(base, scalar(base_factor))?;
        let emissive = color(emission, scalar(emission_factor))?;
        if ![metallic, roughness, alpha].into_iter().all(unit) {
            return Err(invalid("FBX material factors outside normalized range"));
        }
        let mut value = json!({"name":m.element.name.as_ref(),"pbrMetallicRoughness":{"baseColorFactor":[base_color[0],base_color[1],base_color[2],alpha],"metallicFactor":metallic,"roughnessFactor":roughness},"emissiveFactor":emissive,"doubleSided":m.features.double_sided.enabled,"alphaMode":if alpha<1. {"BLEND"} else {"OPAQUE"}});
        for (slot, (map, usage)) in allowed.into_iter().enumerate() {
            if let Some(texture) = &map.texture {
                if !map.texture_enabled || map.feature_disabled {
                    return Err(unsupported("disabled texture binding"));
                }
                let index = self.texture(texture, usage)?;
                let target = if slot == 0 {
                    &mut value["pbrMetallicRoughness"]
                } else {
                    &mut value
                };
                target[match slot {
                    0 => "baseColorTexture",
                    1 => "emissiveTexture",
                    _ => "normalTexture",
                }] = json!({"index":index});
            }
        }
        Ok(value)
    }
    fn texture(&mut self, t: &ufbx::Texture, usage: TextureUsage) -> Result<usize> {
        if t.type_ != ufbx::TextureType::File || t.has_uv_transform || !t.has_file {
            return Err(unsupported(
                "non-file texture or texture-coordinate transform",
            ));
        }
        let key = (t.element.typed_id, usage);
        if let Some(&id) = self.variants.get(&key) {
            return Ok(id);
        }
        if self.images.len() >= 256 {
            return Err(AssetError::Limit("FBX model images"));
        }
        let bytes = if !t.content.is_empty() {
            t.content.to_vec()
        } else {
            // FBX exporters use Windows separators regardless of source host.
            // Absolute legacy paths are never used as fallbacks for relative ones.
            let relative = t.relative_filename.replace('\\', "/");
            if relative.is_empty() || relative.starts_with('/') || relative.contains([':', '\0']) {
                return Err(invalid("FBX image requires a project-relative filename"));
            }
            self.sources
                .read(&self.source.parent().unwrap_or(Path::new("")).join(relative))?
        };
        let image = import_image(&bytes, usage)?;
        self.decoded_bytes = self
            .decoded_bytes
            .checked_add(image.levels.iter().map(Vec::len).sum::<usize>())
            .filter(|n| *n <= MAX_SOURCE_BYTES)
            .ok_or(AssetError::Limit("decoded FBX model textures"))?;
        let image_id = self.images.len();
        self.images.push(image);
        let mut record = ModelTexture {
            color: None,
            linear: None,
            normal: None,
            mag_filter: None,
            min_filter: None,
            wrap_s: wrap(t.wrap_u),
            wrap_t: wrap(t.wrap_v),
        };
        match usage {
            TextureUsage::Color => record.color = Some(image_id),
            TextureUsage::Linear => record.linear = Some(image_id),
            TextureUsage::Normal => record.normal = Some(image_id),
        }
        let id = self.textures.len();
        self.textures.push(record);
        self.variants.insert(key, id);
        Ok(id)
    }
}
fn scalar(map: &MaterialMap) -> f64 {
    map.value_vec4.x
}
fn color(map: &MaterialMap, factor: f64) -> Result<[f64; 3]> {
    let v = map.value_vec4;
    let color = [v.x * factor, v.y * factor, v.z * factor];
    if !color.into_iter().all(unit) || !factor.is_finite() {
        return Err(invalid("FBX color outside normalized range"));
    }
    Ok(color)
}
fn unit(v: f64) -> bool {
    (0. ..=1.).contains(&v)
}
fn nonzero_color(color: &MaterialMap, factor: &MaterialMap) -> bool {
    let v = color.value_vec4;
    [v.x, v.y, v.z]
        .into_iter()
        .any(|v| v * scalar(factor) != 0.)
}
fn wrap(v: ufbx::WrapMode) -> u32 {
    match v {
        ufbx::WrapMode::Repeat => 10497,
        ufbx::WrapMode::Clamp => 33071,
    }
}
