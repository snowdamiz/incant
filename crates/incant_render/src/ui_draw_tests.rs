//! Actual GPU UI draws compared with an independent CPU composition written
//! from the documented contract: physical mapping, snapping, clips, rounded
//! coverage, bilinear glyph taps and premultiplied over. Ignored without a
//! granted native GPU; numeric agreement, not appearance approval.
use super::tests::Fixture;
use super::*;
use crate::{
    Viewport,
    hdr_output::{read, save, texture},
};
use incant_render_data::{UiEdit, UiRect, UiRole, UiWidget};
use incant_text::{Wrap, editing::Selection};
use std::sync::Weak;

const SRGB: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const UNORM: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

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

/// Reference compositor in the target's blending space: linear light for sRGB
/// targets, encoded values for UNORM targets.
struct Canvas {
    size: [u32; 2],
    srgb: bool,
    /// Physical region origin and per-axis scale.
    region: [f64; 4],
    scale: [f64; 2],
    pixels: Vec<[f64; 3]>,
}
impl Canvas {
    fn new(bytes: &[u8], size: [u32; 2], srgb: bool, region: [f64; 4], viewport: [f32; 2]) -> Self {
        Self {
            size,
            srgb,
            region,
            scale: [
                region[2] / f64::from(viewport[0]),
                region[3] / f64::from(viewport[1]),
            ],
            pixels: bytes
                .as_chunks::<4>()
                .0
                .iter()
                .map(|p| std::array::from_fn(|c| self_space(srgb, f64::from(p[c]))))
                .collect(),
        }
    }
    fn bytes(&self) -> Vec<u8> {
        self.pixels
            .iter()
            .flat_map(|p| {
                let v = |c: f64| {
                    if self.srgb { encode(c) } else { c * 255. }
                        .round()
                        .clamp(0., 255.) as u8
                };
                [v(p[0]), v(p[1]), v(p[2]), 255]
            })
            .collect()
    }
    fn over(&mut self, x: u32, y: u32, color: [u8; 4], coverage: f64) {
        let a = f64::from(color[3]) / 255. * coverage;
        let pixel = &mut self.pixels[(y * self.size[0] + x) as usize];
        for c in 0..3 {
            let src = self_space(self.srgb, f64::from(color[c]));
            pixel[c] = src * a + pixel[c] * (1. - a);
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
            self.over(x, y, color, coverage);
        }
    }
    /// Glyph coverage: snapped pen, padded rectangle at `scale` pixels per
    /// texel, one bilinear tap clamped to the padded texels (scale >= 0.8).
    /// Labels first draw the same coverage as a shadow at the theme offset.
    fn glyphs(&mut self, atlas: &GlyphAtlas, widget: &UiWidget, color: [u8; 4]) {
        if widget.role == UiRole::Label {
            self.glyph_pass(
                atlas,
                widget,
                theme::LABEL_SHADOW,
                theme::LABEL_SHADOW_OFFSET,
            );
        }
        self.glyph_pass(atlas, widget, color, [0.; 2]);
    }
    fn glyph_pass(
        &mut self,
        atlas: &GlyphAtlas,
        widget: &UiWidget,
        color: [u8; 4],
        offset: [f32; 2],
    ) {
        assert!(self.scale.iter().all(|s| *s >= 0.8), "single-tap reference");
        let [aw, _] = atlas.size();
        let texel =
            |x: i64, y: i64| f64::from(atlas.pixels()[(y * i64::from(aw) + x) as usize]) / 255.;
        let bilinear = |u: f64, v: f64| {
            let (x, y) = (u - 0.5, v - 0.5);
            let (i, j) = (x.floor(), y.floor());
            let (fx, fy) = (x - i, y - j);
            let (i, j) = (i as i64, j as i64);
            (texel(i, j) * (1. - fx) + texel(i + 1, j) * fx) * (1. - fy)
                + (texel(i, j + 1) * (1. - fx) + texel(i + 1, j + 1) * fx) * fy
        };
        let c = self.clip(widget.clip);
        for (glyph, handle) in widget.layout.glyphs().iter().zip(widget.glyphs.iter()) {
            let entry = atlas.get(handle).unwrap();
            let Some(rect) = entry.rect else { continue };
            let [x, y, w, h] = rect.map(f64::from);
            let [ox, oy] = entry.origin.map(f64::from);
            let pen = [
                snap(self.physical(
                    f64::from(widget.origin[0] + glyph.position[0] + offset[0]),
                    0,
                )),
                snap(self.physical(
                    f64::from(widget.origin[1] + glyph.position[1] + offset[1]),
                    1,
                )),
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
                self.over(px, py, color, bilinear(u, v));
            }
        }
    }
}
fn self_space(srgb: bool, byte: f64) -> f64 {
    if srgb { decode(byte) } else { byte / 255. }
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
    let target = texture(renderer, format, size[0], size[1]);
    let background = background(size);
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

/// One labelled button, one mixed-script label and one focused field with a
/// selection, at fractional logical positions.
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
        0.,
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
/// `swapped` draws the field's text under its selection (a wrong order).
fn reference_order(input: &UiInput, canvas: &mut Canvas, swapped: bool) {
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
    canvas.glyphs(&input.atlas, button, theme::TEXT);
    canvas.glyphs(&input.atlas, label, theme::TEXT);
    canvas.rect(
        field.bounds,
        field.visible,
        theme::RADIUS,
        None,
        theme::FIELD,
    );
    if swapped {
        canvas.glyphs(&input.atlas, field, theme::TEXT);
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
        canvas.glyphs(&input.atlas, field, theme::TEXT);
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
    reference_order(input, canvas, false);
}

#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn logical_layout_maps_to_physical_pixels_at_integer_and_fractional_scales() {
    let renderer = Renderer::headless().unwrap();
    let input = panel();
    for (scale, format) in [(1., SRGB), (2., SRGB), (1.5, SRGB), (1., UNORM)] {
        let size = [(160. * scale) as u32, (90. * scale) as u32];
        let mut layer = renderer.ui_layer();
        let run = run(&renderer, &mut layer, &input, format, size, None);
        let full = [0., 0., f64::from(size[0]), f64::from(size[1])];
        let mut canvas = Canvas::new(
            &run.background,
            size,
            format.is_srgb(),
            full,
            input.viewport,
        );
        reference(&input, &mut canvas);
        let expected = canvas.bytes();
        let (worst, over) = difference(&run.pixels, &expected);
        let changed = ink(&run.pixels, &run.background);
        eprintln!("scale {scale} {format:?}: worst {worst}, >1: {over}, changed {changed}");
        save(
            &format!("ui-panel-{scale}x-{format:?}"),
            &run.pixels,
            size[0],
            size[1],
        );
        assert!(
            worst <= 2 && over * 1000 <= expected.len(),
            "{scale}: {worst}/{over}"
        );
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

#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn nested_clips_cut_actual_glyph_bounds_including_negative_x_rtl_overflow() {
    let renderer = Renderer::headless().unwrap();
    let mut f = Fixture::new();
    let token = "سلامعليكم";
    let mut label = f.widget(1, UiRole::Label, [40., 10., 80., 40.], 0., token, 20.);
    let (layout, glyphs) = f.shape(token, 20., 40., Wrap::Word);
    label.layout = layout;
    label.glyphs = glyphs;
    assert!(label.layout.glyphs().iter().any(|g| g.position[0] < -10.));
    // An ancestor clip narrower than the content box, as a nested container gives.
    let ancestor: UiRect = [44., 12., 74., 38.];
    let mut clipped = label.clone();
    clipped.clip = ancestor;
    clipped.visible = ancestor;
    // A button partly hidden by the same ancestor.
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
    let atlas = Arc::new(f.atlas);
    let make = |widgets| {
        Arc::new(UiInput {
            viewport: [120., 90.],
            atlas: Arc::clone(&atlas),
            widgets,
        })
    };
    let size = [120, 90];
    let open = make(vec![label.clone()]);
    let nested = make(vec![clipped.clone(), button.clone()]);
    let mut layer = renderer.ui_layer();
    let open_run = run(&renderer, &mut layer, &open, SRGB, size, None);
    let nested_run = run(&renderer, &mut layer, &nested, SRGB, size, None);
    save("ui-clip-open", &open_run.pixels, size[0], size[1]);
    save("ui-clip-nested", &nested_run.pixels, size[0], size[1]);
    let region = [0., 0., 120., 90.];
    let mut canvas = Canvas::new(&nested_run.background, size, true, region, nested.viewport);
    canvas.glyphs(&atlas, &clipped, theme::TEXT);
    canvas.rect(
        button.bounds,
        button.visible,
        theme::RADIUS,
        None,
        theme::BUTTON,
    );
    canvas.glyphs(&atlas, &button, theme::TEXT);
    let (worst, over) = difference(&nested_run.pixels, &canvas.bytes());
    eprintln!("nested clips: worst {worst}, >1: {over}");
    assert!(worst <= 2 && over == 0, "{worst}/{over}");
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

#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn ui_composites_over_the_finished_world_frame_and_leaves_uncovered_pixels() {
    use crate::text::draw_tests::{pixel_camera, scene};
    use glam::{DMat4, DVec3};
    let renderer = Renderer::headless().unwrap();
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
    let full = [0., 0., 160., 90.];
    let mut canvas = Canvas::new(&world_only, size, true, full, input.viewport);
    reference(&input, &mut canvas);
    let (worst, over) = difference(&composed, &canvas.bytes());
    eprintln!("over world: worst {worst}, >1: {over}");
    assert!(
        worst <= 2 && over * 1000 <= composed.len(),
        "{worst}/{over}"
    );
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
    let mut swapped = Canvas::new(&world_only, size, true, full, input.viewport);
    reference_order(&input, &mut swapped, true);
    let (wrong, _) = difference(&composed, &swapped.bytes());
    assert!(wrong > 2, "selection/text order is not observable: {wrong}");
}

#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn retained_atlas_and_quads_follow_input_identity_and_survive_refused_stages() {
    let renderer = Renderer::headless().unwrap();
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
    let other = Renderer::headless().unwrap();
    assert!(other.ui_layer().stage(&renderer, &fresh).is_err());
}

#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn editor_viewport_region_offsets_scales_and_bounds_the_ui() {
    let renderer = Renderer::headless().unwrap();
    let size = [200, 140];
    let region = [20.5, 15., 160., 90.];
    let input = panel();
    let mut layer = renderer.ui_layer();
    let run = run(&renderer, &mut layer, &input, SRGB, size, Some(region));
    save("ui-region", &run.pixels, size[0], size[1]);
    let mut canvas = Canvas::new(
        &run.background,
        size,
        true,
        region.map(f64::from),
        input.viewport,
    );
    reference(&input, &mut canvas);
    let (worst, over) = difference(&run.pixels, &canvas.bytes());
    eprintln!("region: worst {worst}, >1: {over}");
    assert!(
        worst <= 2 && over * 1000 <= run.pixels.len(),
        "{worst}/{over}"
    );
    for py in 0..size[1] {
        for px in 0..size[0] {
            if !(21..181).contains(&px) || !(15..105).contains(&py) {
                let i = ((py * size[0] + px) * 4) as usize;
                assert_eq!(run.pixels[i..i + 4], run.background[i..i + 4], "{px},{py}");
            }
        }
    }
}
