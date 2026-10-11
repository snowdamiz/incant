use crate::{Result, invalid};
use serde::{Deserialize, Serialize};
/// glTF texture indices stay stable in material JSON. One source can be used for
/// both color and numeric maps, requiring distinct color space and mip filtering.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelTexture {
    pub color: Option<usize>,
    /// Alpha-independent Color variant for opaque base color and emissive maps.
    /// Introduced by INCMOD03; old formats omit this role entirely.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_unweighted: Option<usize>,
    pub linear: Option<usize>,
    pub normal: Option<usize>,
    pub mag_filter: Option<u32>,
    pub min_filter: Option<u32>,
    pub wrap_s: u32,
    pub wrap_t: u32,
}
pub(crate) fn validate_textures(
    textures: &[ModelTexture],
    count: usize,
    by_role: bool,
) -> Result<()> {
    for t in textures {
        if (!by_role && t.color_unweighted.is_some())
            || [t.color, t.color_unweighted, t.linear, t.normal]
                .into_iter()
                .flatten()
                .any(|i| i >= count)
            || [t.color, t.color_unweighted, t.linear, t.normal]
                .iter()
                .all(Option::is_none)
            || t.mag_filter.is_some_and(|v| ![9728, 9729].contains(&v))
            || t.min_filter
                .is_some_and(|v| ![9728, 9729, 9984, 9985, 9986, 9987].contains(&v))
            || ![33071, 33648, 10497].contains(&t.wrap_s)
            || ![33071, 33648, 10497].contains(&t.wrap_t)
        {
            return Err(invalid("invalid model texture binding"));
        }
    }
    Ok(())
}

pub(crate) fn share_color_bases(
    textures: &[ModelTexture],
    images: &mut [crate::Texture],
) -> Result<()> {
    let mut checked = std::collections::HashSet::new();
    for texture in textures {
        let (Some(coverage), Some(independent)) = (texture.color, texture.color_unweighted) else {
            continue;
        };
        if coverage == independent || !checked.insert((coverage, independent)) {
            continue;
        }
        // Indices are checked by validate_textures before this helper runs.
        if coverage < independent {
            let (first, last) = images.split_at_mut(independent);
            last[0].share_base_from(&first[coverage])?;
        } else {
            let (first, last) = images.split_at_mut(coverage);
            first[independent].share_base_from(&last[0])?;
        }
    }
    // Separate source texture/sampler records may reference the same pixels.
    // This one-time bounded hash index restores sharing after serialized KTX
    // chains have been decoded. Equality is still checked before sharing; hash
    // identity is not assumed to prove either content or resource ownership.
    use sha2::{Digest, Sha256};
    let mut bases = std::collections::BTreeMap::new();
    for i in 0..images.len() {
        let image = &images[i];
        let mut hash = Sha256::new();
        hash.update(image.width.to_le_bytes());
        hash.update(image.height.to_le_bytes());
        hash.update([image.format as u8]);
        hash.update(&image.levels[0]);
        let key: [u8; 32] = hash.finalize().into();
        if let Some(&first) = bases.get(&key) {
            let (earlier, later) = images.split_at_mut(i);
            later[0].share_base_from(&earlier[first])?;
        } else {
            bases.insert(key, i);
        }
    }
    Ok(())
}
