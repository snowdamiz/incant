//! CPU checks of the text route's planning and numeric placement contract.
//! GPU draw checks live in text_draw_tests.rs. Neither approves appearance.
use super::geometry::{expand, glyph_quad, node_matrix};
use super::*;
use glam::{DMat4, DQuat, DVec3};
use incant_render_data::TextNode;
use incant_text::{AtlasConfig, FontSource, TextEngine, TextStyle, Wrap};

const LATIN: &[u8] = include_bytes!("../../incant_text/tests/fonts/NotoSans-Regular.ttf");
const ARABIC: &[u8] = include_bytes!("../../incant_text/tests/fonts/NotoSansArabic.ttf");
const CJK: &[u8] = include_bytes!("../../incant_text/tests/fonts/IncantCJKFixture.ttf");
/// Latin ligature, Arabic RTL run, CJK fallback, spaces and a Hebrew letter no
/// packaged font covers (an observable `.notdef`).
pub(crate) const MIXED: &str = "ffi سلام 中文 ש";

pub(crate) fn id(n: u8) -> StableId {
    StableId([n; 16])
}
pub(crate) fn engine() -> TextEngine {
    TextEngine::new(
        [LATIN, ARABIC, CJK]
            .into_iter()
            .zip(1u128..)
            .map(|(bytes, id)| FontSource {
                id: id.into(),
                bytes: bytes.to_vec(),
            })
            .collect(),
    )
    .unwrap()
}
pub(crate) fn style(font_size: f32) -> TextStyle {
    TextStyle {
        font_size,
        line_height: font_size * 1.4,
        width: 600.,
        wrap: Wrap::WordOrGlyph,
    }
}
/// Real shaping and rasterization with packaged test fonts; one atlas.
pub(crate) fn input(nodes: &[(u8, &str, f32, DMat4)]) -> TextInput {
    let mut engine = engine();
    let mut atlas = GlyphAtlas::new(AtlasConfig {
        width: 512,
        height: 256,
        max_glyphs: 512,
    })
    .unwrap();
    let nodes = nodes
        .iter()
        .map(|&(n, text, size, world)| {
            let layout = engine.shape(text, style(size)).unwrap();
            let glyphs: Vec<_> = (0..layout.glyphs().len())
                .map(|i| engine.cache_glyph(&layout, i, &mut atlas).unwrap())
                .collect();
            TextNode {
                id: id(n),
                world,
                layout,
                glyphs: glyphs.into(),
            }
        })
        .collect();
    TextInput {
        nodes,
        atlas: Arc::new(atlas),
    }
}
/// Handles cached before `GlyphAtlas::reset`, published with the reset atlas.
pub(crate) fn reset_input() -> TextInput {
    let mut engine = engine();
    let mut atlas = GlyphAtlas::new(AtlasConfig {
        width: 128,
        height: 64,
        max_glyphs: 16,
    })
    .unwrap();
    let layout = engine.shape("Hello", style(20.)).unwrap();
    let glyphs: Vec<_> = (0..layout.glyphs().len())
        .map(|i| engine.cache_glyph(&layout, i, &mut atlas).unwrap())
        .collect();
    atlas.reset().unwrap();
    TextInput {
        nodes: vec![TextNode {
            id: id(2),
            world: DMat4::IDENTITY,
            layout,
            glyphs: glyphs.into(),
        }],
        atlas: Arc::new(atlas),
    }
}

#[test]
fn plan_keeps_shaped_order_bidi_spaces_bearings_and_missing_glyphs() {
    let text = input(&[(1, MIXED, 20., DMat4::IDENTITY)]);
    let layout = &text.nodes[0].layout;
    let glyphs = layout.glyphs();
    // The packaged chain shapes the ligature, falls back for Arabic/CJK and
    // keeps the uncovered Hebrew cluster as glyph zero.
    assert!(glyphs.len() < MIXED.chars().count());
    assert_eq!(layout.missing_clusters().len(), 1);
    assert_eq!(&layout.source()[layout.missing_clusters()[0].clone()], "ש");
    assert_eq!(
        glyphs
            .iter()
            .map(|g| g.font)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        3
    );
    let plan = plan(&text).unwrap();
    let entries: Vec<_> = text.nodes[0]
        .glyphs
        .iter()
        .map(|h| *text.atlas.get(h).unwrap())
        .collect();
    let spaces = glyphs
        .iter()
        .zip(&entries)
        .filter(|(g, e)| &layout.source()[g.source.clone()] == " " && e.rect.is_none())
        .count();
    assert_eq!(spaces, 3, "spaces advance the layout but draw nothing");
    assert_eq!(
        plan.quads,
        entries.iter().filter(|e| e.rect.is_some()).count()
    );
    assert_eq!(plan.quads, glyphs.len() - spaces);
    assert_eq!(plan.missing, 1);
    let notdef = entries.iter().find(|e| e.missing).unwrap();
    assert!(
        notdef.rect.is_some(),
        "the .notdef outline is drawn, not hidden"
    );
    let mut quads = Vec::new();
    expand(&plan.atlas, &plan.nodes[0], &mut quads).unwrap();
    assert_eq!(quads.len(), plan.quads);
    // Quads follow the layout's visual glyph order exactly: no second bidi
    // reversal. Within the Arabic run, x grows while source clusters shrink.
    let mut drawn = glyphs
        .iter()
        .zip(&entries)
        .filter(|(_, e)| e.rect.is_some());
    for quad in &quads {
        let (glyph, entry) = drawn.next().unwrap();
        let [x, y, w, h] = entry.rect.unwrap().map(|v| v as f32);
        let left = glyph.position[0] + entry.origin[0] as f32;
        let top = glyph.position[1] + entry.origin[1] as f32;
        assert!((quad.layout[0] - (left - 1.)).abs() < 1e-4);
        assert!((quad.layout[1] - (top - 1.)).abs() < 1e-4);
        assert!((quad.layout[2] - quad.layout[0] - (w + 2.)).abs() < 1e-3);
        assert!((quad.layout[3] - quad.layout[1] - (h + 2.)).abs() < 1e-3);
        assert_eq!(quad.texels, [x - 1., y - 1., x + w + 1., y + h + 1.]);
        assert!(quad.texels[2] as u32 <= plan.extent[0]);
        assert!(quad.texels[3] as u32 <= plan.extent[1]);
    }
    let word = MIXED.find("سلام").unwrap()..MIXED.find(" 中").unwrap();
    let arabic: Vec<_> = glyphs
        .iter()
        .filter(|g| word.contains(&g.source.start))
        .collect();
    assert!(arabic.len() >= 3);
    assert!(arabic.iter().all(|g| g.bidi_level % 2 == 1));
    for pair in arabic.windows(2) {
        assert!(pair[0].position[0] < pair[1].position[0]);
        assert!(pair[0].source.start > pair[1].source.start);
    }
    // A Latin letter without descender sits on its line's baseline.
    let f = glyphs.iter().zip(&entries).next().unwrap();
    let bottom = f.0.position[1] + (f.1.origin[1] + f.1.rect.unwrap()[3] as i32) as f32;
    assert!((bottom - layout.lines()[0].baseline).abs() <= 1.);
    assert!(plan.extent[0] <= 512 && plan.extent[1] <= 256);
    assert_eq!(plan.quad_bytes(), plan.quads as u64 * 32);
    assert_eq!(plan.transform_bytes(), 256);
}

#[test]
fn foreign_reset_mismatched_and_oversized_text_is_refused_before_expansion() {
    let a = input(&[(1, "Hello", 20., DMat4::IDENTITY)]);
    let b = input(&[(1, "Hello", 20., DMat4::IDENTITY)]);
    // Equal content, sizes and slots, but another atlas instance owns them.
    let mut foreign = a.clone();
    foreign.nodes[0].glyphs = Arc::clone(&b.nodes[0].glyphs);
    assert!(matches!(plan(&foreign), Err(SceneError::TextGlyph(n)) if n == id(1)));
    // Reset invalidates every handle of the same atlas allocation.
    assert!(matches!(plan(&reset_input()), Err(SceneError::TextGlyph(n)) if n == id(2)));
    let mut short = a.clone();
    short.nodes[0].glyphs = short.nodes[0].glyphs[1..].into();
    assert!(matches!(plan(&short), Err(SceneError::Input(_))));
    let mut duplicate = a.clone();
    duplicate.nodes.push(duplicate.nodes[0].clone());
    assert!(matches!(plan(&duplicate), Err(SceneError::Input(_))));
    let mut skewed = a.clone();
    skewed.nodes[0].world.x_axis.w = 0.5;
    assert!(matches!(plan(&skewed), Err(SceneError::TransformRange)));
    let mut many = a.clone();
    many.nodes = (0..=MAX_TEXT_NODES)
        .map(|n| TextNode {
            id: StableId((n as u128).to_be_bytes()),
            ..a.nodes[0].clone()
        })
        .collect();
    assert!(matches!(
        plan(&many),
        Err(SceneError::Input("text node limit"))
    ));
    // Five 16 KiB layouts exceed the shared placement ceiling while counting.
    let long = input(&[(1, "i".repeat(16_384).as_str(), 8., DMat4::IDENTITY)]);
    let mut wide = long.clone();
    wide.nodes = (1..=5u8)
        .map(|n| TextNode {
            id: id(n),
            ..long.nodes[0].clone()
        })
        .collect();
    assert!(matches!(
        plan(&wide),
        Err(SceneError::Input("text glyph limit"))
    ));
    assert!(plan(&long).is_ok());
}

#[test]
fn layout_pixels_map_to_entity_local_units_baselines_and_transforms() {
    let to_local = text_layout_to_local();
    assert_eq!(
        to_local.transform_point3(DVec3::new(100., 0., 0.)),
        DVec3::X
    );
    assert_eq!(
        to_local.transform_point3(DVec3::new(0., 100., 0.)),
        -DVec3::Y
    );
    assert_eq!(to_local.transform_point3(DVec3::ZERO), DVec3::ZERO);
    let text = input(&[(1, "Ag", 32., DMat4::IDENTITY)]);
    let baseline = f64::from(text.nodes[0].layout.lines()[0].baseline);
    let local = to_local.transform_point3(DVec3::new(0., baseline, 0.));
    assert!((local.y + baseline / TEXT_LOGICAL_PIXELS_PER_UNIT).abs() < 1e-12);
    // Translation, rotation and scale of the entity apply after the unit map.
    let world = DMat4::from_scale_rotation_translation(
        DVec3::splat(2.),
        DQuat::from_rotation_z(std::f64::consts::FRAC_PI_2),
        DVec3::new(2., 3., 4.),
    );
    let gpu = glam::Mat4::from_cols_array_2d(&node_matrix(world, [0., 0., 100., 100.]).unwrap());
    let point = gpu.transform_point3(glam::Vec3::new(100., 0., 0.));
    assert!((point - glam::Vec3::new(2., 5., 4.)).length() < 1e-5);
    let point = gpu.transform_point3(glam::Vec3::new(0., 100., 0.));
    assert!((point - glam::Vec3::new(4., 3., 4.)).length() < 1e-5);
    assert!(node_matrix(DMat4::from_scale(DVec3::ZERO), [0., 0., 10., 10.]).is_ok());
    assert!(matches!(
        node_matrix(DMat4::from_scale(DVec3::splat(1e40)), [0., 0., 10., 10.]),
        Err(SceneError::TransformRange)
    ));
    assert!(matches!(
        node_matrix(DMat4::from_scale(DVec3::splat(1e37)), [0., 0., 1e4, 1e4]),
        Err(SceneError::TransformRange)
    ));
    let quad = glyph_quad([10.25, 20.], [-2, -15], [7, 9, 4, 5]);
    assert_eq!(quad.layout, [7.25, 4., 13.25, 11.]);
    assert_eq!(quad.texels, [6., 8., 12., 15.]);
}
