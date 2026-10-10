//! Bounded sprite geometry and document values. No GPU, filesystem or mutation path.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SpriteSampling {
    #[default]
    Nearest,
    Linear,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SpriteMode {
    #[default]
    Simple,
    NineSlice {
        /// Source-pixel widths of left, right, top and bottom borders.
        border: [u32; 4],
        /// Unstretched border density. Small targets proportionally compress borders.
        pixels_per_unit: f64,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SpriteRenderer {
    /// Stable ID of a cooked color texture. Source files are unnecessary at runtime.
    pub texture: String,
    /// Top-left source pixel x/y followed by width/height. No implicit cropping.
    pub region: [u32; 4],
    /// Local width and height in metres, before the entity's inherited transform.
    #[schemars(extend("x-incant-unit" = "m"))]
    pub size: [f64; 2],
    /// Normalized local anchor measured from the bottom left.
    pub pivot: [f64; 2],
    /// Linear-light RGBA multiplier; alpha is straight in the document.
    pub tint: [f64; 4],
    #[serde(default)]
    pub flip_x: bool,
    #[serde(default)]
    pub flip_y: bool,
    /// Lower layers draw first. Equal layer/order uses the stable entity ID.
    #[serde(default)]
    pub layer: i16,
    #[serde(default)]
    pub order: i32,
    #[serde(default)]
    pub sampling: SpriteSampling,
    #[serde(default)]
    pub mode: SpriteMode,
}

#[derive(Debug, Clone, PartialEq, Error)]
pub enum SpriteError {
    #[error("sprite size must be finite, positive and at most 1000000 metres per axis")]
    Size,
    #[error("sprite pivot and tint must be finite normalized values")]
    Normalized,
    #[error(
        "sprite source region must be nonempty and fit inside a texture of at most 8192 pixels per axis"
    )]
    Region,
    #[error(
        "nine-slice borders must fit the source region and density must be finite in 0.000001..1000000"
    )]
    Slice,
}

impl SpriteRenderer {
    pub fn validate(&self) -> Result<(), SpriteError> {
        if self
            .size
            .iter()
            .any(|v| !v.is_finite() || *v <= 0. || *v > 1_000_000.)
        {
            return Err(SpriteError::Size);
        }
        if self
            .pivot
            .iter()
            .chain(&self.tint)
            .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
        {
            return Err(SpriteError::Normalized);
        }
        let [x, y, w, h] = self.region;
        if w == 0
            || h == 0
            || u64::from(x) + u64::from(w) > 8192
            || u64::from(y) + u64::from(h) > 8192
        {
            return Err(SpriteError::Region);
        }
        if let SpriteMode::NineSlice {
            border: [left, right, top, bottom],
            pixels_per_unit,
        } = self.mode
            && (!pixels_per_unit.is_finite()
                || !(0.000001..=1_000_000.).contains(&pixels_per_unit)
                || u64::from(left) + u64::from(right) > u64::from(w)
                || u64::from(top) + u64::from(bottom) > u64::from(h))
        {
            return Err(SpriteError::Slice);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpriteVertex {
    pub position: [f64; 2],
    /// Texture-normalized top-left-origin coordinates.
    pub uv: [f64; 2],
}

/// At most 54 vertices. No authored value is changed; flipping affects UVs only.
/// Degenerate middle cells (a target smaller than its borders) are omitted.
pub fn tessellate(
    sprite: &SpriteRenderer,
    texture_size: [u32; 2],
) -> Result<Vec<SpriteVertex>, SpriteError> {
    sprite.validate()?;
    let [x, y, w, h] = sprite.region;
    if texture_size.contains(&0)
        || texture_size.iter().any(|v| *v > 8192)
        || u64::from(x) + u64::from(w) > u64::from(texture_size[0])
        || u64::from(y) + u64::from(h) > u64::from(texture_size[1])
    {
        return Err(SpriteError::Region);
    }
    let (px, py, tx, ty) = match sprite.mode {
        SpriteMode::Simple => (
            vec![0., sprite.size[0]],
            vec![0., sprite.size[1]],
            vec![0., f64::from(w)],
            vec![0., f64::from(h)],
        ),
        SpriteMode::NineSlice {
            border: [left, right, top, bottom],
            pixels_per_unit,
        } => {
            // Geometry goes bottom to top; source image rows go top to bottom.
            let [left, right, top, bottom] = [left, right, top, bottom].map(f64::from);
            let (left, right) = if sprite.flip_x {
                (right, left)
            } else {
                (left, right)
            };
            let (bottom, top) = if sprite.flip_y {
                (top, bottom)
            } else {
                (bottom, top)
            };
            (
                cuts(
                    sprite.size[0],
                    left / pixels_per_unit,
                    right / pixels_per_unit,
                ),
                cuts(
                    sprite.size[1],
                    bottom / pixels_per_unit,
                    top / pixels_per_unit,
                ),
                vec![0., left, f64::from(w) - right, f64::from(w)],
                vec![0., bottom, f64::from(h) - top, f64::from(h)],
            )
        }
    };
    let mut vertices = Vec::with_capacity(54);
    for j in 0..py.len() - 1 {
        for i in 0..px.len() - 1 {
            if px[i] == px[i + 1] || py[j] == py[j + 1] {
                continue;
            }
            for [a, b] in [
                [i, j],
                [i + 1, j],
                [i + 1, j + 1],
                [i, j],
                [i + 1, j + 1],
                [i, j + 1],
            ] {
                let u = if sprite.flip_x {
                    f64::from(w) - tx[a]
                } else {
                    tx[a]
                };
                let v = if sprite.flip_y {
                    ty[b]
                } else {
                    f64::from(h) - ty[b]
                };
                vertices.push(SpriteVertex {
                    position: [
                        px[a] - sprite.size[0] * sprite.pivot[0],
                        py[b] - sprite.size[1] * sprite.pivot[1],
                    ],
                    uv: [
                        (f64::from(x) + u) / f64::from(texture_size[0]),
                        (f64::from(y) + v) / f64::from(texture_size[1]),
                    ],
                });
            }
        }
    }
    Ok(vertices)
}
fn cuts(size: f64, low: f64, high: f64) -> Vec<f64> {
    if low + high >= size {
        let middle = size * (low / (low + high));
        vec![0., middle, middle, size]
    } else {
        vec![0., low, size - high, size]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sprite() -> SpriteRenderer {
        SpriteRenderer {
            texture: "test".into(),
            region: [16, 32, 32, 16],
            size: [4., 2.],
            pivot: [0.25, 0.5],
            tint: [1.; 4],
            flip_x: false,
            flip_y: false,
            layer: 0,
            order: 0,
            sampling: SpriteSampling::Nearest,
            mode: SpriteMode::Simple,
        }
    }
    #[test]
    fn subregion_anchor_and_flips_preserve_geometry_and_source_coordinates() {
        let mut s = sprite();
        let original = s.clone();
        let a = tessellate(&s, [128, 64]).unwrap();
        assert_eq!(a.len(), 6);
        assert_eq!(
            a[0],
            SpriteVertex {
                position: [-1., -1.],
                uv: [0.125, 0.75]
            }
        );
        assert_eq!(
            a[2],
            SpriteVertex {
                position: [3., 1.],
                uv: [0.375, 0.5]
            }
        );
        assert_eq!(s, original);
        s.flip_x = true;
        s.flip_y = true;
        let b = tessellate(&s, [128, 64]).unwrap();
        for (a, b) in a.iter().zip(&b) {
            assert_eq!(a.position, b.position);
        }
        assert_eq!(b[0].uv, [0.375, 0.5]);
        assert_eq!(b[2].uv, [0.125, 0.75]);
    }
    #[test]
    fn nine_slice_preserves_border_density_and_compresses_small_targets() {
        let mut s = sprite();
        s.mode = SpriteMode::NineSlice {
            border: [4, 8, 2, 6],
            pixels_per_unit: 4.,
        };
        let a = tessellate(&s, [128, 64]).unwrap();
        assert_eq!(a.len(), 36); // vertical centre is zero height
        assert!(a.iter().any(|v| v.position == [0., 0.5]));
        s.flip_x = true;
        s.flip_y = true;
        let flipped = tessellate(&s, [128, 64]).unwrap();
        // The asymmetric right/top borders move to the left/bottom without stretching.
        assert_eq!(flipped[1].position, [1., -1.]);
        assert_eq!(flipped[1].uv, [40. / 128., 32. / 64.]);
        assert_eq!(flipped[2].position, [1., -0.5]);
        assert_eq!(flipped[2].uv, [40. / 128., 34. / 64.]);
        s.flip_x = false;
        s.flip_y = false;
        s.size = [1.5, 1.];
        let b = tessellate(&s, [128, 64]).unwrap();
        assert_eq!(b.len(), 24);
        assert!(b.iter().all(|v| v.position[0] >= -0.375
            && v.position[0] <= 1.125
            && v.position[1] >= -0.5
            && v.position[1] <= 0.5));
        let area: f64 = b
            .as_chunks::<3>()
            .0
            .iter()
            .map(|t| {
                let a = t[0].position;
                let b = t[1].position;
                let c = t[2].position;
                ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])) * 0.5
            })
            .sum();
        assert!((area - 1.5).abs() < 1e-12);
    }
    #[test]
    fn malformed_and_out_of_bounds_regions_fail_before_geometry_allocation() {
        let mut s = sprite();
        assert!(matches!(tessellate(&s, [32, 64]), Err(SpriteError::Region)));
        s.region = [u32::MAX, 0, 16, 16];
        assert!(matches!(s.validate(), Err(SpriteError::Region)));
        s = sprite();
        s.size[0] = f64::NAN;
        assert!(matches!(s.validate(), Err(SpriteError::Size)));
        s = sprite();
        s.tint[3] = 1.01;
        assert!(matches!(s.validate(), Err(SpriteError::Normalized)));
        s = sprite();
        s.mode = SpriteMode::NineSlice {
            border: [30, 30, 0, 0],
            pixels_per_unit: 32.,
        };
        assert!(matches!(s.validate(), Err(SpriteError::Slice)));
    }
}
