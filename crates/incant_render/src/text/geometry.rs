//! CPU half of the text route: validation, counts, quads and node matrices.
//! No GPU, font, shaping, project or source access; nothing here allocates per
//! glyph until the caller has checked the counted totals.
use crate::SceneError;
use glam::{DMat4, DVec3, DVec4};
use incant_render_data::TextInput;
use incant_text::{GlyphAtlas, GlyphHandle, ShapedText};
use incant_types::StableId;
use std::sync::Arc;

/// Logical layout pixels per local unit of the text entity's own transform.
pub const TEXT_LOGICAL_PIXELS_PER_UNIT: f64 = 100.;
/// Unlit linear Rec. 709 radiance of fully covered glyph texels.
pub const TEXT_COLOR: [f32; 3] = [1.; 3];
/// The native world's own text limits; the renderer refuses larger inputs.
pub const MAX_TEXT_NODES: usize = 1024;
pub const MAX_TEXT_GLYPHS: usize = 65_536;
/// WebGPU caps the dynamic uniform offset alignment at 256 bytes.
pub(crate) const NODE_STRIDE: u64 = 256;
pub(crate) const QUAD_BYTES: u64 = size_of::<GlyphQuad>() as u64;

/// Layout logical pixels (Y down) to the entity's local XY plane (Y up).
pub fn text_layout_to_local() -> DMat4 {
    let unit = 1. / TEXT_LOGICAL_PIXELS_PER_UNIT;
    DMat4::from_scale(DVec3::new(unit, -unit, 1.))
}

/// One glyph's padded coverage rectangle in layout pixels and atlas texels.
/// Both rectangles have the same size: one layout pixel is one atlas texel.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct GlyphQuad {
    /// [left, top, right, bottom], Y down, including the one-texel zero padding.
    pub layout: [f32; 4],
    /// [u0, v0, u1, v1] in unnormalized atlas texel coordinates.
    pub texels: [f32; 4],
}
/// `rect` excludes the atlas's one-texel zero padding; `origin` is the Y-down
/// top-left bearing relative to the glyph origin on its baseline.
pub(crate) fn glyph_quad(position: [f32; 2], origin: [i32; 2], rect: [u32; 4]) -> GlyphQuad {
    let [x, y, w, h] = rect.map(f64::from);
    let left = f64::from(position[0]) + f64::from(origin[0]) - 1.;
    let top = f64::from(position[1]) + f64::from(origin[1]) - 1.;
    GlyphQuad {
        layout: [left, top, left + w + 2., top + h + 2.].map(|v| v as f32),
        texels: [x - 1., y - 1., x + w + 1., y + h + 1.].map(|v| v as f32),
    }
}

pub(crate) struct PlannedNode {
    pub id: StableId,
    pub world: DMat4,
    pub layout: Arc<ShapedText>,
    pub glyphs: Arc<[GlyphHandle]>,
    pub quads: u32,
    /// Union of padded quads in layout pixels; all zero for a node with none.
    pub bounds: [f64; 4],
}
/// Checked text input. Holding the atlas Arc keeps it immutable: GlyphAtlas
/// mutation needs unique ownership, and it is not Clone.
pub(crate) struct TextPlan {
    pub atlas: Arc<GlyphAtlas>,
    pub nodes: Vec<PlannedNode>,
    pub quads: usize,
    pub missing: usize,
    /// Smallest atlas prefix [width, height] holding every referenced padded rect.
    pub extent: [u32; 2],
}
impl TextPlan {
    pub fn quad_bytes(&self) -> u64 {
        self.quads as u64 * QUAD_BYTES
    }
    pub fn transform_bytes(&self) -> u64 {
        self.nodes.len() as u64 * NODE_STRIDE
    }
}

/// Validate and count before any expansion. Every handle must resolve in this
/// exact atlas: a handle from another atlas or from before a reset is refused.
pub(crate) fn plan(input: &TextInput) -> Result<TextPlan, SceneError> {
    if input.nodes.len() > MAX_TEXT_NODES {
        return Err(SceneError::Input("text node limit"));
    }
    let [width, height] = input.atlas.size();
    if !(1..=2048).contains(&width)
        || !(1..=2048).contains(&height)
        || input.atlas.pixels().len() != width as usize * height as usize
    {
        return Err(SceneError::Input("text atlas dimensions"));
    }
    let mut plan = TextPlan {
        atlas: Arc::clone(&input.atlas),
        nodes: Vec::with_capacity(input.nodes.len()),
        quads: 0,
        missing: 0,
        extent: [0; 2],
    };
    let mut placements = 0usize;
    for (index, node) in input.nodes.iter().enumerate() {
        if index > 0 && input.nodes[index - 1].id >= node.id {
            return Err(SceneError::Input(
                "text nodes must be sorted by unique stable ID",
            ));
        }
        if !node.world.is_finite() || node.world.row(3) != DVec4::W {
            return Err(SceneError::TransformRange);
        }
        let glyphs = node.layout.glyphs();
        if glyphs.len() != node.glyphs.len() {
            return Err(SceneError::Input("text layout and glyph handles disagree"));
        }
        placements = placements
            .checked_add(glyphs.len())
            .filter(|&n| n <= MAX_TEXT_GLYPHS)
            .ok_or(SceneError::Input("text glyph limit"))?;
        let mut bounds = [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ];
        let mut quads = 0u32;
        for (glyph, handle) in glyphs.iter().zip(node.glyphs.iter()) {
            let entry = input
                .atlas
                .get(handle)
                .ok_or(SceneError::TextGlyph(node.id))?;
            plan.missing += usize::from(entry.missing);
            let Some(rect) = entry.rect else {
                continue;
            };
            if !glyph.position.iter().all(|v| v.is_finite()) {
                return Err(SceneError::Input("text glyph position"));
            }
            // The atlas surrounds each inner rectangle with one zero texel.
            if rect[0] == 0
                || rect[1] == 0
                || rect[0] + rect[2] >= width
                || rect[1] + rect[3] >= height
            {
                return Err(SceneError::Input("text atlas entry bounds"));
            }
            let quad = glyph_quad(glyph.position, entry.origin, rect);
            for i in 0..2 {
                bounds[i] = bounds[i].min(f64::from(quad.layout[i]));
                bounds[i + 2] = bounds[i + 2].max(f64::from(quad.layout[i + 2]));
            }
            plan.extent[0] = plan.extent[0].max(rect[0] + rect[2] + 1);
            plan.extent[1] = plan.extent[1].max(rect[1] + rect[3] + 1);
            quads += 1;
        }
        plan.quads += quads as usize;
        plan.nodes.push(PlannedNode {
            id: node.id,
            world: node.world,
            layout: Arc::clone(&node.layout),
            glyphs: Arc::clone(&node.glyphs),
            quads,
            bounds: if quads == 0 { [0.; 4] } else { bounds },
        });
    }
    Ok(plan)
}

/// Append one checked node's quads in shaped glyph order (bidi runs and
/// fallback fonts as laid out). Spaces and other empty outlines add none.
pub(crate) fn expand(
    atlas: &GlyphAtlas,
    node: &PlannedNode,
    out: &mut Vec<GlyphQuad>,
) -> Result<(), SceneError> {
    for (glyph, handle) in node.layout.glyphs().iter().zip(node.glyphs.iter()) {
        let entry = atlas.get(handle).ok_or(SceneError::TextGlyph(node.id))?;
        if let Some(rect) = entry.rect {
            out.push(glyph_quad(glyph.position, entry.origin, rect));
        }
    }
    Ok(())
}

/// Entity world × layout-to-local in f64, converted once to GPU f32. The node's
/// transformed quad bounds must stay finite in f32; zero scale is allowed.
pub(crate) fn node_matrix(world: DMat4, bounds: [f64; 4]) -> Result<[[f32; 4]; 4], SceneError> {
    if !world.is_finite() || world.row(3) != DVec4::W {
        return Err(SceneError::TransformRange);
    }
    let matrix = world * text_layout_to_local();
    let gpu = matrix.as_mat4();
    if !gpu.is_finite() {
        return Err(SceneError::TransformRange);
    }
    for x in [bounds[0], bounds[2]] {
        for y in [bounds[1], bounds[3]] {
            if !matrix
                .transform_point3(DVec3::new(x, y, 0.))
                .as_vec3()
                .is_finite()
            {
                return Err(SceneError::TransformRange);
            }
        }
    }
    Ok(gpu.to_cols_array_2d())
}
