use super::*;
use std::{cmp::Reverse, collections::BTreeSet};

/// Deterministic, unrotated skyline packing with bounded alternative orders and
/// axis sweeps. Candidate widths come from the frames and a fixed 32-texel grid,
/// not the requested bound. Larger bounds therefore retain every formerly
/// eligible layout and cannot increase the chosen page area. This is a heuristic,
/// not a proof that every feasible packing will be found.
pub fn pack(
    descriptor: &SpriteAtlas,
    sizes: &BTreeMap<String, [u32; 2]>,
) -> Result<AtlasLayout, AtlasError> {
    descriptor.validate()?;
    if descriptor.frames.keys().ne(sizes.keys()) {
        return Err(AtlasError::Frames);
    }
    let padding = descriptor.padding;
    let mut original = Vec::new();
    for (id, &[w, h]) in sizes {
        if w == 0 || h == 0 || w > 8192 || h > 8192 {
            return Err(AtlasError::Region);
        }
        let size = [w + padding * 2, h + padding * 2];
        if size
            .iter()
            .zip(descriptor.maximum_size)
            .any(|(a, b)| *a > b)
        {
            return Err(AtlasError::NoFit);
        }
        original.push((id.clone(), size[0], size[1]));
    }
    let mut best: Option<Packed> = None;
    // Transposing the entire packing problem does not rotate any source frame.
    // It merely considers a vertical sweep, then transposes coordinates back.
    for transpose in [false, true] {
        let mut items = original.clone();
        let mut maximum = descriptor.maximum_size;
        if transpose {
            maximum.swap(0, 1);
            for (_, w, h) in &mut items {
                std::mem::swap(w, h);
            }
        }
        for tall_first in [false, true] {
            items.sort_by_key(|(id, w, h)| {
                (
                    Reverse(if tall_first { *h } else { (*w).max(*h) }),
                    Reverse(u64::from(*w) * u64::from(*h)),
                    id.clone(),
                )
            });
            let minimum = items.iter().map(|(_, w, _)| *w).max().unwrap();
            let mut widths = BTreeSet::from([minimum]);
            widths.extend((32..=8192).step_by(32).filter(|w| *w >= minimum));
            let mut sum = 0;
            for (_, w, _) in &items {
                sum += w;
                if sum <= 8192 {
                    widths.insert(sum);
                }
            }
            let area: u64 = items
                .iter()
                .map(|(_, w, h)| u64::from(*w) * u64::from(*h))
                .sum();
            for width in widths {
                if width > maximum[0]
                    || width > sum
                    || area > u64::from(width) * u64::from(maximum[1])
                {
                    continue;
                }
                let Some(mut packed) = attempt(&items, width, padding) else {
                    continue;
                };
                if packed.0[1] > maximum[1] {
                    continue;
                }
                if transpose {
                    packed.0.swap(0, 1);
                    for region in packed.1.values_mut() {
                        region.swap(0, 1);
                        region.swap(2, 3);
                    }
                }
                if best
                    .as_ref()
                    .is_none_or(|old| score(packed.0) < score(old.0))
                {
                    best = Some(packed);
                }
            }
        }
    }
    let (size, frames) = best.ok_or(AtlasError::NoFit)?;
    let layout = AtlasLayout {
        id: descriptor.id.clone(),
        size,
        padding,
        frames: frames
            .into_iter()
            .map(|(id, region)| {
                let name = descriptor.frames[&id].name.clone();
                (id, AtlasFrame { name, region })
            })
            .collect(),
    };
    layout.validate()?;
    Ok(layout)
}
fn score(size: [u32; 2]) -> (u64, u32, u32) {
    (
        u64::from(size[0]) * u64::from(size[1]),
        size[0].max(size[1]),
        size[0],
    )
}
type Packed = ([u32; 2], BTreeMap<String, [u32; 4]>);

fn attempt(items: &[(String, u32, u32)], width: u32, padding: u32) -> Option<Packed> {
    // Each pair starts a constant-height skyline interval. A placed rectangle
    // covers all intervals below it; gaps below the skyline are not reused.
    let mut skyline = vec![(0_u32, 0_u32)];
    let mut regions = BTreeMap::new();
    let mut size = [0, 0];
    for (id, w, h) in items {
        let mut best: Option<(u32, u32, usize)> = None;
        for (index, &(x, y)) in skyline.iter().enumerate() {
            if x + w > width {
                continue;
            }
            let mut top = y;
            for &(next_x, next_y) in &skyline[index + 1..] {
                if next_x >= x + w {
                    break;
                }
                top = top.max(next_y);
            }
            if top + h <= 8192 {
                let candidate = (top + h, x, index);
                if best.is_none_or(|old| candidate < old) {
                    best = Some(candidate);
                }
            }
        }
        let (top, x, index) = best?;
        let y = top - h;
        let end = x + w;
        let after = skyline.partition_point(|(sx, _)| *sx < end);
        let previous_height = skyline[after - 1].1;
        let add_tail = end < width && skyline.get(after).is_none_or(|(sx, _)| *sx != end);
        skyline.splice(index..after, [(x, top)]);
        if add_tail {
            skyline.insert(index + 1, (end, previous_height));
        }
        // Equal-height neighbours have no meaningful boundary and need not be searched.
        skyline.dedup_by(|a, b| a.1 == b.1);
        regions.insert(
            id.clone(),
            [x + padding, y + padding, w - padding * 2, h - padding * 2],
        );
        size[0] = size[0].max(end);
        size[1] = size[1].max(top);
    }
    Some((size, regions))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn descriptor(sizes: &BTreeMap<String, [u32; 2]>) -> SpriteAtlas {
        SpriteAtlas {
            version: 1,
            id: ulid::Ulid::from(999_u128).to_string(),
            maximum_size: [512, 512],
            padding: 2,
            frames: sizes
                .keys()
                .map(|id| {
                    (
                        id.clone(),
                        AtlasSource {
                            name: id.clone(),
                            path: "test.png".into(),
                            region: None,
                        },
                    )
                })
                .collect(),
        }
    }
    #[test]
    fn generated_sets_preserve_every_size_and_padding_without_overlap() {
        let mut rng = 42_u64;
        for count in 1..=96 {
            let mut sizes = BTreeMap::new();
            for i in 0..count {
                rng ^= rng << 13;
                rng ^= rng >> 7;
                rng ^= rng << 17;
                sizes.insert(
                    ulid::Ulid::from(i as u128 + 1).to_string(),
                    [1 + (rng % 31) as u32, 1 + ((rng >> 8) % 31) as u32],
                );
            }
            let descriptor = descriptor(&sizes);
            let layout = pack(&descriptor, &sizes).unwrap();
            layout.validate().unwrap();
            assert_eq!(layout, pack(&descriptor, &sizes).unwrap());
            assert!(layout.size[0] <= 512 && layout.size[1] <= 512);
            assert_eq!(layout.frames.len(), count);
            for (id, frame) in &layout.frames {
                assert_eq!(&frame.region[2..], &sizes[id]);
            }
        }
    }
    #[test]
    fn impossible_and_malformed_layouts_fail_explicitly() {
        let id = ulid::Ulid::from(1_u128).to_string();
        let sizes = BTreeMap::from([(id.clone(), [510, 510])]);
        let descriptor = descriptor(&sizes);
        assert_eq!(pack(&descriptor, &sizes), Err(AtlasError::NoFit));
        let sizes = BTreeMap::from([(id.clone(), [32, 32])]);
        let mut layout = pack(&descriptor, &sizes).unwrap();
        layout.frames.insert(
            ulid::Ulid::from(2_u128).to_string(),
            layout.frames[&id].clone(),
        );
        assert_eq!(layout.validate(), Err(AtlasError::Overlap));
        layout.frames.get_mut(&id).unwrap().region[0] = u32::MAX;
        assert_eq!(layout.validate(), Err(AtlasError::Region));
    }
}
