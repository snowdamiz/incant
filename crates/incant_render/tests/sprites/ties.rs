use super::*;

fn sheet(height: u32) -> Vec<u8> {
    let mut pixels = vec![0_u8; 64 * height as usize * 4];
    for y in 0..7 {
        for x in 0..7 {
            let offset = ((17 + y) * 64 + 13 + x) * 4;
            pixels[offset..offset + 4].copy_from_slice(&[
                30 + x as u8 * 20,
                30 + y as u8 * 20,
                60,
                255,
            ]);
        }
    }
    pixels
}

fn grid(pixels: &[[u8; 4]], side: usize) -> Vec<[u8; 4]> {
    let occupied: Vec<_> = pixels
        .iter()
        .enumerate()
        .filter(|(_, p)| **p != [20, 21, 25, 255])
        .map(|(i, _)| i)
        .collect();
    let left = occupied.iter().map(|i| i % 64).min().unwrap();
    let right = occupied.iter().map(|i| i % 64).max().unwrap();
    let top = occupied.iter().map(|i| i / 64).min().unwrap();
    let bottom = occupied.iter().map(|i| i / 64).max().unwrap();
    assert_eq!(
        (right - left + 1, bottom - top + 1, occupied.len()),
        (side, side, side * side)
    );
    (top..=bottom)
        .flat_map(|y| (left..=right).map(move |x| pixels[y * 64 + x]))
        .collect()
}

#[test]
#[ignore = "requires a native GPU; desktop workflows execute this"]
fn nearest_texel_ties_preserve_each_texel_through_orientation_and_sheet_resize() {
    let renderer = Renderer::headless().unwrap();
    let mut fixture = Fixture::new(&sheet(64), 64, 64);
    fixture.edit("region", json!([13, 17, 7, 7]));
    fixture.edit("size", json!([7. / 8., 7. / 8.]));
    // Integer image edges provide an unambiguous independent palette/order.
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
                translation: [0.5 / 8., 0.5 / 8., 0.],
                ..Default::default()
            }),
        );
    let reference = grid(&fixture.pixels(&renderer), 7);
    let mut mirror = Entity::new("Mirror parent");
    mirror.components.insert(
        "Transform".into(),
        json!(Transform {
            scale: [-1., 1., 1.],
            ..Default::default()
        }),
    );
    for scale in 1..=3 {
        fixture.edit(
            "size",
            json!([7. / 8. * scale as f64, 7. / 8. * scale as f64]),
        );
        let mut frames = Vec::new();
        for height in [64, 80] {
            fixture.cook(&sheet(height), 64, height);
            for orientation in 0..8 {
                fixture.edit("flip_x", json!(orientation == 1 || orientation == 3));
                fixture.edit("flip_y", json!(orientation == 2 || orientation == 3));
                let q = std::f64::consts::FRAC_1_SQRT_2;
                let transform = Transform {
                    rotation: match orientation {
                        4 => [0., 0., 1., 0.],
                        6 => [0., 0., q, q],
                        7 => [0., 0., -q, q],
                        _ => [0., 0., 0., 1.],
                    },
                    ..Default::default()
                };
                let entities = &mut fixture
                    .project
                    .scenes
                    .get_mut(&fixture.scene)
                    .unwrap()
                    .entities;
                entities.remove(&mirror.id);
                if orientation == 5 {
                    entities.insert(mirror.id.clone(), mirror.clone());
                }
                let sprite = entities.get_mut(&fixture.sprite).unwrap();
                sprite.parent = (orientation == 5).then(|| mirror.id.clone());
                sprite
                    .components
                    .insert("Transform".into(), json!(transform));
                let frame = fixture.pixels(&renderer);
                let side = 7 * scale;
                let actual = grid(&frame, side);
                for y in 0..side {
                    for x in 0..side {
                        let (u, v) = (x / scale, y / scale);
                        let (source_x, source_y) = match orientation {
                            1 | 5 => (6 - u, v),
                            2 => (u, 6 - v),
                            3 | 4 => (6 - u, 6 - v),
                            6 => (6 - v, u),
                            7 => (v, 6 - u),
                            _ => (u, v),
                        };
                        assert_eq!(
                            actual[y * side + x],
                            reference[source_y * 7 + source_x],
                            "height {height}, scale {scale}, orientation {orientation}, x {x}, y {y}"
                        );
                    }
                }
                if height == 64 {
                    frames.push(frame);
                } else {
                    assert_eq!(frame, frames[orientation]);
                }
            }
        }
    }
}
