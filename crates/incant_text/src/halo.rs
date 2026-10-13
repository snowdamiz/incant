//! Halo coverage: a glyph's coverage dilated by a radius, prepared once when the
//! glyph enters an atlas. Drawing a halo then costs what drawing a glyph costs;
//! no dilation happens per frame or per pixel. The halo keeps the glyph's
//! antialiased edge: it is an outline grown from the coverage, not a blur.
use crate::TextError;

/// The largest halo radius in texels. It bounds the preparation work: each halo
/// texel reads at most (2 * ceil(radius) + 1)^2 glyph texels, at most 81.
pub const MAX_HALO_RADIUS: f32 = 4.;

/// Halo radius for a glyph of `font_size` (texels at the raster size): the
/// size times `em`, clamped to `[min, max]`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HaloRule {
    pub em: f32,
    pub min: f32,
    pub max: f32,
}
impl HaloRule {
    /// The radius, refusing a rule outside `0 < min <= max <= MAX_HALO_RADIUS`
    /// or a non-positive font size.
    pub fn radius(&self, font_size: f32) -> Result<f32, TextError> {
        let valid = self.em.is_finite()
            && self.em > 0.
            && self.min.is_finite()
            && self.min > 0.
            && self.max.is_finite()
            && self.min <= self.max
            && self.max <= MAX_HALO_RADIUS
            && font_size.is_finite()
            && font_size > 0.;
        if !valid {
            return Err(TextError::Invalid("halo rule"));
        }
        Ok((font_size * self.em).clamp(self.min, self.max))
    }
}

/// Texels a halo of `radius` adds on every side of its glyph: `ceil(radius)`.
pub(crate) fn margin(radius: f32) -> u32 {
    radius.ceil() as u32
}

/// Write `pixels` (`width` x `height` coverage) dilated by `radius` texels into
/// `out`, `margin(radius)` texels larger on every side. Each halo texel is the
/// largest `clamp(radius + c - d, 0, 1)` over the glyph texels of coverage
/// `c > 0` at centre distance `d`: an antialiased disc of `radius` around every
/// covered texel, its edge moved by the texel's partial coverage, so a straight
/// edge through a half-covered texel grows by exactly `radius`. Every halo
/// texel over the glyph is at least the glyph's coverage. The arithmetic is
/// bounded and plain f32: square roots of integers up to 32, one division by
/// 255, additions, a clamp and a final rounding to 8 bits. Its oracle is local:
/// a renderer CPU test recomputes real halos in f64 and accepts at most one
/// code of difference. Equal bytes on every platform are not verified.
pub(crate) fn dilate(pixels: &[u8], width: u32, height: u32, radius: f32, out: &mut Vec<u8>) {
    debug_assert!(radius > 0. && radius <= MAX_HALO_RADIUS);
    debug_assert_eq!(pixels.len(), width as usize * height as usize);
    // A glyph texel farther than the margin along either axis is at least
    // radius + 1 away from the halo texel centre, where every term is zero.
    let reach = i64::from(margin(radius));
    let side = (2 * reach + 1) as usize;
    let distance: Vec<f32> = (0..side * side)
        .map(|i| {
            let (dx, dy) = ((i % side) as i64 - reach, (i / side) as i64 - reach);
            ((dx * dx + dy * dy) as f32).sqrt()
        })
        .collect();
    let (w, h) = (i64::from(width), i64::from(height));
    let (out_w, out_h) = (w + 2 * reach, h + 2 * reach);
    out.clear();
    out.resize((out_w * out_h) as usize, 0);
    for oy in 0..out_h {
        for ox in 0..out_w {
            // This halo texel's centre, in glyph texel coordinates.
            let (cx, cy) = (ox - reach, oy - reach);
            let mut best = 0f32;
            'search: for sy in (cy - reach).max(0)..=(cy + reach).min(h - 1) {
                for sx in (cx - reach).max(0)..=(cx + reach).min(w - 1) {
                    let c = pixels[(sy * w + sx) as usize];
                    if c == 0 {
                        continue;
                    }
                    let d =
                        distance[(sy - cy + reach) as usize * side + (sx - cx + reach) as usize];
                    let value = (radius + f32::from(c) / 255. - d).clamp(0., 1.);
                    if value > best {
                        best = value;
                        if best >= 1. {
                            break 'search;
                        }
                    }
                }
            }
            out[(oy * out_w + ox) as usize] = (best * 255.).round() as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn halo(pixels: &[u8], width: u32, height: u32, radius: f32) -> (Vec<u8>, u32) {
        let mut out = Vec::new();
        dilate(pixels, width, height, radius, &mut out);
        (out, margin(radius))
    }

    #[test]
    fn one_covered_texel_grows_into_an_antialiased_disc() {
        let (out, margin) = halo(&[255], 1, 1, 1.);
        assert_eq!(margin, 1);
        // Centre and edge neighbours (distance 1) are covered; diagonals at
        // sqrt(2) take 2 - 1.414 = 0.586.
        let diagonal = ((2. - 2f32.sqrt()) * 255.).round() as u8;
        assert_eq!(
            out,
            [
                diagonal, 255, diagonal, 255, 255, 255, diagonal, 255, diagonal
            ]
        );
    }

    #[test]
    fn a_half_covered_edge_grows_by_exactly_the_radius() {
        // A vertical edge through the middle texel column: full | half | empty.
        let row = [255u8, 128, 0];
        let (out, margin) = halo(&row, 3, 1, 2.);
        assert_eq!(margin, 2);
        let middle = &out[2 * 7..3 * 7];
        // The halo's 0.5 level sits radius texels right of the half-covered
        // texel centre: index 1 + 2 in glyph space, 5 in halo space.
        let value = |i: usize| f32::from(middle[i]) / 255.;
        assert!((value(5) - (2. + 128. / 255. - 2.)).abs() < 1. / 255.);
        assert_eq!(middle[6], 0);
        assert!(middle[..5].iter().all(|v| *v == 255));
    }

    #[test]
    fn the_halo_covers_the_glyph_and_grows_with_its_radius() {
        // A small antialiased glyph-like shape with every coverage level.
        let (w, h) = (5u32, 4u32);
        let pixels: Vec<u8> = (0..w * h).map(|i| ((i * 37) % 256) as u8).collect();
        let mut previous: Option<(Vec<u8>, u32)> = None;
        for radius in [1., 1.375, 2., 2.5, MAX_HALO_RADIUS] {
            let (out, margin) = halo(&pixels, w, h, radius);
            assert_eq!(margin, radius.ceil() as u32);
            let out_w = w + 2 * margin;
            assert_eq!(out.len() as u32, out_w * (h + 2 * margin));
            for y in 0..h {
                for x in 0..w {
                    let glyph = pixels[(y * w + x) as usize];
                    let over = out[((y + margin) * out_w + x + margin) as usize];
                    assert!(over >= glyph, "{radius}: {over} < {glyph}");
                }
            }
            if let Some((smaller, small)) = previous {
                let grow = margin - small;
                let small_w = w + 2 * small;
                for (i, v) in smaller.iter().enumerate() {
                    let (x, y) = (i as u32 % small_w + grow, i as u32 / small_w + grow);
                    assert!(out[(y * out_w + x) as usize] >= *v);
                }
            }
            previous = Some((out, margin));
        }
    }

    #[test]
    fn empty_coverage_stays_empty_and_preparation_is_deterministic() {
        let (out, _) = halo(&[0; 12], 4, 3, 2.5);
        assert!(out.iter().all(|v| *v == 0));
        let pixels: Vec<u8> = (0..64).map(|i| (i * 53 % 256) as u8).collect();
        assert_eq!(halo(&pixels, 8, 8, 1.0625), halo(&pixels, 8, 8, 1.0625));
    }

    #[test]
    fn halo_rules_clamp_and_refuse_out_of_bounds() {
        let rule = HaloRule {
            em: 1. / 16.,
            min: 1.,
            max: 2.5,
        };
        assert_eq!(rule.radius(12.).unwrap(), 1.);
        assert_eq!(rule.radius(22.).unwrap(), 1.375);
        assert_eq!(rule.radius(32.).unwrap(), 2.);
        assert_eq!(rule.radius(64.).unwrap(), 2.5);
        for bad in [
            HaloRule { em: 0., ..rule },
            HaloRule { min: 0., ..rule },
            HaloRule { min: 3., ..rule },
            HaloRule {
                max: MAX_HALO_RADIUS + 1.,
                ..rule
            },
            HaloRule {
                em: f32::NAN,
                ..rule
            },
        ] {
            assert!(bad.radius(16.).is_err(), "{bad:?}");
        }
        assert!(rule.radius(0.).is_err() && rule.radius(f32::INFINITY).is_err());
    }
}
