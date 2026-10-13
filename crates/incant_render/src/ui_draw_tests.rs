//! Actual GPU UI draws compared with an independent CPU composition written
//! from the documented contract: physical mapping, snapping, clips, rounded
//! coverage, bilinear glyph taps, label halos (prepared entries drawn as
//! glyphs, clipped past the text clip) and premultiplied over. Ignored without a granted native GPU; numeric
//! agreement, not appearance approval.
//!
//! The reference gives every channel the stored codes the contract admits.
//! Geometry, clips, layering, colours and rounded coverage are exact; only the
//! precision the contract leaves to the backend widens a channel:
//! * `TAP`, the coverage of a glyph tap between texel centres. The R8 atlas is
//!   filtered in fixed point with at least 8 sub-texel bits (Mesa llvmpipe:
//!   weights rounded to 1/256, then two 8-bit rounded lerp stages): at most
//!   1/255 from the lerps, plus 1/255 when a position is not a multiple of
//!   1/256 texel (each weight then moves by at most 1/512). A tap exactly on a
//!   texel centre returns that texel. Backends below 8 sub-texel bits (Vulkan
//!   allows 4) are outside this reference.
//! * `DECODE`/`ENCODE`, the conversions of an 8-bit sRGB target that is read
//!   back and stored between blends: the decoded destination within 0.5 code
//!   and the stored code within 0.67 code of exact, rounding included
//!   (llvmpipe documents ±0.35 for decode and ±0.17 before rounding for
//!   encode). UNORM targets read back exactly.
//! * `LINEAR_ENCODE`, a half step of a 12-bit UNORM linear value immediately
//!   before sRGB encoding. The retained 0069 Apple M5 Pro readbacks match
//!   this conversion model (including all 33,000 black-backdrop channels);
//!   this is observed precision, not a universal Metal specification. Near
//!   zero its contribution reaches 0.4023 stored codes before rounding, so
//!   the existing code-space ENCODE bound alone does not cover it. The
//!   destination decode error remains covered by DECODE. Neither this term
//!   nor the measured candidate changes UNORM targets or glyph coverage.
//! * `CONVERT`/`PRODUCT`, a UNORM target blended at its own 8-bit precision,
//!   which Vulkan ("no lower than that used to represent destination
//!   components") and D3D11.3 §17.5 ("equal or more ... precision ... than the
//!   output format") permit, and llvmpipe and WARP do. The premultiplied
//!   source and the alpha each become a code within `CONVERT` (D3D11.3
//!   §3.2.3.6 allows 0.6; Vulkan recommends and llvmpipe uses the nearest),
//!   the destination product d·(255 − a)/255 one within `PRODUCT` (llvmpipe's
//!   `(t + (t >> 8) + 128) >> 8` misrounds 24 of the 65,536 8-bit products by
//!   128/255), then the sum saturates. A channel admits this blend or the
//!   float one: at most 1.71 codes from exact per blend, exact for opaque
//!   fragments. Coarser blend arithmetic (truncation, `a·(b + 1) >> 8`) is
//!   outside this reference.
//!
//! Near black the sRGB encoding magnifies a coverage step of 1/255 to several
//! codes, so a fixed code tolerance cannot express the contract there.
use super::tests::Fixture;
use super::*;
use crate::{
    Viewport,
    hdr_output::{read, save, texture},
};
use incant_render_data::{UiEdit, UiRect, UiRole, UiWidget};
use incant_text::{AtlasEntry, Wrap, editing::Selection};
use std::sync::Weak;

const SRGB: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const UNORM: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const TAP: f64 = 1. / 255.;
const DECODE: f64 = 0.5;
const ENCODE: f64 = 0.67;
const LINEAR_ENCODE: f64 = 0.5 / 4095.;
const CONVERT: f64 = 0.6;
const PRODUCT: f64 = 128. / 255.;

// Existing checkpoint runs use --nocapture; retain the actual selected adapter
// rather than inferring a backend from installed driver packages.
fn renderer_with_adapter_receipt() -> Renderer {
    let renderer = crate::test_support::checked(Renderer::headless().unwrap());
    eprintln!(
        "UI draw selected adapter: {:?}",
        renderer.adapter.get_info()
    );
    renderer
}

/// How a reference composes.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Model {
    /// The admitted codes per channel under the backend precision above.
    Portable,
    /// One candidate image: exact filtering and conversion, nearest storage.
    Exact,
    /// Measured Apple M5 Pro candidate: exact glyph filtering, nearest
    /// 12-bit linear destination/encode conversion, nearest stored sRGB.
    /// Fits retained 0069 readbacks; not a universal backend guarantee.
    ObservedM5,
    /// One candidate image emulated from the source of Mesa 25.2.8 llvmpipe,
    /// the Linux CI adapter: `lp_bld_sample_aos.c` (8-bit fixed-point
    /// bilinear), `lp_bld_arit.c` (SSSE3/AVX2 rounded lerp, `mul_norm`),
    /// `lp_bld_format_srgb.c` (polynomial sRGB around a float blend) and, for
    /// UNORM8 targets, the 8-bit blend of `lp_state_fs.c` and `lp_bld_conv.c`.
    Llvmpipe,
}
mod llvmpipe {
    use incant_text::GlyphAtlas;
    /// One bilinear tap of an R8 texture at texel position `(u, v)`.
    pub fn tap(atlas: &GlyphAtlas, u: f64, v: f64) -> f64 {
        let [width, _] = atlas.size();
        let texel = |x: i64, y: i64| i64::from(atlas.pixels()[(y * i64::from(width) + x) as usize]);
        // Coordinates to 8 fractional bits, rounded, less half a texel.
        let fixed = |p: f64| (p * 256.).round_ties_even() as i64 - 128;
        let (s, t) = (fixed(u), fixed(v));
        let (i, j, ws, wt) = (s >> 8, t >> 8, s & 255, t & 255);
        // pmulhrsw(w, delta << 7): v0 + floor((w * delta + 128) / 256).
        let lerp = |a: i64, b: i64, w: i64| a + (w * (b - a) + 128).div_euclid(256);
        let top = lerp(texel(i, j), texel(i + 1, j), ws);
        let bottom = lerp(texel(i, j + 1), texel(i + 1, j + 1), ws);
        lerp(top, bottom, wt) as f64 / 255.
    }
    pub fn decode(code: f64) -> f32 {
        let s = code as f32;
        if s <= 15. {
            s * (1. / (12.6 * 255.))
        } else {
            let x = s / 255.;
            0.0023 + 0.0030 * x + 0.6935 * x * x + 0.3012 * x * x * x
        }
    }
    /// The stored code. llvmpipe's reciprocal square roots are taken exactly.
    pub fn encode(linear: f32) -> f64 {
        let x = linear.clamp(0., 1.);
        let v = if x <= 0.003_130_8 {
            x * (12.92 * 255.)
        } else {
            (0.675 * 1.0622 * 255.) * x.powf(0.375) + (0.325 * 1.0622 * 255.) * x.sqrt()
                - 0.062 * 255.
        };
        f64::from(v.round_ties_even())
    }
    /// The code an 8-bit UNORM target stores for the shader's premultiplied
    /// f32 output: UNORM8 targets blend in their own 8-bit type, the colour
    /// and alpha converted by `cvtps2dq`, then `uadd.sat(s, mul_norm(d, !a))`.
    pub fn unorm_over(color: u8, opacity: u8, coverage: f64, destination: f64) -> f64 {
        // Fetch scales UNORM8 by 1/255 (`lp_build_unsigned_norm_to_float`).
        let unit = |code: u8| f32::from(code) * (1. / 255.);
        let alpha = unit(opacity) * coverage as f32;
        let code = |x: f32| i64::from((x.min(1.) * 255.).round_ties_even() as u8);
        let (s, a) = (code(unit(color) * alpha), code(alpha));
        let t = destination as i64 * (255 - a);
        (s + ((t + (t >> 8) + 128) >> 8)).min(255) as f64
    }
}

fn decode(byte: f64) -> f64 {
    let s = byte / 255.;
    if s <= 0.04045 {
        s / 12.92
    } else {
        ((s + 0.055) / 1.055).powf(2.4)
    }
}
fn encode(v: f64) -> f64 {
    255. * if v <= 0.0031308 {
        12.92 * v
    } else {
        1.055 * v.powf(1. / 2.4) - 0.055
    }
}
/// An independently evaluated candidate, not an admission tolerance.
fn measured_linear(value: f64) -> f64 {
    (value.clamp(0., 1.) * 4095.).round() / 4095.
}
fn snap(v: f64) -> f64 {
    (v + 0.5).floor()
}
/// Deterministic opaque background, so blending is checked over varied colour.
fn background(size: [u32; 2]) -> Vec<u8> {
    (0..size[1])
        .flat_map(|y| {
            (0..size[0]).flat_map(move |x| {
                [
                    (x * 7 % 256) as u8,
                    (y * 5 % 256) as u8,
                    ((x + y) * 3 % 256) as u8,
                    255,
                ]
            })
        })
        .collect()
}

/// The label halo a reference draws: the contract, or a wrong one that the
/// mutation checks must reject.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Halo {
    Contract,
    Absent,
    /// The glyph's own coverage in the halo colour, not its prepared halo.
    Undilated,
    /// Drawn after the label's glyphs instead of before them.
    Over,
    /// Cut at the text clip instead of the grown halo clip.
    ContentClip,
}

/// One bilinear tap of an R8 atlas at texel position `(u, v)`, as the lowest
/// and highest coverage the backend may return: exact on a texel centre or
/// between equal texels, else within TAP (twice TAP when a position is not a
/// multiple of 1/256 texel). The candidate models give one value.
fn tap(atlas: &GlyphAtlas, model: Model, u: f64, v: f64) -> [f64; 2] {
    if model == Model::Llvmpipe {
        return [llvmpipe::tap(atlas, u, v); 2];
    }
    let [aw, _] = atlas.size();
    let texel = |x: i64, y: i64| f64::from(atlas.pixels()[(y * i64::from(aw) + x) as usize]) / 255.;
    let (x, y) = (u - 0.5, v - 0.5);
    let (i, j) = (x.floor(), y.floor());
    let (fx, fy) = (x - i, y - j);
    let (i, j) = (i as i64, j as i64);
    let corners = [
        texel(i, j),
        texel(i + 1, j),
        texel(i, j + 1),
        texel(i + 1, j + 1),
    ];
    let exact = (corners[0] * (1. - fx) + corners[1] * fx) * (1. - fy)
        + (corners[2] * (1. - fx) + corners[3] * fx) * fy;
    let flat = corners.iter().all(|c| *c == corners[0]);
    let weights = [u, v].map(|p| (p - 0.5).fract());
    let e = if model != Model::Portable || flat || weights == [0.; 2] {
        0.
    } else if weights.iter().all(|w| (w * 256.).fract() == 0.) {
        TAP
    } else {
        2. * TAP
    };
    [(exact - e).max(0.), (exact + e).min(1.)]
}

/// Reference compositor in the target's blending space (linear light for sRGB
/// targets, encoded values for UNORM targets) over stored 8-bit codes: every
/// blend reads the stored destination and stores a code again, as the target
/// does. Each channel holds its lowest and highest admitted code; the
/// candidate models hold one code.
struct Canvas {
    size: [u32; 2],
    srgb: bool,
    model: Model,
    halo: Halo,
    /// Physical region origin and per-axis scale.
    region: [f64; 4],
    scale: [f64; 2],
    pixels: Vec<[[f64; 2]; 3]>,
}
impl Canvas {
    fn new(
        bytes: &[u8],
        size: [u32; 2],
        srgb: bool,
        region: [f64; 4],
        viewport: [f32; 2],
        model: Model,
    ) -> Self {
        Self {
            size,
            srgb,
            model,
            halo: Halo::Contract,
            region,
            scale: [
                region[2] / f64::from(viewport[0]),
                region[3] / f64::from(viewport[1]),
            ],
            pixels: bytes
                .as_chunks::<4>()
                .0
                .iter()
                .map(|p| std::array::from_fn(|c| [f64::from(p[c]); 2]))
                .collect(),
        }
    }
    /// A candidate model's image (the lowest admitted code otherwise).
    fn image(&self) -> Vec<u8> {
        self.pixels
            .iter()
            .flat_map(|p| [p[0][0] as u8, p[1][0] as u8, p[2][0] as u8, 255])
            .collect()
    }
    /// `coverage` is the lowest and highest coverage the backend may produce.
    /// A fragment adding nothing is discarded and stores nothing.
    fn over(&mut self, x: u32, y: u32, color: [u8; 4], coverage: [f64; 2]) {
        let alpha = coverage.map(|c| f64::from(color[3]) / 255. * c);
        if alpha == [0.; 2] {
            return;
        }
        let (srgb, model) = (self.srgb, self.model);
        let pixel = &mut self.pixels[(y * self.size[0] + x) as usize];
        for c in 0..3 {
            let src = self_space(srgb, f64::from(color[c]));
            let [low, high] = pixel[c];
            if model == Model::Llvmpipe {
                // A zero llvmpipe tap was discarded above.
                let a = alpha[0] as f32;
                let stored = if srgb {
                    llvmpipe::encode(src as f32 * a + llvmpipe::decode(low) * (1. - a))
                } else {
                    llvmpipe::unorm_over(color[c], color[3], coverage[0], low)
                };
                pixel[c] = [stored; 2];
                continue;
            }
            if model == Model::ObservedM5 && srgb {
                let a = alpha[0];
                let blended = src * a + measured_linear(decode(low)) * (1. - a);
                let stored = encode(measured_linear(blended)).round();
                pixel[c] = [stored; 2];
                continue;
            }
            let (read, write) = match model {
                Model::Portable => (DECODE, ENCODE),
                _ => (0., 0.5),
            };
            let destination = if srgb {
                [
                    decode((low - read).max(0.)),
                    decode((high + read).min(255.)),
                ]
            } else {
                [low / 255., high / 255.]
            };
            let blended = alpha
                .into_iter()
                .flat_map(|a| destination.map(|d| src * a + d * (1. - a)));
            let (min, max) = blended.fold((f64::MAX, f64::MIN), |(l, h), v| (l.min(v), h.max(v)));
            let (min, max) = if model == Model::Portable && srgb {
                ((min - LINEAR_ENCODE).max(0.), (max + LINEAR_ENCODE).min(1.))
            } else {
                (min, max)
            };
            let stored = |v: f64| if srgb { encode(v) } else { v * 255. };
            let float = [
                (stored(min) - write).ceil().max(0.),
                (stored(max) + write).floor().min(255.),
            ];
            pixel[c] = if model == Model::Portable && !srgb {
                let fixed = fixed_point(src, alpha, [low, high]);
                [float[0].min(fixed[0]), float[1].max(fixed[1])]
            } else {
                float
            };
        }
    }
    fn physical(&self, x: f64, axis: usize) -> f64 {
        self.region[axis] + x * self.scale[axis]
    }
    /// Snapped clip in physical pixels, limited to the region.
    fn clip(&self, clip: UiRect) -> [f64; 4] {
        [
            snap(self.physical(f64::from(clip[0]), 0)).max(snap(self.region[0])),
            snap(self.physical(f64::from(clip[1]), 1)).max(snap(self.region[1])),
            snap(self.physical(f64::from(clip[2]), 0)).min(snap(self.region[0] + self.region[2])),
            snap(self.physical(f64::from(clip[3]), 1)).min(snap(self.region[1] + self.region[3])),
        ]
    }
    fn pixels_in(&self, r: [f64; 4]) -> impl Iterator<Item = (u32, u32, f64, f64)> {
        let [w, h] = self.size;
        (0..h).flat_map(move |y| {
            (0..w).filter_map(move |x| {
                let (cx, cy) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
                (cx >= r[0] && cx < r[2] && cy >= r[1] && cy < r[3]).then_some((x, y, cx, cy))
            })
        })
    }
    /// Rounded rectangle (or its inset stroke) at snapped edges, by signed distance.
    fn rect(
        &mut self,
        rect: UiRect,
        clip: UiRect,
        radius: f32,
        stroke: Option<f32>,
        color: [u8; 4],
    ) {
        let p0 = [
            snap(self.physical(f64::from(rect[0]), 0)),
            snap(self.physical(f64::from(rect[1]), 1)),
        ];
        let p1 = [
            snap(self.physical(f64::from(rect[2]), 0)),
            snap(self.physical(f64::from(rect[3]), 1)),
        ];
        let c = self.clip(clip);
        let area = [
            p0[0].max(c[0]),
            p0[1].max(c[1]),
            p1[0].min(c[2]),
            p1[1].min(c[3]),
        ];
        let unit = self.scale[0].min(self.scale[1]);
        let half = [(p1[0] - p0[0]) / 2., (p1[1] - p0[1]) / 2.];
        let center = [(p0[0] + p1[0]) / 2., (p0[1] + p1[1]) / 2.];
        let r = (f64::from(radius) * unit).min(half[0]).min(half[1]);
        let distance = |x: f64, y: f64, half: [f64; 2], r: f64| {
            let q = [
                (x - center[0]).abs() - half[0] + r,
                (y - center[1]).abs() - half[1] + r,
            ];
            (q[0].max(0.).powi(2) + q[1].max(0.).powi(2)).sqrt() + q[0].max(q[1]).min(0.) - r
        };
        let points: Vec<_> = self.pixels_in(area).collect();
        for (x, y, cx, cy) in points {
            let mut coverage = (0.5 - distance(cx, cy, half, r)).clamp(0., 1.);
            if let Some(width) = stroke {
                let w = (f64::from(width) * unit + 0.5).floor().max(1.);
                let inner = [half[0] - w, half[1] - w];
                if inner[0] > 0. && inner[1] > 0. {
                    let hole = (0.5 - distance(cx, cy, inner, (r - w).max(0.))).clamp(0., 1.);
                    coverage = (coverage - hole).max(0.);
                }
            }
            self.over(x, y, color, [coverage; 2]);
        }
    }
    /// Glyph coverage: snapped pen, padded rectangle at `scale` pixels per
    /// texel, one bilinear tap clamped to the padded texels (scale >= 0.8).
    /// A label first draws its halos (muted with disabled text).
    fn glyphs(&mut self, atlas: &GlyphAtlas, widget: &UiWidget, color: [u8; 4]) {
        let label = widget.role == UiRole::Label;
        let halo = if widget.disabled {
            theme::alpha(
                theme::LABEL_HALO,
                f32::from(theme::TEXT_DISABLED_ALPHA) / 255.,
            )
        } else {
            theme::LABEL_HALO
        };
        if label && !matches!(self.halo, Halo::Absent | Halo::Over) {
            self.halo_pass(atlas, widget, halo);
        }
        for (glyph, handle) in widget.layout.glyphs().iter().zip(widget.glyphs.iter()) {
            let entry = *atlas.get(handle).unwrap();
            self.entry(atlas, pen(widget, glyph), &entry, widget.clip, color);
        }
        if label && self.halo == Halo::Over {
            self.halo_pass(atlas, widget, halo);
        }
    }
    /// One atlas entry at a snapped logical pen, cut by the snapped `clip`.
    fn entry(
        &mut self,
        atlas: &GlyphAtlas,
        pen: [f32; 2],
        entry: &AtlasEntry,
        clip: UiRect,
        color: [u8; 4],
    ) {
        assert!(self.scale.iter().all(|s| *s >= 0.8), "single-tap reference");
        let Some(rect) = entry.rect else { return };
        let model = self.model;
        let c = self.clip(clip);
        let [x, y, w, h] = rect.map(f64::from);
        let [ox, oy] = entry.origin.map(f64::from);
        let pen = [
            snap(self.physical(f64::from(pen[0]), 0)),
            snap(self.physical(f64::from(pen[1]), 1)),
        ];
        let p0 = [
            pen[0] + (ox - 1.) * self.scale[0],
            pen[1] + (oy - 1.) * self.scale[1],
        ];
        let p1 = [
            pen[0] + (ox + w + 1.) * self.scale[0],
            pen[1] + (oy + h + 1.) * self.scale[1],
        ];
        let area = [
            p0[0].max(c[0]),
            p0[1].max(c[1]),
            p1[0].min(c[2]),
            p1[1].min(c[3]),
        ];
        let points: Vec<_> = self.pixels_in(area).collect();
        for (px, py, cx, cy) in points {
            let u = (x - 1. + (cx - p0[0]) / self.scale[0]).clamp(x - 0.5, x + w + 0.5);
            let v = (y - 1. + (cy - p0[1]) / self.scale[1]).clamp(y - 0.5, y + h + 0.5);
            self.over(px, py, color, tap(atlas, model, u, v));
        }
    }
    /// A label's prepared halos (`UiWidget::halos`), drawn exactly as glyphs:
    /// each glyph's halo entry at that glyph's pen, clipped to the text clip
    /// grown by the halo margin plus one within the visible box, and nothing
    /// where the text clip is empty. The margin comes from the documented
    /// rule, m = ceil(clamp(size / 16, 1, 2.5)); each entry must be m texels
    /// larger than its glyph on every side.
    fn halo_pass(&mut self, atlas: &GlyphAtlas, widget: &UiWidget, color: [u8; 4]) {
        let text = widget.clip;
        if text[2] <= text[0] || text[3] <= text[1] {
            return;
        }
        let halos = widget.halos.as_ref().expect("a label's halos");
        let placements = widget.layout.glyphs().iter().zip(widget.glyphs.iter());
        for ((glyph, handle), halo) in placements.zip(halos.iter()) {
            let (own, entry) = (*atlas.get(handle).unwrap(), *atlas.get(halo).unwrap());
            let margin = (glyph.font_size / 16.).clamp(1., 2.5).ceil();
            let m = margin as i32;
            assert_eq!(entry.origin, [own.origin[0] - m, own.origin[1] - m]);
            if let (Some(g), Some(h)) = (own.rect, entry.rect) {
                assert_eq!([h[2], h[3]], [g[2] + 2 * m as u32, g[3] + 2 * m as u32]);
            }
            let (grow, visible) = (margin + 1., widget.visible);
            let clip = if self.halo == Halo::ContentClip {
                text
            } else {
                [
                    (text[0] - grow).max(visible[0]),
                    (text[1] - grow).max(visible[1]),
                    (text[2] + grow).min(visible[2]),
                    (text[3] + grow).min(visible[3]),
                ]
            };
            let drawn = if self.halo == Halo::Undilated {
                own
            } else {
                entry
            };
            self.entry(atlas, pen(widget, glyph), &drawn, clip, color);
        }
    }
}
/// A glyph's logical pen: content origin plus its shaped position.
fn pen(widget: &UiWidget, glyph: &incant_text::Glyph) -> [f32; 2] {
    [
        widget.origin[0] + glyph.position[0],
        widget.origin[1] + glyph.position[1],
    ]
}
fn self_space(srgb: bool, byte: f64) -> f64 {
    if srgb { decode(byte) } else { byte / 255. }
}
/// The codes a UNORM blend at the target's 8-bit precision may store over the
/// destination codes `[low, high]`: source and alpha codes within `CONVERT`,
/// the destination product within `PRODUCT`, then a saturating sum.
fn fixed_point(src: f64, alpha: [f64; 2], [low, high]: [f64; 2]) -> [f64; 2] {
    let codes = |[l, h]: [f64; 2]| {
        [
            (l * 255. - CONVERT).ceil().max(0.),
            (h * 255. + CONVERT).floor().min(255.),
        ]
    };
    let source = codes(alpha.map(|a| src * a));
    let a = codes(alpha);
    let product = [low * (255. - a[1]) / 255., high * (255. - a[0]) / 255.];
    [
        (source[0] + (product[0] - PRODUCT).ceil().max(0.)).min(255.),
        (source[1] + (product[1] + PRODUCT).floor()).min(255.),
    ]
}

struct Run {
    pixels: Vec<u8>,
    background: Vec<u8>,
}
fn run(
    renderer: &Renderer,
    layer: &mut UiLayer,
    input: &Arc<UiInput>,
    format: wgpu::TextureFormat,
    size: [u32; 2],
    region: Option<[f32; 4]>,
) -> Run {
    run_over(
        renderer,
        layer,
        input,
        format,
        size,
        region,
        background(size),
    )
}
/// `run` over a caller's opaque backdrop instead of the deterministic one.
fn run_over(
    renderer: &Renderer,
    layer: &mut UiLayer,
    input: &Arc<UiInput>,
    format: wgpu::TextureFormat,
    size: [u32; 2],
    region: Option<[f32; 4]>,
    background: Vec<u8>,
) -> Run {
    let target = texture(renderer, format, size[0], size[1]);
    renderer.queue.write_texture(
        target.as_image_copy(),
        &background,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(size[0] * 4),
            rows_per_image: Some(size[1]),
        },
        target.size(),
    );
    let view = target.create_view(&Default::default());
    let stage = layer.stage(renderer, input).unwrap();
    layer
        .submit(
            renderer,
            stage,
            &RetainedTarget {
                view: &view,
                format,
                width: size[0],
                height: size[1],
                viewport: region.map(|rect| Viewport {
                    rect,
                    corner_radii: [0.; 4],
                    canvas_srgb: [0; 3],
                }),
            },
        )
        .unwrap();
    Run {
        pixels: read(renderer, &target),
        background,
    }
}
/// How an image compares with a reference's admitted codes.
#[derive(Debug)]
struct Admission {
    /// Channels outside their admitted codes (or alpha other than 255).
    outside: usize,
    /// Largest distance outside, in codes.
    excess: f64,
    /// Widest admitted span and the number of channels wider than two codes.
    widest: f64,
    wide: usize,
}
fn admitted(image: &[u8], reference: &Canvas) -> Admission {
    let mut result = Admission {
        outside: 0,
        excess: 0.,
        widest: 0.,
        wide: 0,
    };
    for (pixel, allowed) in image.as_chunks::<4>().0.iter().zip(&reference.pixels) {
        for c in 0..3 {
            let [low, high] = allowed[c];
            let value = f64::from(pixel[c]);
            let distance = (low - value).max(value - high);
            if distance > 0. {
                result.outside += 1;
                result.excess = result.excess.max(distance);
            }
            result.widest = result.widest.max(high - low);
            result.wide += usize::from(high - low > 2.);
        }
        if pixel[3] != 255 {
            result.outside += 1;
        }
    }
    result
}
/// Largest per-channel difference, and how many channels differ by more than 1.
fn difference(gpu: &[u8], expected: &[u8]) -> (u8, usize) {
    assert_eq!(gpu.len(), expected.len());
    gpu.iter()
        .zip(expected)
        .fold((0, 0), |(worst, over), (a, b)| {
            let d = a.abs_diff(*b);
            (worst.max(d), over + usize::from(d > 1))
        })
}
fn ink(pixels: &[u8], background: &[u8]) -> usize {
    pixels
        .as_chunks::<4>()
        .0
        .iter()
        .zip(background.as_chunks::<4>().0)
        .filter(|(a, b)| a != b)
        .count()
}

/// One labelled button, one mixed-script label (padded by its halo's reach, so
/// its halo clip is its border box) and one focused field with a selection, at
/// fractional logical positions.
fn panel() -> Arc<UiInput> {
    let mut f = Fixture::new();
    let button = f.widget(
        1,
        UiRole::Button,
        [10.3, 10.6, 90.4, 38.2],
        7.,
        "Go 中文",
        16.,
    );
    let label = f.widget(
        2,
        UiRole::Label,
        [8., 46., 152., 70.],
        2.,
        "ffi سلام 中文",
        15.,
    );
    let mut field = f.widget(
        3,
        UiRole::TextInput,
        [96.5, 10.5, 152.25, 38.5],
        6.,
        "abc",
        16.,
    );
    field.focused = true;
    field.edit = Some(UiEdit {
        selection: Some(Selection {
            anchor: 1,
            focus: 3,
        }),
        preedit: None,
    });
    f.finish([160., 90.], vec![button, label, field])
}
/// The reference picture of `panel()`: surfaces, selection, glyphs, ring.
/// `swapped` draws the field's text under its selection (a wrong order);
/// `text` is the glyph colour (a wrong opacity in candidates).
fn reference_order(input: &UiInput, canvas: &mut Canvas, swapped: bool, text: [u8; 4]) {
    let [button, label, field] = &input.widgets[..] else {
        panic!("panel")
    };
    canvas.rect(
        button.bounds,
        button.visible,
        theme::RADIUS,
        None,
        theme::BUTTON,
    );
    canvas.glyphs(&input.atlas, button, text);
    canvas.glyphs(&input.atlas, label, text);
    canvas.rect(
        field.bounds,
        field.visible,
        theme::RADIUS,
        None,
        theme::FIELD,
    );
    if swapped {
        canvas.glyphs(&input.atlas, field, text);
    }
    // Selection 1..3 of "abc": from the left edge of "b" to the right of "c".
    let glyphs = field.layout.glyphs();
    let line = &field.layout.lines()[0];
    let (b, c) = (&glyphs[1], &glyphs[2]);
    canvas.rect(
        [
            field.origin[0] + b.position[0],
            field.origin[1] + line.top,
            field.origin[0] + c.position[0] + c.advance,
            field.origin[1] + line.top + line.height,
        ],
        field.visible,
        0.,
        None,
        theme::SELECTION,
    );
    if !swapped {
        canvas.glyphs(&input.atlas, field, text);
    }
    canvas.rect(
        field.bounds,
        field.visible,
        theme::RADIUS,
        Some(theme::FOCUS_WIDTH),
        theme::ACCENT,
    );
}
fn reference(input: &UiInput, canvas: &mut Canvas) {
    reference_order(input, canvas, false, theme::TEXT);
}
/// `panel()` composed over `background`, filling a target of `size`.
fn panel_canvas(
    input: &UiInput,
    background: &[u8],
    size: [u32; 2],
    srgb: bool,
    model: Model,
) -> Canvas {
    let full = [0., 0., f64::from(size[0]), f64::from(size[1])];
    let mut canvas = Canvas::new(background, size, srgb, full, input.viewport, model);
    reference(input, &mut canvas);
    canvas
}

#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn logical_layout_maps_to_physical_pixels_at_integer_and_fractional_scales() {
    let renderer = renderer_with_adapter_receipt();
    let input = panel();
    for (scale, format) in [(1., SRGB), (2., SRGB), (1.5, SRGB), (1., UNORM)] {
        let size = [(160. * scale) as u32, (90. * scale) as u32];
        let mut layer = renderer.ui_layer();
        let run = run(&renderer, &mut layer, &input, format, size, None);
        let srgb = format.is_srgb();
        let portable = panel_canvas(&input, &run.background, size, srgb, Model::Portable);
        let expected = panel_canvas(&input, &run.background, size, srgb, Model::Exact).image();
        let admission = admitted(&run.pixels, &portable);
        let (worst, over) = difference(&run.pixels, &expected);
        let changed = ink(&run.pixels, &run.background);
        eprintln!(
            "scale {scale} {format:?}: outside {} by {}, widest {} ({} channels > 2); \
             vs exact worst {worst}, >1: {over}, changed {changed}",
            admission.outside, admission.excess, admission.widest, admission.wide
        );
        save(
            &format!("ui-panel-{scale}x-{format:?}"),
            &run.pixels,
            size[0],
            size[1],
        );
        assert_eq!(admission.outside, 0, "{scale}: {admission:?}");
        // Nothing outside every widget's snapped visible box changes at all.
        let boxes: Vec<_> = input
            .widgets
            .iter()
            .map(|w| w.visible.map(|v| snap(f64::from(v) * scale)))
            .collect();
        for (i, (a, b)) in run
            .pixels
            .chunks(4)
            .zip(run.background.chunks(4))
            .enumerate()
        {
            let (x, y) = (
                (i as u32 % size[0]) as f64 + 0.5,
                (i as u32 / size[0]) as f64 + 0.5,
            );
            if !boxes
                .iter()
                .any(|r| x >= r[0] && x < r[2] && y >= r[1] && y < r[3])
            {
                assert_eq!(a, b, "pixel {i} outside the UI changed");
            }
        }
        assert!(changed * 2 > ink(&expected, &run.background));
        // Surface edges are pixel exact: the snapped button edge and its outside.
        let button = input.widgets[0]
            .bounds
            .map(|v| snap(f64::from(v) * scale) as u32);
        let row = button[1] + (button[3] - button[1]) / 2;
        let at = |x: u32| ((row * size[0] + x) * 4) as usize;
        assert_eq!(
            run.pixels[at(button[0] - 1)..at(button[0])],
            run.background[at(button[0] - 1)..at(button[0])]
        );
        assert_ne!(
            run.pixels[at(button[0])..at(button[0] + 1)],
            run.background[at(button[0])..at(button[0] + 1)]
        );
        assert_eq!(
            run.pixels[at(button[2])..at(button[2] + 1)],
            run.background[at(button[2])..at(button[2] + 1)]
        );
        let stats = layer.stats().unwrap();
        assert_eq!(stats.widgets, 3);
        assert!(stats.glyph_quads > 10);
    }
}

/// An overflowing RTL label, open and under a narrower ancestor clip, and a
/// button partly hidden by the same ancestor, in a 120x90 target at 1x.
struct Nested {
    atlas: Arc<GlyphAtlas>,
    label: UiWidget,
    clipped: UiWidget,
    button: UiWidget,
}
fn nested() -> Nested {
    let mut f = Fixture::new();
    let token = "سلامعليكم";
    let mut label = f.widget(1, UiRole::Label, [40., 10., 80., 40.], 0., token, 20.);
    let (layout, glyphs) = f.shape(token, 20., 40., Wrap::Word);
    label.halos = Some(f.halos(&layout));
    label.layout = layout;
    label.glyphs = glyphs;
    assert!(label.layout.glyphs().iter().any(|g| g.position[0] < -10.));
    // An ancestor clip narrower than the content box, as a nested container gives.
    let ancestor: UiRect = [44., 12., 74., 38.];
    let mut clipped = label.clone();
    clipped.clip = ancestor;
    clipped.visible = ancestor;
    let mut button = f.widget(
        2,
        UiRole::Button,
        [30., 50., 90., 76.],
        6.,
        "Wide label",
        14.,
    );
    button.visible = [44., 50., 74., 76.];
    button.clip = [44., 56., 74., 70.];
    Nested {
        atlas: Arc::new(f.atlas),
        label,
        clipped,
        button,
    }
}
impl Nested {
    const SIZE: [u32; 2] = [120, 90];
    fn input(&self, widgets: Vec<UiWidget>) -> Arc<UiInput> {
        Arc::new(UiInput {
            viewport: [120., 90.],
            atlas: Arc::clone(&self.atlas),
            widgets,
        })
    }
    /// `label` (normally the clipped one), then the button.
    fn canvas(&self, background: &[u8], label: &UiWidget, model: Model) -> Canvas {
        let region = [0., 0., 120., 90.];
        let mut canvas = Canvas::new(background, Self::SIZE, true, region, [120., 90.], model);
        canvas.glyphs(&self.atlas, label, theme::TEXT);
        canvas.rect(
            self.button.bounds,
            self.button.visible,
            theme::RADIUS,
            None,
            theme::BUTTON,
        );
        canvas.glyphs(&self.atlas, &self.button, theme::TEXT);
        canvas
    }
}

#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn nested_clips_cut_actual_glyph_bounds_including_negative_x_rtl_overflow() {
    let renderer = renderer_with_adapter_receipt();
    let n = nested();
    let size = Nested::SIZE;
    let open = n.input(vec![n.label.clone()]);
    let nested = n.input(vec![n.clipped.clone(), n.button.clone()]);
    let mut layer = renderer.ui_layer();
    let open_run = run(&renderer, &mut layer, &open, SRGB, size, None);
    let nested_run = run(&renderer, &mut layer, &nested, SRGB, size, None);
    save("ui-clip-open", &open_run.pixels, size[0], size[1]);
    save("ui-clip-nested", &nested_run.pixels, size[0], size[1]);
    let background = &nested_run.background;
    let admission = admitted(
        &nested_run.pixels,
        &n.canvas(background, &n.clipped, Model::Portable),
    );
    let exact = n.canvas(background, &n.clipped, Model::Exact).image();
    let (worst, over) = difference(&nested_run.pixels, &exact);
    eprintln!(
        "nested clips: outside {} by {}, widest {} ({} channels > 2); vs exact worst {worst}, >1: {over}",
        admission.outside, admission.excess, admission.widest, admission.wide
    );
    assert_eq!(admission.outside, 0, "{admission:?}");
    // The overflowing RTL token really inks left of its own box when unclipped
    // by an ancestor, and nothing outside either clip changes when nested.
    let inked = |run: &Run, x: std::ops::Range<u32>, y: std::ops::Range<u32>| {
        y.clone().any(|py| {
            x.clone().any(|px| {
                let i = ((py * size[0] + px) * 4) as usize;
                run.pixels[i..i + 3] != run.background[i..i + 3]
            })
        })
    };
    assert!(!inked(&open_run, 0..40, 0..90), "content clip at x=40");
    assert!(inked(&open_run, 40..44, 10..40) || inked(&open_run, 74..80, 10..40));
    for py in 0..size[1] {
        for px in 0..size[0] {
            let inside_label = (44..74).contains(&px) && (12..38).contains(&py);
            let inside_button = (44..74).contains(&px) && (50..76).contains(&py);
            if !inside_label && !inside_button {
                let i = ((py * size[0] + px) * 4) as usize;
                assert_eq!(
                    nested_run.pixels[i..i + 4],
                    nested_run.background[i..i + 4],
                    "{px},{py}"
                );
            }
        }
    }
}

/// A mixed-script label at 1x whose text clip hugs its ink, inside 4 px of
/// padding: none of its text is cut, and its halos show only through the
/// documented growth past that clip.
fn hugged() -> (Arc<UiInput>, [u32; 2]) {
    let mut f = Fixture::new();
    let bounds = [6., 6., 214., 44.];
    let mut label = f.widget(1, UiRole::Label, bounds, 4., "Samples 4/4 中文 سلام", 17.);
    let mut ink = [f32::MAX, f32::MAX, f32::MIN, f32::MIN];
    for (glyph, handle) in label.layout.glyphs().iter().zip(label.glyphs.iter()) {
        let entry = f.atlas.get(handle).unwrap();
        let Some([_, _, w, h]) = entry.rect else {
            continue;
        };
        // At 1x the pen snaps to a whole pixel and texels land on pixels.
        let [x, y] = pen(&label, glyph).map(|v| (v + 0.5).floor());
        let [left, top] = [x + entry.origin[0] as f32, y + entry.origin[1] as f32];
        ink = [
            ink[0].min(left),
            ink[1].min(top),
            ink[2].max(left + w as f32),
            ink[3].max(top + h as f32),
        ];
    }
    let c = label.clip;
    assert!(
        ink[0] >= c[0] && ink[1] >= c[1] && ink[2] <= c[2] && ink[3] <= c[3],
        "{ink:?} {c:?}"
    );
    label.clip = ink;
    (f.finish([220., 50.], vec![label]), [220, 50])
}

/// The hugged label over pure white and over black. Each draw stays within
/// the reference, including the halo past the text clip, and over white the
/// halo really separates the text: the text colour measures at least 4.5:1
/// against the darkest pixel drawn.
#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn label_halo_separates_text_from_bright_and_dark_backdrops() {
    let renderer = renderer_with_adapter_receipt();
    let (input, size) = hugged();
    let label = &input.widgets[0];
    let luminance = |p: &[u8]| {
        let c = |v: u8| decode(f64::from(v));
        0.2126 * c(p[0]) + 0.7152 * c(p[1]) + 0.0722 * c(p[2])
    };
    let text = luminance(&theme::TEXT[..]);
    for (name, value) in [("white", 255u8), ("black", 0u8)] {
        let backdrop: Vec<u8> = (0..size[0] * size[1])
            .flat_map(|_| [value, value, value, 255])
            .collect();
        let mut layer = renderer.ui_layer();
        let run = run_over(
            &renderer,
            &mut layer,
            &input,
            SRGB,
            size,
            None,
            backdrop.clone(),
        );
        save(&format!("ui-halo-{name}"), &run.pixels, size[0], size[1]);
        let full = [0., 0., f64::from(size[0]), f64::from(size[1])];
        let mut portable =
            Canvas::new(&backdrop, size, true, full, input.viewport, Model::Portable);
        portable.glyphs(&input.atlas, label, theme::TEXT);
        let admission = admitted(&run.pixels, &portable);
        let darkest = run
            .pixels
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| luminance(&p[..]))
            .fold(f64::MAX, f64::min);
        let ratio = (text + 0.05) / (darkest + 0.05);
        eprintln!(
            "halo over {name}: outside {} by {}, widest {}; text {ratio:.2}:1 against the darkest pixel",
            admission.outside, admission.excess, admission.widest
        );
        assert_eq!(admission.outside, 0, "{name}: {admission:?}");
        if value == 255 {
            assert!(ratio >= 4.5, "{ratio}");
        }
    }
}

#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn ui_composites_over_the_finished_world_frame_and_leaves_uncovered_pixels() {
    use crate::text::draw_tests::{pixel_camera, scene};
    use glam::{DMat4, DVec3};
    let renderer = renderer_with_adapter_receipt();
    let size = [160, 90];
    let world_text = crate::text::tests::input(&[(
        9,
        crate::text::tests::MIXED,
        24.,
        DMat4::from_translation(DVec3::new(-0.75, 0.3, 0.)),
    )]);
    let world = renderer
        .prepare(&scene(world_text, pixel_camera(size)))
        .unwrap()
        .with_camera("camera")
        .unwrap();
    let frame = |ui: Option<(&mut UiLayer, &Arc<UiInput>)>| {
        let target = texture(&renderer, SRGB, size[0], size[1]);
        let view = target.create_view(&Default::default());
        renderer.queue.submit([renderer
            .draw_scene(&world, &view, SRGB, size[0], size[1], None)
            .unwrap()]);
        if let Some((layer, input)) = ui {
            let stage = layer.stage(&renderer, input).unwrap();
            layer
                .submit(
                    &renderer,
                    stage,
                    &RetainedTarget {
                        view: &view,
                        format: SRGB,
                        width: size[0],
                        height: size[1],
                        viewport: None,
                    },
                )
                .unwrap();
        }
        read(&renderer, &target)
    };
    let world_only = frame(None);
    let input = panel();
    let mut layer = renderer.ui_layer();
    let composed = frame(Some((&mut layer, &input)));
    save("ui-over-world", &composed, size[0], size[1]);
    let admission = admitted(
        &composed,
        &panel_canvas(&input, &world_only, size, true, Model::Portable),
    );
    let exact = panel_canvas(&input, &world_only, size, true, Model::Exact).image();
    let (worst, over) = difference(&composed, &exact);
    eprintln!(
        "over world: outside {} by {}, widest {} ({} channels > 2); vs exact worst {worst}, >1: {over}",
        admission.outside, admission.excess, admission.widest, admission.wide
    );
    assert_eq!(admission.outside, 0, "{admission:?}");
    // The world really shows through outside the UI and under translucent
    // surfaces: world text pixels differ from the backdrop there.
    let backdrop = [20u8, 21, 25];
    let world_ink = world_only
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| p[..3] != backdrop)
        .count();
    assert!(world_ink > 100, "{world_ink}");
    // The comparison distinguishes layering: text under its selection is wrong.
    let full = [0., 0., 160., 90.];
    let mut swapped = Canvas::new(
        &world_only,
        size,
        true,
        full,
        input.viewport,
        Model::Portable,
    );
    reference_order(&input, &mut swapped, true, theme::TEXT);
    let wrong = admitted(&composed, &swapped);
    eprintln!(
        "over world, text under selection: outside {} by {}",
        wrong.outside, wrong.excess
    );
    assert!(
        wrong.outside > 20 && wrong.excess > 8.,
        "selection/text order is not observable: {wrong:?}"
    );
}

#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn retained_atlas_and_quads_follow_input_identity_and_survive_refused_stages() {
    let renderer = renderer_with_adapter_receipt();
    let size = [160, 90];
    let input = panel();
    let mut layer = renderer.ui_layer();
    let before = renderer.ui_uploads();
    let first = run(&renderer, &mut layer, &input, SRGB, size, None);
    let after = renderer.ui_uploads();
    assert_eq!(after.atlas_textures - before.atlas_textures, 1);
    assert_eq!(after.quad_buffers - before.quad_buffers, 1);
    assert_eq!(after.frame_writes - before.frame_writes, 1);
    assert!(after.atlas_bytes > before.atlas_bytes);
    // The identical Arc: no upload of any kind, identical pixels.
    let again = run(&renderer, &mut layer, &input, SRGB, size, None);
    assert_eq!(renderer.ui_uploads(), after);
    assert_eq!(again.pixels, first.pixels);
    // A new frame sharing the atlas (focus moved): one instance buffer only.
    let mut widgets = input.widgets.clone();
    widgets[2].focused = false;
    widgets[0].focused = true;
    let moved = Arc::new(UiInput {
        viewport: input.viewport,
        atlas: Arc::clone(&input.atlas),
        widgets,
    });
    let atlas_gpu = layer.accepted_atlas().unwrap();
    let focus = run(&renderer, &mut layer, &moved, SRGB, size, None);
    let now = renderer.ui_uploads();
    assert_eq!(now.atlas_textures, after.atlas_textures);
    assert_eq!(now.quad_buffers, after.quad_buffers + 1);
    assert_ne!(focus.pixels, first.pixels);
    assert!(Weak::ptr_eq(&atlas_gpu, &layer.accepted_atlas().unwrap()));
    // A refused candidate (handle from a reset atlas) changes nothing accepted.
    let mut f = Fixture::new();
    let w = f.widget(1, UiRole::Label, [0., 0., 50., 20.], 0., "x", 12.);
    f.atlas.reset().unwrap();
    let refused = f.finish([160., 90.], vec![w]);
    assert!(layer.stage(&renderer, &refused).is_err());
    assert_eq!(renderer.ui_uploads(), now);
    let kept = run(&renderer, &mut layer, &moved, SRGB, size, None);
    assert_eq!(kept.pixels, focus.pixels);
    assert_eq!(renderer.ui_uploads(), now);
    // A new atlas uploads once; the replaced GPU atlas is released with it.
    let fresh = panel();
    run(&renderer, &mut layer, &fresh, SRGB, size, None);
    let last = renderer.ui_uploads();
    assert_eq!(last.atlas_textures, now.atlas_textures + 1);
    drop((input, moved));
    assert!(atlas_gpu.upgrade().is_none(), "old GPU atlas retained");
    // A new target size rewrites only the frame region uniform.
    run(&renderer, &mut layer, &fresh, SRGB, [320, 180], None);
    let resized = renderer.ui_uploads();
    assert_eq!(resized.frame_writes, last.frame_writes + 1);
    assert_eq!(
        (resized.atlas_textures, resized.quad_buffers),
        (last.atlas_textures, last.quad_buffers)
    );
    // A layer refuses another renderer's stage.
    let other = renderer_with_adapter_receipt();
    assert!(other.ui_layer().stage(&renderer, &fresh).is_err());
}

#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn editor_viewport_region_offsets_scales_and_bounds_the_ui() {
    let renderer = renderer_with_adapter_receipt();
    let size = [200, 140];
    let region = [20.5, 15., 160., 90.];
    let input = panel();
    let mut layer = renderer.ui_layer();
    let run = run(&renderer, &mut layer, &input, SRGB, size, Some(region));
    save("ui-region", &run.pixels, size[0], size[1]);
    let compose = |model| {
        let mut canvas = Canvas::new(
            &run.background,
            size,
            true,
            region.map(f64::from),
            input.viewport,
            model,
        );
        reference(&input, &mut canvas);
        canvas
    };
    let admission = admitted(&run.pixels, &compose(Model::Portable));
    let (worst, over) = difference(&run.pixels, &compose(Model::Exact).image());
    eprintln!(
        "region: outside {} by {}, widest {} ({} channels > 2); vs exact worst {worst}, >1: {over}",
        admission.outside, admission.excess, admission.widest, admission.wide
    );
    assert_eq!(admission.outside, 0, "{admission:?}");
    for py in 0..size[1] {
        for px in 0..size[0] {
            if !(21..181).contains(&px) || !(15..105).contains(&py) {
                let i = ((py * size[0] + px) * 4) as usize;
                assert_eq!(run.pixels[i..i + 4], run.background[i..i + 4], "{px},{py}");
            }
        }
    }
}

/// CPU only: the admitted codes accept exact arithmetic and an emulated
/// conformant backend, yet still reject wrong geometry, clipping, opacity,
/// blending space, layering and label halos by wide margins. Prints the
/// emulated backend's statistics as the GPU test does. For the earlier
/// one-pixel label shadow, its 1x UNORM distance from exact (worst 2, five
/// channels over 1, 4,248 changed pixels) matched what checkpoint 12's llvmpipe
/// and WARP draws printed; the prepared-halo panel has not been measured on
/// them yet.
#[test]
fn reference_admits_backend_precision_but_not_wrong_geometry_clip_or_opacity() {
    let input = panel();
    for (scale, format) in [(1., SRGB), (2., SRGB), (1.5, SRGB), (1., UNORM)] {
        let (srgb, case) = (format.is_srgb(), format!("scale {scale} {format:?}"));
        let size = [(160. * scale) as u32, (90. * scale) as u32];
        let background = background(size);
        let compose = |input: &UiInput, model, text, srgb| {
            let full = [0., 0., f64::from(size[0]), f64::from(size[1])];
            let mut canvas = Canvas::new(&background, size, srgb, full, input.viewport, model);
            reference_order(input, &mut canvas, false, text);
            canvas.image()
        };
        let candidate = |input: &UiInput, model, text| compose(input, model, text, srgb);
        let portable = panel_canvas(&input, &background, size, srgb, Model::Portable);
        let exact = candidate(&input, Model::Exact, theme::TEXT);
        assert_eq!(admitted(&exact, &portable).outside, 0, "{case} exact");
        let measured = candidate(&input, Model::ObservedM5, theme::TEXT);
        assert_eq!(
            admitted(&measured, &portable).outside,
            0,
            "{case} measured M5"
        );
        let emulated = candidate(&input, Model::Llvmpipe, theme::TEXT);
        let (worst, over) = difference(&emulated, &exact);
        let admission = admitted(&emulated, &portable);
        eprintln!(
            "{case}: emulated llvmpipe outside {} by {}, widest {} ({} channels > 2); \
             vs exact worst {worst}, >1: {over}, changed {}",
            admission.outside,
            admission.excess,
            admission.widest,
            admission.wide,
            ink(&emulated, &background)
        );
        assert_eq!(admission.outside, 0, "{case} llvmpipe: {admission:?}");
        // Linear-light blending on a UNORM target, encoded on an sRGB one.
        let wrong = admitted(
            &compose(&input, Model::Exact, theme::TEXT, !srgb),
            &portable,
        );
        eprintln!("{case}: blending in the other space rejected by {wrong:?}");
        assert!(
            wrong.outside > 100 && wrong.excess > 10.,
            "{case}: {wrong:?}"
        );
        // Text one physical pixel right, or half a pixel at 2x.
        for shift in [1., 0.5] {
            if shift * scale < 1. && scale != 2. {
                continue;
            }
            let mut widgets = input.widgets.clone();
            for widget in &mut widgets {
                widget.origin[0] += (shift / scale) as f32;
            }
            let moved = UiInput {
                viewport: input.viewport,
                atlas: Arc::clone(&input.atlas),
                widgets,
            };
            let wrong = admitted(&candidate(&moved, Model::Exact, theme::TEXT), &portable);
            eprintln!("{case}: text {shift} px right rejected by {wrong:?}");
            assert!(
                wrong.outside > 100 && wrong.excess > 30.,
                "{case}/{shift}: {wrong:?}"
            );
        }
        // Text at 90 % opacity.
        let faint = theme::alpha(theme::TEXT, 0.9);
        let wrong = admitted(&candidate(&input, Model::Exact, faint), &portable);
        eprintln!("{case}: 90% text opacity rejected by {wrong:?}");
        assert!(
            wrong.outside > 100 && wrong.excess > 5.,
            "{case}: {wrong:?}"
        );
        // A label drawn without its halo, with its undilated coverage as the
        // halo, or with the halo over its text.
        for halo in [Halo::Absent, Halo::Undilated, Halo::Over] {
            let full = [0., 0., f64::from(size[0]), f64::from(size[1])];
            let mut canvas =
                Canvas::new(&background, size, srgb, full, input.viewport, Model::Exact);
            canvas.halo = halo;
            reference_order(&input, &mut canvas, false, theme::TEXT);
            let wrong = admitted(&canvas.image(), &portable);
            eprintln!("{case}: label halo {halo:?} rejected by {wrong:?}");
            assert!(
                wrong.outside > 20 && wrong.excess > 8.,
                "{case}: {halo:?} {wrong:?}"
            );
        }
    }
    // A label drawn to its own content box instead of the ancestor clip.
    let n = nested();
    let background = background(Nested::SIZE);
    let portable = n.canvas(&background, &n.clipped, Model::Portable);
    for model in [Model::Exact, Model::Llvmpipe, Model::ObservedM5] {
        let image = n.canvas(&background, &n.clipped, model).image();
        let (worst, over) = difference(
            &image,
            &n.canvas(&background, &n.clipped, Model::Exact).image(),
        );
        eprintln!("nested {model:?} vs exact worst {worst}, >1: {over}");
        assert_eq!(admitted(&image, &portable).outside, 0, "nested {model:?}");
    }
    let mut unclipped = n.clipped.clone();
    unclipped.clip = n.label.clip;
    unclipped.visible = n.label.visible;
    let wrong = admitted(
        &n.canvas(&background, &unclipped, Model::Exact).image(),
        &portable,
    );
    eprintln!("nested: label without its ancestor clip rejected by {wrong:?}");
    assert!(wrong.outside > 50 && wrong.excess > 30., "{wrong:?}");
    // The hugged label: the halo past its text clip is part of the contract,
    // so a halo cut at the text clip is rejected, as are the other mistakes.
    let (input, size) = hugged();
    let backdrop = self::background(size);
    let compose = |halo, model| {
        let full = [0., 0., f64::from(size[0]), f64::from(size[1])];
        let mut canvas = Canvas::new(&backdrop, size, true, full, input.viewport, model);
        canvas.halo = halo;
        canvas.glyphs(&input.atlas, &input.widgets[0], theme::TEXT);
        canvas
    };
    let portable = compose(Halo::Contract, Model::Portable);
    for model in [Model::Exact, Model::Llvmpipe, Model::ObservedM5] {
        let image = compose(Halo::Contract, model).image();
        assert_eq!(admitted(&image, &portable).outside, 0, "hugged {model:?}");
    }
    for halo in [Halo::Absent, Halo::Undilated, Halo::Over, Halo::ContentClip] {
        let wrong = admitted(&compose(halo, Model::Exact).image(), &portable);
        eprintln!("hugged label: halo {halo:?} rejected by {wrong:?}");
        assert!(
            wrong.outside > 20 && wrong.excess > 8.,
            "{halo:?} {wrong:?}"
        );
    }
}

#[test]
fn measured_linear_conversion_preserves_the_isolated_dark_channel() {
    // Retained 0069 2x pixel (64,110): one halo fragment over this background.
    // Either destination or encode quantization alone produces blue 11;
    // their combination produces the observed 10. No geometry error is used.
    let background = [192, 38, 10, 255];
    let coverage = 0.162_009_803_921_568_63;
    let compose = |model, range| {
        let mut canvas = Canvas::new(&background, [1, 1], true, [0., 0., 1., 1.], [1., 1.], model);
        canvas.over(0, 0, theme::LABEL_HALO, range);
        canvas
    };
    let exact = compose(Model::Exact, [coverage; 2]);
    let measured = compose(Model::ObservedM5, [coverage; 2]);
    let portable = compose(Model::Portable, [coverage - TAP, coverage + TAP]);
    assert_eq!(exact.image(), [177, 35, 11, 255]);
    assert_eq!(measured.image(), [177, 35, 10, 255]);
    assert_eq!(portable.pixels[0][2], [10., 12.]);
    assert_eq!(admitted(&measured.image(), &portable).outside, 0);
    // Zero-alpha fragments still discard, including any encode conversion.
    let mut empty = Canvas::new(
        &background,
        [1, 1],
        true,
        [0., 0., 1., 1.],
        [1., 1.],
        Model::Portable,
    );
    empty.over(0, 0, theme::LABEL_HALO, [0.; 2]);
    assert_eq!(empty.image(), background);
}

#[test]
fn reference_admits_measured_linear_conversion_over_black_and_white() {
    let (input, size) = hugged();
    for value in [0, 255] {
        let background: Vec<u8> = (0..size[0] * size[1])
            .flat_map(|_| [value, value, value, 255])
            .collect();
        let compose = |model, halo, text| {
            let full = [0., 0., f64::from(size[0]), f64::from(size[1])];
            let mut canvas = Canvas::new(&background, size, true, full, input.viewport, model);
            canvas.halo = halo;
            canvas.glyphs(&input.atlas, &input.widgets[0], text);
            canvas
        };
        let portable = compose(Model::Portable, Halo::Contract, theme::TEXT);
        for model in [Model::Exact, Model::Llvmpipe, Model::ObservedM5] {
            let candidate = compose(model, Halo::Contract, theme::TEXT).image();
            assert_eq!(
                admitted(&candidate, &portable).outside,
                0,
                "background {value}, {model:?}"
            );
        }
        for (halo, text) in [
            (Halo::Absent, theme::TEXT),
            (Halo::Contract, theme::alpha(theme::TEXT, 0.9)),
        ] {
            let candidate = compose(Model::ObservedM5, halo, text).image();
            let wrong = admitted(&candidate, &portable);
            assert!(
                wrong.outside > 100 && wrong.excess > 5.,
                "background {value}, {halo:?}: {wrong:?}"
            );
        }
    }
}
