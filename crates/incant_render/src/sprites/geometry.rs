//! Bound interpolated nearest-sampling coordinates without changing source UVs.
//! Integer source bases remain flat across each quad; linear sampling is separate.
use incant_2d::SpriteVertex;

pub(super) struct NearestVertex {
    pub position: [f64; 2],
    pub texel: [f64; 2],
    pub base: [f64; 2],
}

/// Short spans bound both interpolation error and projected endpoint rounding at
/// high magnification. Keep camera-independent retained geometry: camera motion
/// does not require rebuilding it. Total uploaded geometry remains separately capped.
/// Each source axis is at most 8192 texels, split into spans of at most 64.
/// Nine-slice boundaries add at most two spans per axis: 130² × 6 = 101400 vertices.
pub(super) fn nearest(
    geometry: &[SpriteVertex],
    region: [u32; 4],
    texture_size: [u32; 2],
) -> Vec<NearestVertex> {
    let mut output = Vec::new();
    for quad in geometry.as_chunks::<6>().0 {
        let low = quad[0];
        let high = quad[2];
        let source = |v: SpriteVertex, axis: usize| {
            // Tessellation boundaries are authored integer source pixels. Undo
            // normalized-UV division in f64 before crossing the f32 boundary.
            (v.uv[axis] * f64::from(texture_size[axis]) - f64::from(region[axis])).round()
        };
        let x = cuts(source(low, 0), source(high, 0));
        let y = cuts(source(low, 1), source(high, 1));
        for pair_y in y.windows(2) {
            for pair_x in x.windows(2) {
                let base = [pair_x[0].1.min(pair_x[1].1), pair_y[0].1.min(pair_y[1].1)];
                for [a, b] in [[0, 0], [1, 0], [1, 1], [0, 0], [1, 1], [0, 1]] {
                    let points = [pair_x[a], pair_y[b]];
                    output.push(NearestVertex {
                        position: std::array::from_fn(|axis| {
                            low.position[axis]
                                + (high.position[axis] - low.position[axis]) * points[axis].0
                        }),
                        texel: std::array::from_fn(|axis| points[axis].1 - base[axis]),
                        base,
                    });
                }
            }
        }
    }
    output
}

fn cuts(start: f64, end: f64) -> Vec<(f64, f64)> {
    let distance = (end - start).abs();
    let count = (distance / 64.).ceil().max(1.) as usize;
    (0..=count)
        .map(|i| {
            if i == count {
                (1., end)
            } else {
                let part = i as f64 * 64.;
                (
                    part / distance.max(1.),
                    start + (end - start).signum() * part,
                )
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use incant_2d::{SpriteMode, SpriteRenderer, SpriteSampling};

    #[test]
    fn bounded_spans_preserve_full_geometry_and_source_mapping_for_flips_and_slices() {
        for flipped in [false, true] {
            for mode in [
                SpriteMode::Simple,
                SpriteMode::NineSlice {
                    border: [511, 1, 17, 3],
                    pixels_per_unit: 8.,
                },
            ] {
                for size in [[1024., 1024.], [0.5, 0.25]] {
                    let sprite = SpriteRenderer {
                        texture: "test".into(),
                        region: [0, 0, 8192, 8192],
                        size,
                        pivot: [0.25, 0.75],
                        tint: [1.; 4],
                        flip_x: flipped,
                        flip_y: flipped,
                        layer: 0,
                        order: 0,
                        sampling: SpriteSampling::Nearest,
                        mode: mode.clone(),
                    };
                    let original = incant_2d::tessellate(&sprite, [8192, 8192]).unwrap();
                    let vertices = nearest(&original, sprite.region, [8192, 8192]);
                    assert!(vertices.len() <= 101400);
                    assert_eq!(vertices.len() % 6, 0);
                    let mut area = 0.;
                    for triangle in vertices.as_chunks::<3>().0 {
                        let [a, b, c] = [&triangle[0], &triangle[1], &triangle[2]];
                        area += ((b.position[0] - a.position[0]) * (c.position[1] - a.position[1])
                            - (b.position[1] - a.position[1]) * (c.position[0] - a.position[0]))
                            .abs()
                            * 0.5;
                        assert!(triangle.iter().all(|v| v.base == a.base));
                    }
                    assert!((area - size[0] * size[1]).abs() <= 1e-9 * area.max(1.));
                    for vertex in &vertices {
                        assert!(vertex.texel.iter().all(|c| (0.0..=64.0).contains(c)));
                        let texel: [f64; 2] =
                            std::array::from_fn(|i| vertex.base[i] + vertex.texel[i]);
                        // Every new vertex lies on an original quad's mapping,
                        // including asymmetric or collapsed nine-slice borders.
                        assert!(original.as_chunks::<6>().0.iter().any(|quad| {
                            (0..2).all(|i| {
                                let a = quad[0];
                                let b = quad[2];
                                let ratio = (vertex.position[i] - a.position[i])
                                    / (b.position[i] - a.position[i]);
                                (-1e-10..=1.0 + 1e-10).contains(&ratio)
                                    && ((a.uv[i] + (b.uv[i] - a.uv[i]) * ratio) * 8192. - texel[i])
                                        .abs()
                                        < 1e-8
                            })
                        }));
                    }
                }
            }
        }
    }
}
