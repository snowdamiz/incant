//! Compare actual GPU texel runs at the large-coordinate end of source regions.
//! No shader-formula oracle, screenshot artifact or appearance approval.
use super::*;

#[test]
#[ignore = "requires a native GPU; desktop workflows execute this"]
fn wide_magnified_nearest_regions_preserve_their_leading_and_trailing_texels() {
    let renderer = crate::support::checked(Renderer::headless().unwrap());
    eprintln!(
        "wide sprite sweep adapter: {:?}",
        renderer.adapter.get_info()
    );
    let background = [20, 21, 25, 255];
    let mut failures = Vec::new();
    let mut cases = 0;
    let mut interior_cases = 0;
    for width in [512_u32, 1024, 2048, 4096, 8192] {
        let pixels: Vec<_> = (0..width)
            .flat_map(|i| {
                [
                    24 + (i % 32) as u8 * 6,
                    24 + (i / 32 % 16) as u8 * 12,
                    24 + (i / 512) as u8 * 12,
                    255,
                ]
            })
            .collect();
        for vertical in [false, true] {
            let (w, h) = if vertical { (1, width) } else { (width, 1) };
            let mut fixture = Fixture::new(&pixels, w, h);
            let line = |frame: Vec<[u8; 4]>| -> Vec<_> {
                (0..64)
                    .map(|i| frame[if vertical { i * 64 + 32 } else { 32 * 64 + i }])
                    .collect()
            };
            // Integer-aligned 1:1 crops independently measure the post-display
            // colour of every source texel, including its span identity. A
            // repeating palette could silently accept an omitted flat base.
            fixture.edit("size", json!([8., 8.]));
            let mut palette = Vec::with_capacity(width as usize);
            for start in (0..width).step_by(64) {
                fixture.edit(
                    "region",
                    if vertical {
                        json!([0, start, 1, 64])
                    } else {
                        json!([start, 0, 64, 1])
                    },
                );
                palette.extend(line(fixture.pixels(&renderer)));
            }
            assert!(!palette.contains(&background));
            assert_eq!(palette.len(), width as usize);
            assert_eq!(
                palette
                    .iter()
                    .collect::<std::collections::BTreeSet<_>>()
                    .len(),
                width as usize,
                "every source texel must remain distinguishable after display"
            );
            fixture.edit("region", json!([0, 0, w, h]));
            for scale in [1_usize, 2, 4, 8, 16, 20, 28, 32, 36, 44, 48, 64, 96, 128] {
                for offset in [0., 0.625, 1., 2., 3.] {
                    for flipped in [false, true] {
                        fixture.edit(if vertical { "flip_y" } else { "flip_x" }, json!(flipped));
                        let side = f64::from(width) * scale as f64;
                        fixture.edit(
                            "size",
                            if vertical {
                                json!([8., side / 8.])
                            } else {
                                json!([side / 8., 8.])
                            },
                        );
                        // View the final source texels: unflipped right/bottom
                        // edge, or flipped left/top edge. Most of the very wide
                        // geometry is intentionally outside the 64-pixel view.
                        let edge = if flipped { 4.5 } else { 60.5 } + offset / 256.;
                        let center = edge + if flipped { side / 2. } else { -side / 2. };
                        let translation = if vertical {
                            [0., (32. - center) / 8., 0.]
                        } else {
                            [(center - 32.) / 8., 0., 0.]
                        };
                        fixture
                            .project
                            .scenes
                            .get_mut(&fixture.scene)
                            .unwrap()
                            .entities
                            .get_mut(&fixture.sprite)
                            .unwrap()
                            .components
                            .insert(
                                "Transform".into(),
                                json!(Transform {
                                    translation,
                                    ..Default::default()
                                }),
                            );
                        let actual: Vec<_> = line(fixture.pixels(&renderer))
                            .into_iter()
                            .filter(|p| *p != background)
                            .collect();
                        assert!(actual.len() >= 59, "strip must fill the measured edge");
                        let incorrect = actual
                            .iter()
                            .enumerate()
                            .filter(|(i, pixel)| {
                                let distance = if flipped { *i } else { actual.len() - 1 - i };
                                let texel = width as usize - 1 - distance / scale;
                                **pixel != palette[texel]
                            })
                            .count();
                        if incorrect != 0 {
                            failures.push(format!("width {width}, scale {scale}, offset {offset}/256 px, vertical {vertical}, flipped {flipped}: {incorrect} wrong pixels"));
                        }
                        cases += 1;
                    }
                }
                // Observe internal span boundaries as well as outer edges. A
                // flat source base must never introduce a seam or change the
                // texel selected on either side of a shared edge.
                let anchors = std::collections::BTreeSet::from([
                    288,
                    448,
                    480,
                    width / 2,
                    512.min(width / 2),
                    width.saturating_sub(512).max(width / 2),
                ]);
                for offset in [-1, 0, 1] {
                    for &anchor in &anchors {
                        for flipped in [false, true] {
                            fixture
                                .edit(if vertical { "flip_y" } else { "flip_x" }, json!(flipped));
                            let side = f64::from(width) * scale as f64;
                            let from_left = if flipped { width - anchor } else { anchor };
                            let center = 4.5 + f64::from(offset) / 256.
                                - f64::from(from_left) * scale as f64
                                + side / 2.;
                            let translation = if vertical {
                                [0., (32. - center) / 8., 0.]
                            } else {
                                [(center - 32.) / 8., 0., 0.]
                            };
                            fixture
                                .project
                                .scenes
                                .get_mut(&fixture.scene)
                                .unwrap()
                                .entities
                                .get_mut(&fixture.sprite)
                                .unwrap()
                                .components
                                .insert(
                                    "Transform".into(),
                                    json!(Transform {
                                        translation,
                                        ..Default::default()
                                    }),
                                );
                            let actual = line(fixture.pixels(&renderer));
                            let incorrect = actual
                                .iter()
                                .enumerate()
                                .filter(|(i, pixel)| {
                                    let displacement = (256 * (*i as i32 - 4) - offset)
                                        .div_euclid(256 * scale as i32);
                                    let texel = if flipped {
                                        anchor as i32 - 1 - displacement
                                    } else {
                                        anchor as i32 + displacement
                                    };
                                    **pixel
                                        != palette
                                            .get(texel as usize)
                                            .copied()
                                            .unwrap_or(background)
                                })
                                .count();
                            if incorrect != 0 {
                                failures.push(format!("interior width {width}, scale {scale}, anchor {anchor}, offset {offset}/256 px, vertical {vertical}, flipped {flipped}: {incorrect} wrong pixels"));
                            }
                            interior_cases += 1;
                        }
                    }
                }
            }
        }
    }
    assert_eq!(cases, 1400);
    assert_eq!(interior_cases, 4368);
    cases += interior_cases;
    assert!(
        failures.is_empty(),
        "{} of {cases} cases failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
