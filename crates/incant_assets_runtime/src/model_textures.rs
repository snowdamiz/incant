use crate::{Result, invalid};
use serde::{Deserialize, Serialize};
/// glTF texture indices stay stable in material JSON. One source can be used for
/// both color and numeric maps, requiring distinct color space and mip filtering.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelTexture {
    pub color: Option<usize>,
    pub linear: Option<usize>,
    pub normal: Option<usize>,
    pub mag_filter: Option<u32>,
    pub min_filter: Option<u32>,
    pub wrap_s: u32,
    pub wrap_t: u32,
}
pub(crate) fn validate_textures(textures: &[ModelTexture], count: usize) -> Result<()> {
    for t in textures {
        if [t.color, t.linear, t.normal]
            .into_iter()
            .flatten()
            .any(|i| i >= count)
            || [t.color, t.linear, t.normal].iter().all(Option::is_none)
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
