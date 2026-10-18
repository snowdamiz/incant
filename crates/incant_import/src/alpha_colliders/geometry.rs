use super::{AlphaColliderError, MAX_ALPHA_COLLIDER_PIXELS, MAX_ALPHA_COLLIDER_RECTANGLES, Result};
use incant_doc::{Collider2D, ColliderShape2D, Entity, SpriteRenderer, Transform};

/// Row-major greedy cover: extend the first unclaimed run right, then down only
/// while every texel is solid/unclaimed. No simplification or hole filling.
pub(super) fn cover(
    rgba: &[u8],
    size: [u32; 2],
    [x, y, width, height]: [u32; 4],
    threshold: u8,
) -> Result<(Vec<[u32; 4]>, usize)> {
    if width == 0
        || height == 0
        || u64::from(x) + u64::from(width) > u64::from(size[0])
        || u64::from(y) + u64::from(height) > u64::from(size[1])
    {
        return Err(AlphaColliderError::Region);
    }
    if u64::from(width) * u64::from(height) > MAX_ALPHA_COLLIDER_PIXELS as u64 {
        return Err(AlphaColliderError::PixelLimit);
    }
    let (w, h) = (width as usize, height as usize);
    let mut mask = Vec::with_capacity(w * h);
    for row in y..y + height {
        for col in x..x + width {
            let offset = (row as usize * size[0] as usize + col as usize) * 4 + 3;
            mask.push(rgba[offset] >= threshold);
        }
    }
    let solid = mask.iter().filter(|&&v| v).count();
    if solid == 0 {
        return Err(AlphaColliderError::EmptyMask);
    }
    let mut rectangles = Vec::new();
    for row in 0..h {
        for col in 0..w {
            if !mask[row * w + col] {
                continue;
            }
            if rectangles.len() == MAX_ALPHA_COLLIDER_RECTANGLES {
                return Err(AlphaColliderError::RectangleLimit);
            }
            let mut right = col + 1;
            while right < w && mask[row * w + right] {
                right += 1;
            }
            let mut bottom = row + 1;
            while bottom < h && (col..right).all(|c| mask[bottom * w + c]) {
                bottom += 1;
            }
            for r in row..bottom {
                mask[r * w + col..r * w + right].fill(false);
            }
            rectangles.push([
                col as u32,
                row as u32,
                (right - col) as u32,
                (bottom - row) as u32,
            ]);
        }
    }
    Ok((rectangles, solid))
}

pub(super) fn collider(
    sprite: &SpriteRenderer,
    transform: &Transform,
    region: [u32; 4],
    [x, y, width, height]: [u32; 4],
    index: usize,
) -> Result<Entity> {
    // Match simple-sprite tessellation: source origin is top-left, geometry
    // origin bottom-left, then mirror UVs within the fixed geometry bounds.
    let mut u = (f64::from(x) + f64::from(width) / 2.) / f64::from(region[2]);
    let mut v = 1. - (f64::from(y) + f64::from(height) / 2.) / f64::from(region[3]);
    if sprite.flip_x {
        u = 1. - u;
    }
    if sprite.flip_y {
        v = 1. - v;
    }
    let local = [
        (u - sprite.pivot[0]) * sprite.size[0],
        (v - sprite.pivot[1]) * sprite.size[1],
    ];
    // Runtime world transforms normalize quaternions. This is the same planar
    // rotation without introducing another math/runtime dependency.
    let [_, _, z, w] = transform.rotation;
    let norm = z * z + w * w;
    let (sin, cos) = (2. * z * w / norm, (w * w - z * z) / norm);
    let mut placement = transform.clone();
    placement.translation[0] += cos * local[0] - sin * local[1];
    placement.translation[1] += sin * local[0] + cos * local[1];
    let collider = Collider2D {
        shape: ColliderShape2D::Box {
            half_extents: [
                sprite.size[0] * f64::from(width) / f64::from(region[2]) / 2.,
                sprite.size[1] * f64::from(height) / f64::from(region[3]) / 2.,
            ],
        },
        ..Collider2D::default()
    };
    let mut entity = Entity::new(format!("Alpha collider {}", index + 1));
    entity
        .components
        .insert("Transform".into(), serde_json::to_value(placement)?);
    entity
        .components
        .insert("Collider2D".into(), serde_json::to_value(collider)?);
    Ok(entity)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_three_by_three_mask_has_exact_nonoverlapping_cover() {
        for bits in 0u32..512 {
            let rgba: Vec<_> = (0..9)
                .flat_map(|i| [0, 0, 0, if bits & (1 << i) != 0 { 255 } else { 0 }])
                .collect();
            let result = cover(&rgba, [3, 3], [0, 0, 3, 3], 128);
            if bits == 0 {
                assert!(matches!(result, Err(AlphaColliderError::EmptyMask)));
                continue;
            }
            let (rects, count) = result.unwrap();
            assert_eq!(count, bits.count_ones() as usize);
            let mut seen = [false; 9];
            for [x, y, w, h] in rects {
                for row in y..y + h {
                    for col in x..x + w {
                        let i = (row * 3 + col) as usize;
                        assert!(!seen[i], "overlap at mask {bits}");
                        seen[i] = true;
                    }
                }
            }
            for (i, actual) in seen.into_iter().enumerate() {
                assert_eq!(actual, bits & (1 << i) != 0, "mask {bits} at pixel {i}");
            }
        }
    }
}
