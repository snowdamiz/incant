//! Admission before the pinned outline parser: bounded expanded points, visits
//! and depth. No allocation or execution of font hinting/variation programs.
use crate::TextError;
use rustybuzz::ttf_parser::{Face, Tag};
const MAX_VISITS: usize = 1024;
pub(super) const MAX_POINTS: usize = 8192;
fn invalid() -> TextError {
    TextError::Invalid("malformed TrueType outline")
}
fn u16_at(data: &[u8], offset: usize) -> Result<u16, TextError> {
    Ok(u16::from_be_bytes(
        data.get(offset..offset + 2)
            .ok_or_else(invalid)?
            .try_into()
            .unwrap(),
    ))
}
pub(super) fn preflight(face: &Face<'_>, glyph: u16) -> Result<bool, TextError> {
    let raw = face.raw_face();
    for tag in [
        b"COLR", b"CPAL", b"CBDT", b"CBLC", b"sbix", b"SVG ", b"EBDT", b"EBLC",
    ] {
        if raw.table(Tag::from_bytes(tag)).is_some() {
            return Err(TextError::Invalid(
                "color/bitmap glyph rasterization is unsupported",
            ));
        }
    }
    let glyf = raw
        .table(Tag::from_bytes(b"glyf"))
        .ok_or(TextError::Invalid(
            "rasterization requires TrueType glyf outlines",
        ))?;
    let loca = raw.table(Tag::from_bytes(b"loca")).ok_or_else(invalid)?;
    let head = raw.table(Tag::from_bytes(b"head")).ok_or_else(invalid)?;
    let long = match u16_at(head, 50)? {
        0 => false,
        1 => true,
        _ => return Err(invalid()),
    };
    let mut check = Check {
        glyf,
        loca,
        long,
        glyphs: face.number_of_glyphs(),
        visits: 0,
        points: 0,
        stack: [0; 32],
    };
    check.visit(glyph, 0)?;
    Ok(check.points != 0)
}
struct Check<'a> {
    glyf: &'a [u8],
    loca: &'a [u8],
    long: bool,
    glyphs: u16,
    visits: usize,
    points: usize,
    stack: [u16; 32],
}
impl Check<'_> {
    fn offset(&self, index: usize) -> Result<usize, TextError> {
        if self.long {
            let at = index * 4;
            Ok(u32::from_be_bytes(
                self.loca
                    .get(at..at + 4)
                    .ok_or_else(invalid)?
                    .try_into()
                    .unwrap(),
            ) as usize)
        } else {
            Ok(usize::from(u16_at(self.loca, index * 2)?) * 2)
        }
    }
    fn visit(&mut self, id: u16, depth: usize) -> Result<(), TextError> {
        self.visits += 1;
        if self.visits > MAX_VISITS || depth >= self.stack.len() {
            return Err(TextError::Limit("glyph expansion"));
        }
        if id >= self.glyphs || self.stack[..depth].contains(&id) {
            return Err(invalid());
        }
        self.stack[depth] = id;
        let begin = self.offset(usize::from(id))?;
        let end = self.offset(usize::from(id) + 1)?;
        let data = self.glyf.get(begin..end).ok_or_else(invalid)?;
        if data.is_empty() {
            return Ok(());
        }
        data.get(..10).ok_or_else(invalid)?;
        let contours = u16_at(data, 0)? as i16;
        if contours >= 0 {
            if contours == 0 {
                return Ok(());
            }
            let count = usize::from(contours as u16);
            if count > MAX_POINTS {
                return Err(TextError::Limit("glyph contours"));
            }
            let mut previous = None;
            for i in 0..count {
                let end = usize::from(u16_at(data, 10 + i * 2)?);
                if previous.is_some_and(|p| end <= p) {
                    return Err(invalid());
                }
                previous = Some(end);
            }
            let points = previous.ok_or_else(invalid)? + 1;
            self.points = self
                .points
                .checked_add(points)
                .filter(|&n| n <= MAX_POINTS)
                .ok_or(TextError::Limit("glyph points"))?;
            let mut at = 10 + count * 2;
            let instructions = usize::from(u16_at(data, at)?);
            at += 2 + instructions;
            data.get(..at).ok_or_else(invalid)?;
            let mut seen = 0;
            let mut coordinate_bytes = 0;
            while seen < points {
                let flag = *data.get(at).ok_or_else(invalid)?;
                if flag & 0x80 != 0 {
                    return Err(invalid());
                }
                at += 1;
                let repeat = if flag & 8 != 0 {
                    let n = usize::from(*data.get(at).ok_or_else(invalid)?) + 1;
                    at += 1;
                    n
                } else {
                    1
                };
                seen += repeat;
                if seen > points {
                    return Err(invalid());
                }
                coordinate_bytes += repeat
                    * (if flag & 2 != 0 {
                        1
                    } else if flag & 16 == 0 {
                        2
                    } else {
                        0
                    });
                coordinate_bytes += repeat
                    * (if flag & 4 != 0 {
                        1
                    } else if flag & 32 == 0 {
                        2
                    } else {
                        0
                    });
            }
            data.get(at..at + coordinate_bytes).ok_or_else(invalid)?;
        } else {
            if contours != -1 {
                return Err(invalid());
            }
            let mut at = 10;
            let mut instructions = false;
            loop {
                let flags = u16_at(data, at)?;
                let child = u16_at(data, at + 2)?;
                if flags & 0xe010 != 0 {
                    return Err(invalid());
                }
                // The pinned outline parser applies unscaled component offsets.
                // ROUND_XY_TO_GRID is intentionally ignored by this unhinted path.
                if flags & 0x0800 != 0 {
                    return Err(TextError::Invalid("scaled compound offset unsupported"));
                }
                instructions |= flags & 256 != 0;
                // The pinned parser does not implement point-to-point attachment.
                if flags & 2 == 0 {
                    return Err(TextError::Invalid(
                        "point-matched compound glyph unsupported",
                    ));
                }
                let transforms = [8, 64, 128]
                    .into_iter()
                    .filter(|bit| flags & bit != 0)
                    .count();
                if transforms > 1 {
                    return Err(invalid());
                }
                at += 4 + if flags & 1 != 0 { 4 } else { 2 };
                at += if flags & 8 != 0 {
                    2
                } else if flags & 64 != 0 {
                    4
                } else if flags & 128 != 0 {
                    8
                } else {
                    0
                };
                data.get(..at).ok_or_else(invalid)?;
                self.visit(child, depth + 1)?;
                if flags & 32 == 0 {
                    if instructions {
                        let n = usize::from(u16_at(data, at)?);
                        data.get(at + 2..at + 2 + n).ok_or_else(invalid)?;
                    }
                    break;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repeated_empty_components_and_deep_chains_have_finite_expansion_budgets() {
        // A compact DAG may expand exponentially; count every expanded visit,
        // including empty children, before allocating any outline storage.
        let mut glyf = vec![0; 10];
        glyf[..2].copy_from_slice(&(-1_i16).to_be_bytes());
        for i in 0..MAX_VISITS {
            let flags = 2_u16 | if i + 1 < MAX_VISITS { 32 } else { 0 };
            glyf.extend(flags.to_be_bytes());
            glyf.extend(1_u16.to_be_bytes());
            glyf.extend([0; 2]);
        }
        let end = glyf.len() as u32;
        let loca: Vec<_> = [0_u32, end, end]
            .into_iter()
            .flat_map(u32::to_be_bytes)
            .collect();
        let mut check = Check {
            glyf: &glyf,
            loca: &loca,
            long: true,
            glyphs: 2,
            visits: 0,
            points: 0,
            stack: [0; 32],
        };
        assert!(matches!(
            check.visit(0, 0),
            Err(TextError::Limit("glyph expansion"))
        ));
        assert_eq!(check.visits, MAX_VISITS + 1);

        let mut glyf = Vec::new();
        let mut offsets = vec![0_u32];
        for id in 0..33_u16 {
            let mut header = [0; 10];
            if id < 32 {
                header[..2].copy_from_slice(&(-1_i16).to_be_bytes());
            }
            glyf.extend(header);
            if id < 32 {
                glyf.extend(2_u16.to_be_bytes());
                glyf.extend((id + 1).to_be_bytes());
                glyf.extend([0; 2]);
            }
            offsets.push(glyf.len() as u32);
        }
        let loca: Vec<_> = offsets.into_iter().flat_map(u32::to_be_bytes).collect();
        let mut check = Check {
            glyf: &glyf,
            loca: &loca,
            long: true,
            glyphs: 33,
            visits: 0,
            points: 0,
            stack: [0; 32],
        };
        assert!(matches!(
            check.visit(0, 0),
            Err(TextError::Limit("glyph expansion"))
        ));
        assert_eq!(check.visits, 33);
    }
}
