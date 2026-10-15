//! Finite GPU-independent ModelMaterialV1 keys and shared placement arithmetic.
//! The fixed color/depth contract is Rgba16Float/Depth32Float. Values are logical
//! selections from engine-owned descriptors, never caller-defined native state.
use crate::SceneError;
use incant_assets_runtime::AlphaMode;
use serde::{Deserialize, Serialize};

pub const MATERIAL_KEY_COUNT: usize = 24;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaterialSamples {
    One,
    Four,
}
impl MaterialSamples {
    pub const fn count(self) -> u32 {
        match self {
            Self::One => 1,
            Self::Four => 4,
        }
    }
}

/// Stable encoding: sample mode ×12 + alpha ×4 + double-sided ×2 + mirrored.
/// The numeric range is closed; future families require a new manifest ABI.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub struct MaterialKey(u8);
impl MaterialKey {
    pub const fn new(
        alpha: AlphaMode,
        double_sided: bool,
        mirrored: bool,
        samples: MaterialSamples,
    ) -> Self {
        let alpha = match alpha {
            AlphaMode::Opaque => 0,
            AlphaMode::Mask => 1,
            AlphaMode::Blend => 2,
        };
        Self(
            match samples {
                MaterialSamples::One => 0,
                MaterialSamples::Four => 12,
            } + alpha * 4
                + double_sided as u8 * 2
                + mirrored as u8,
        )
    }
    pub const fn index(self) -> usize {
        self.0 as usize
    }
    pub const fn alpha(self) -> AlphaMode {
        match (self.0 % 12) / 4 {
            0 => AlphaMode::Opaque,
            1 => AlphaMode::Mask,
            _ => AlphaMode::Blend,
        }
    }
    pub const fn double_sided(self) -> bool {
        self.0 & 2 != 0
    }
    pub const fn mirrored(self) -> bool {
        self.0 & 1 != 0
    }
    pub const fn samples(self) -> MaterialSamples {
        if self.0 < 12 {
            MaterialSamples::One
        } else {
            MaterialSamples::Four
        }
    }
}
impl TryFrom<u8> for MaterialKey {
    type Error = SceneError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if usize::from(value) < MATERIAL_KEY_COUNT {
            Ok(Self(value))
        } else {
            Err(SceneError::Input("unknown ModelMaterialV1 key"))
        }
    }
}
impl From<MaterialKey> for u8 {
    fn from(value: MaterialKey) -> Self {
        value.0
    }
}

/// Exact existing renderer arithmetic, shared by GPU-free key recording. The
/// determinant is evaluated after narrowing to the actual f32 vertex transform.
pub struct PlacementTransform {
    pub world: [[f32; 4]; 4],
    pub normal: [[f32; 4]; 3],
}
impl PlacementTransform {
    pub fn new(world: glam::Mat4) -> Result<Self, SceneError> {
        let inverse = world.inverse();
        if !world.is_finite() || !inverse.is_finite() {
            return Err(SceneError::TransformRange);
        }
        let mut columns = inverse.transpose().to_cols_array_2d();
        columns[0][3] = if world.as_dmat4().determinant() < 0. {
            -1.
        } else {
            1.
        };
        Ok(Self {
            world: world.to_cols_array_2d(),
            normal: [columns[0], columns[1], columns[2]],
        })
    }
    pub fn mirrored(&self) -> bool {
        self.normal[0][3] < 0.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closed_keys_roundtrip_every_supported_state_and_refuse_unknown_values() {
        let mut seen = [false; MATERIAL_KEY_COUNT];
        for samples in [MaterialSamples::One, MaterialSamples::Four] {
            for alpha in [AlphaMode::Opaque, AlphaMode::Mask, AlphaMode::Blend] {
                for double in [false, true] {
                    for mirror in [false, true] {
                        let key = MaterialKey::new(alpha, double, mirror, samples);
                        assert!(!std::mem::replace(&mut seen[key.index()], true));
                        let decoded = MaterialKey::try_from(u8::from(key)).unwrap();
                        assert_eq!(decoded.alpha(), alpha);
                        assert_eq!(decoded.double_sided(), double);
                        assert_eq!(decoded.mirrored(), mirror);
                        assert_eq!(decoded.samples(), samples);
                    }
                }
            }
        }
        assert!(seen.into_iter().all(|value| value));
        for value in 24..=u8::MAX {
            assert!(MaterialKey::try_from(value).is_err());
        }
    }

    #[test]
    fn orientation_tracks_real_narrowed_transform_and_refuses_singular_values() {
        for (scale, mirrored) in [
            ([1., 1., 1.], false),
            ([-1., 1., 1.], true),
            ([-1., -1., 1.], false),
        ] {
            let transform = PlacementTransform::new(glam::Mat4::from_scale(scale.into())).unwrap();
            assert_eq!(transform.mirrored(), mirrored);
        }
        assert!(PlacementTransform::new(glam::Mat4::from_scale([0., 1., 1.].into())).is_err());
        assert!(
            PlacementTransform::new(glam::Mat4::from_scale([f32::NAN, 1., 1.].into())).is_err()
        );
        let tiny = glam::DMat4::from_scale([f64::MIN_POSITIVE, 1., 1.].into());
        assert!(PlacementTransform::new(tiny.as_mat4()).is_err());
    }
}
