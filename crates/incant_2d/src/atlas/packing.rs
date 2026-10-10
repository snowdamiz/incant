use super::*;
use std::cmp::Reverse;

/// Deterministic unrotated guillotine packing. Several bounded candidate widths
/// are compared; the smallest occupied area wins. This is not an optimal-packing
/// guarantee. Failure is explicit and never drops frames or creates extra pages.
pub fn pack(
    descriptor: &SpriteAtlas,
    sizes: &BTreeMap<String, [u32; 2]>,
) -> Result<AtlasLayout, AtlasError> {
    descriptor.validate()?;
    if descriptor.frames.keys().ne(sizes.keys()) {
        return Err(AtlasError::Frames);
    }
    let p = descriptor.padding;
    let mut items = Vec::new();
    for (id, &[w, h]) in sizes {
        if w == 0 || h == 0 || w > 8192 || h > 8192 {
            return Err(AtlasError::Region);
        }
        items.push((id.clone(), w + p * 2, h + p * 2));
    }
    items.sort_by_key(|(id, w, h)| {
        (
            Reverse((*w).max(*h)),
            Reverse(u64::from(*w) * u64::from(*h)),
            id.clone(),
        )
    });
    let minimum = items.iter().map(|(_, w, _)| *w).max().unwrap();
    if minimum > descriptor.maximum_size[0]
        || items
            .iter()
            .any(|(_, _, h)| *h > descriptor.maximum_size[1])
    {
        return Err(AtlasError::NoFit);
    }
    let mut widths = std::collections::BTreeSet::from([minimum, descriptor.maximum_size[0]]);
    let mut width = minimum.next_power_of_two();
    while width < descriptor.maximum_size[0] {
        widths.insert(width);
        width *= 2;
    }
    let mut best: Option<AtlasLayout> = None;
    for width in widths {
        if let Some((size, rectangles)) = attempt(&items, [width, descriptor.maximum_size[1]], p) {
            let layout = AtlasLayout {
                id: descriptor.id.clone(),
                size,
                padding: p,
                frames: rectangles
                    .into_iter()
                    .map(|(id, region)| {
                        let name = descriptor.frames[&id].name.clone();
                        (id, AtlasFrame { name, region })
                    })
                    .collect(),
            };
            let score = |a: &AtlasLayout| {
                (
                    u64::from(a.size[0]) * u64::from(a.size[1]),
                    a.size[0].max(a.size[1]),
                    a.size[0],
                )
            };
            if best.as_ref().is_none_or(|old| score(&layout) < score(old)) {
                best = Some(layout);
            }
        }
    }
    let layout = best.ok_or(AtlasError::NoFit)?;
    layout.validate()?;
    Ok(layout)
}
type Packed = ([u32; 2], BTreeMap<String, [u32; 4]>);
fn attempt(items: &[(String, u32, u32)], maximum: [u32; 2], padding: u32) -> Option<Packed> {
    let mut free = vec![[0, 0, maximum[0], maximum[1]]];
    let mut regions = BTreeMap::new();
    let mut size = [0, 0];
    for (id, w, h) in items {
        let index = free
            .iter()
            .enumerate()
            .filter(|(_, r)| r[2] >= *w && r[3] >= *h)
            .min_by_key(|(i, r)| {
                (
                    (r[2] - w).min(r[3] - h),
                    (r[2] - w).max(r[3] - h),
                    r[1],
                    r[0],
                    *i,
                )
            })
            .map(|(i, _)| i)?;
        let [x, y, rw, rh] = free.remove(index);
        let dw = rw - w;
        let dh = rh - h;
        let split = if dw > dh {
            [[x + w, y, dw, rh], [x, y + h, *w, dh]]
        } else {
            [[x + w, y, dw, *h], [x, y + h, rw, dh]]
        };
        free.extend(split.into_iter().filter(|r| r[2] > 0 && r[3] > 0));
        regions.insert(
            id.clone(),
            [x + padding, y + padding, w - padding * 2, h - padding * 2],
        );
        size[0] = size[0].max(x + w);
        size[1] = size[1].max(y + h);
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
