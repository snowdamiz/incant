//! Monochrome, unhinted default-instance TrueType coverage. Same pinned Zeno
//! rasterizer family as cosmic-text/Swash; no ambient fonts or color fallback.
mod glyf;
use crate::TextError;
use rustybuzz::ttf_parser::{Face, GlyphId, OutlineBuilder};
use zeno::{Command, Mask, Point};
pub(crate) const MAX_GLYPH_SIDE: u32 = 1024;
const MAX_COMMANDS: usize = glyf::MAX_POINTS * 2 + 1024;
#[derive(Default)]
pub(crate) struct Raster {
    commands: Vec<Command>,
    pixels: Vec<u8>,
}
pub(crate) struct Image<'a> {
    pub pixels: &'a [u8],
    pub width: u32,
    pub height: u32,
    pub origin: [i32; 2],
}
impl Raster {
    pub fn image(&mut self, bytes: &[u8], glyph: u16, size: f32) -> Result<Image<'_>, TextError> {
        let face = Face::parse(bytes, 0).map_err(|_| TextError::Invalid("invalid raster face"))?;
        if !glyf::preflight(&face, glyph)? {
            return Ok(Image {
                pixels: &[],
                width: 0,
                height: 0,
                origin: [0; 2],
            });
        }
        self.commands.clear();
        // Bounded once; repeated misses reuse this storage. Preflight bounds the
        // parser work before collecting anything; the sink independently checks.
        if self.commands.capacity() < MAX_COMMANDS {
            self.commands.reserve_exact(MAX_COMMANDS);
        }
        let mut sink = Sink {
            commands: &mut self.commands,
            scale: size / f32::from(face.units_per_em()),
            failed: false,
        };
        // Explicit base/default outline, not variable-axis or hint execution.
        let result = face
            .tables()
            .glyf
            .ok_or(TextError::Invalid("missing glyf table"))?
            .outline(GlyphId(glyph), &mut sink);
        if sink.failed || result.is_none() {
            return Err(TextError::Invalid("invalid or excessive glyph outline"));
        }
        let mut mask = Mask::new(self.commands.as_slice());
        let mut dimensions = [0; 2];
        mask.inspect(|_, width, height| dimensions = [width, height]);
        let [width, height] = dimensions;
        if width > MAX_GLYPH_SIDE || height > MAX_GLYPH_SIDE {
            return Err(TextError::Limit("glyph dimensions"));
        }
        let bytes = (width as usize)
            .checked_mul(height as usize)
            .ok_or(TextError::Limit("glyph mask"))?;
        self.pixels.clear();
        if self.pixels.capacity() < bytes {
            self.pixels.reserve_exact(bytes);
        }
        self.pixels.resize(bytes, 0);
        let placement = mask.render_into(&mut self.pixels, None);
        Ok(Image {
            pixels: &self.pixels,
            width,
            height,
            origin: [placement.left, placement.top],
        })
    }
}
struct Sink<'a> {
    commands: &'a mut Vec<Command>,
    scale: f32,
    failed: bool,
}
impl Sink<'_> {
    fn point(&mut self, x: f32, y: f32) -> Point {
        let (x, y) = (x * self.scale, -y * self.scale); // baseline, Y down
        if !x.is_finite() || !y.is_finite() || x.abs() > 65536. || y.abs() > 65536. {
            self.failed = true;
        }
        Point::new(x, y)
    }
    fn push(&mut self, command: Command) {
        if self.commands.len() == MAX_COMMANDS {
            self.failed = true;
        }
        if !self.failed {
            self.commands.push(command);
        }
    }
}
impl OutlineBuilder for Sink<'_> {
    fn move_to(&mut self, x: f32, y: f32) {
        let p = self.point(x, y);
        self.push(Command::MoveTo(p));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        let p = self.point(x, y);
        self.push(Command::LineTo(p));
    }
    fn quad_to(&mut self, x: f32, y: f32, a: f32, b: f32) {
        let c = self.point(x, y);
        let p = self.point(a, b);
        self.push(Command::QuadTo(c, p));
    }
    fn curve_to(&mut self, x: f32, y: f32, a: f32, b: f32, c: f32, d: f32) {
        let c1 = self.point(x, y);
        let c2 = self.point(a, b);
        let p = self.point(c, d);
        self.push(Command::CurveTo(c1, c2, p));
    }
    fn close(&mut self) {
        self.push(Command::Close);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const FONT: &[u8] = include_bytes!("../tests/fonts/NotoSans-Regular.ttf");
    fn table(bytes: &[u8], name: &[u8; 4]) -> usize {
        let count = u16::from_be_bytes(bytes[4..6].try_into().unwrap()) as usize;
        let row = bytes[12..12 + count * 16]
            .as_chunks::<16>()
            .0
            .iter()
            .find(|r| &r[..4] == name)
            .unwrap();
        u32::from_be_bytes(row[8..12].try_into().unwrap()) as usize
    }
    fn glyph() -> u16 {
        Face::parse(FONT, 0).unwrap().glyph_index('A').unwrap().0
    }
    fn offset(bytes: &[u8], glyph: u16) -> usize {
        let loca = table(bytes, b"loca");
        let head = table(bytes, b"head");
        let offset = if u16::from_be_bytes(bytes[head + 50..head + 52].try_into().unwrap()) == 0 {
            let at = loca + usize::from(glyph) * 2;
            usize::from(u16::from_be_bytes(bytes[at..at + 2].try_into().unwrap())) * 2
        } else {
            let at = loca + usize::from(glyph) * 4;
            u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap()) as usize
        };
        table(bytes, b"glyf") + offset
    }
    #[test]
    fn cyclic_overlarge_and_truncated_glyphs_refuse_before_outline_or_mask_allocation() {
        for case in 0..6 {
            let mut bytes = FONT.to_vec();
            let at = offset(&bytes, glyph());
            match case {
                0 => {
                    bytes[at..at + 2].copy_from_slice(&(-1_i16).to_be_bytes());
                    bytes[at + 10..at + 12].copy_from_slice(&3_u16.to_be_bytes());
                    bytes[at + 12..at + 14].copy_from_slice(&glyph().to_be_bytes());
                }
                1 => {
                    bytes[at..at + 2].copy_from_slice(&1_u16.to_be_bytes());
                    bytes[at + 10..at + 12].copy_from_slice(&65534_u16.to_be_bytes());
                }
                2 => {
                    bytes[at..at + 2].copy_from_slice(&1_u16.to_be_bytes());
                    bytes[at + 10..at + 12].copy_from_slice(&2_u16.to_be_bytes());
                    bytes[at + 12..at + 14].copy_from_slice(&65535_u16.to_be_bytes());
                }
                3 => {
                    bytes[at..at + 2].copy_from_slice(&(-1_i16).to_be_bytes());
                    bytes[at + 10..at + 12].copy_from_slice(&(3_u16 | 8 | 64).to_be_bytes());
                }
                _ => {
                    bytes[at..at + 2].copy_from_slice(&(-1_i16).to_be_bytes());
                    let flags = 3_u16 | if case == 4 { 0x0800 } else { 0x2000 };
                    bytes[at + 10..at + 12].copy_from_slice(&flags.to_be_bytes());
                }
            }
            let mut raster = Raster::default();
            assert!(raster.image(&bytes, glyph(), 20.).is_err(), "case {case}");
            assert_eq!(raster.commands.capacity(), 0);
            assert_eq!(raster.pixels.capacity(), 0);
        }
        let mut raster = Raster::default();
        assert!(raster.image(FONT, u16::MAX, 20.).is_err());
        assert_eq!(raster.commands.capacity(), 0);
    }
    #[test]
    fn malicious_finite_dimensions_refuse_before_mask_allocation() {
        let mut bytes = FONT.to_vec();
        let head = table(&bytes, b"head");
        bytes[head + 18..head + 20].copy_from_slice(&16_u16.to_be_bytes());
        let mut raster = Raster::default();
        assert!(matches!(
            raster.image(&bytes, glyph(), 512.),
            Err(TextError::Limit("glyph dimensions"))
        ));
        assert_eq!(raster.pixels.capacity(), 0);
    }
    #[test]
    fn base_outline_coverage_agrees_with_independent_area_and_y_down_bounds() {
        // Integrate line and quadratic Bezier contour area analytically,
        // independently of rasterizer scan conversion, then compare coverage.
        #[derive(Default)]
        struct Area {
            first: [f64; 2],
            last: [f64; 2],
            area: f64,
            min: [f32; 2],
            max: [f32; 2],
        }
        impl OutlineBuilder for Area {
            fn move_to(&mut self, x: f32, y: f32) {
                self.first = [x as f64, y as f64];
                self.last = self.first;
            }
            fn line_to(&mut self, x: f32, y: f32) {
                let p = [x as f64, y as f64];
                self.area += self.last[0] * p[1] - p[0] * self.last[1];
                self.last = p;
                self.min[0] = self.min[0].min(x);
                self.min[1] = self.min[1].min(y);
                self.max[0] = self.max[0].max(x);
                self.max[1] = self.max[1].max(y);
            }
            fn close(&mut self) {
                self.area += self.last[0] * self.first[1] - self.first[0] * self.last[1];
            }
            fn quad_to(&mut self, x: f32, y: f32, a: f32, b: f32) {
                let c = [f64::from(x), f64::from(y)];
                let end = [f64::from(a), f64::from(b)];
                let px = [
                    self.last[0],
                    2. * (c[0] - self.last[0]),
                    self.last[0] - 2. * c[0] + end[0],
                ];
                let py = [
                    self.last[1],
                    2. * (c[1] - self.last[1]),
                    self.last[1] - 2. * c[1] + end[1],
                ];
                for i in 0..3 {
                    for j in 1..3 {
                        self.area += (px[i] * py[j] - py[i] * px[j]) * j as f64 / (i + j) as f64;
                    }
                }
                self.last = end;
            }
            fn curve_to(&mut self, _: f32, _: f32, _: f32, _: f32, _: f32, _: f32) {
                panic!("TrueType oracle expects lines and quadratic curves")
            }
        }
        let face = Face::parse(FONT, 0).unwrap();
        let mut area = Area {
            min: [f32::INFINITY; 2],
            max: [f32::NEG_INFINITY; 2],
            ..Default::default()
        };
        face.tables()
            .glyf
            .unwrap()
            .outline(GlyphId(glyph()), &mut area)
            .unwrap();
        let size = 64.;
        let scale = f64::from(size) / f64::from(face.units_per_em());
        let expected = area.area.abs() * 0.5 * scale * scale;
        let mut raster = Raster::default();
        let image = raster.image(FONT, glyph(), size).unwrap();
        let covered = image
            .pixels
            .iter()
            .map(|&p| f64::from(p) / 255.)
            .sum::<f64>();
        assert!(
            (covered - expected).abs() / expected < 0.015,
            "{covered} vs {expected}"
        );
        assert_eq!(
            image.origin[1],
            (-f64::from(area.max[1]) * scale).floor() as i32
        );
        assert!(image.origin[1] < 0);
        assert_eq!(
            image.width,
            ((f64::from(area.max[0]) * scale).ceil() - (f64::from(area.min[0]) * scale).floor())
                as u32
        );
    }
}
