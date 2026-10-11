use incant_2d::atlas::{AtlasSource, SpriteAtlas, pack};
use std::collections::BTreeMap;

fn input(
    dimensions: &[[u32; 2]],
    maximum_size: [u32; 2],
) -> (SpriteAtlas, BTreeMap<String, [u32; 2]>) {
    let sizes: BTreeMap<_, _> = dimensions
        .iter()
        .enumerate()
        .map(|(i, size)| (ulid::Ulid::from(i as u128 + 1).to_string(), *size))
        .collect();
    let atlas = SpriteAtlas {
        version: 1,
        id: ulid::Ulid::from(999_u128).to_string(),
        maximum_size,
        padding: 2,
        frames: sizes
            .keys()
            .map(|id| {
                (
                    id.clone(),
                    AtlasSource {
                        name: id.clone(),
                        path: "source.png".into(),
                        region: None,
                    },
                )
            })
            .collect(),
    };
    (atlas, sizes)
}

#[test]
fn wide_row_and_column_fit_the_explicit_unrotated_layout() {
    // A known feasible layout, including two pixels of extrusion on every side:
    // wide at (2,2), column at (2,46), row at (10,46), inside 1204 x 1148.
    let (atlas, sizes) = input(&[[1200, 40], [1100, 4], [4, 1100]], [1204, 1148]);
    let layout = pack(&atlas, &sizes).unwrap();
    assert!(layout.size[0] <= 1204 && layout.size[1] <= 1148);
    layout.validate().unwrap();
    for (id, frame) in &layout.frames {
        assert_eq!(&frame.region[2..], &sizes[id]);
    }
}

#[test]
fn a_more_generous_bound_does_not_increase_the_occupied_area() {
    let (mut atlas, sizes) = input(
        &[
            [22, 23],
            [50, 55],
            [61, 26],
            [70, 67],
            [25, 3],
            [5, 77],
            [47, 63],
            [12, 61],
            [47, 50],
            [40, 38],
            [32, 27],
        ],
        [256, 128],
    );
    let first = pack(&atlas, &sizes).unwrap();
    let mut previous = u64::from(first.size[0]) * u64::from(first.size[1]);
    for bound in [
        [256, 256],
        [512, 256],
        [512, 512],
        [1024, 1024],
        [2048, 2048],
    ] {
        atlas.maximum_size = bound;
        let layout = pack(&atlas, &sizes).unwrap();
        let area = u64::from(layout.size[0]) * u64::from(layout.size[1]);
        assert!(
            area <= previous,
            "larger {bound:?} produced {area} after {previous}"
        );
        previous = area;
    }
}

#[test]
fn maximum_frame_count_and_exact_page_limits_remain_bounded() {
    let dimensions: Vec<_> = (0..256).map(|i| [1 + i % 17, 1 + (i * 7) % 19]).collect();
    let (atlas, sizes) = input(&dimensions, [512, 512]);
    let layout = pack(&atlas, &sizes).unwrap();
    assert_eq!(layout.frames.len(), 256);
    layout.validate().unwrap();
    let (atlas, sizes) = input(&[[8188, 508]], [8192, 512]);
    assert_eq!(pack(&atlas, &sizes).unwrap().size, [8192, 512]);
}

#[test]
fn malformed_bounds_have_specific_diagnostics() {
    use incant_2d::atlas::AtlasError;
    let (mut atlas, _) = input(&[[8, 8]], [64, 64]);
    atlas.padding = 0;
    assert_eq!(atlas.validate(), Err(AtlasError::Padding));
    atlas.padding = 2;
    atlas.maximum_size = [8193, 1];
    assert_eq!(atlas.validate(), Err(AtlasError::Dimensions));
    atlas.maximum_size = [4096, 2048];
    assert_eq!(atlas.validate(), Err(AtlasError::Texels));
}
