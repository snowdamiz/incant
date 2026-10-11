use super::*;
struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8]> {
        if n > self.0.len() {
            return Err(invalid("truncated animation section"));
        }
        let (a, b) = self.0.split_at(n);
        self.0 = b;
        Ok(a)
    }
    fn byte(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn index(&mut self) -> Result<usize> {
        Ok(self.u32()? as usize)
    }
    fn optional(&mut self) -> Result<Option<usize>> {
        let n = self.u32()?;
        Ok((n != u32::MAX).then_some(n as usize))
    }
    fn count(&mut self, max: usize, stride: usize) -> Result<usize> {
        let n = self.index()?;
        cap(n, max, "animation collection")?;
        if n > self.0.len() / stride {
            return Err(invalid("truncated animation collection"));
        }
        Ok(n)
    }
    fn floats<const N: usize>(&mut self) -> Result<[f32; N]> {
        let mut v = [0.; N];
        for f in &mut v {
            *f = f32::from_le_bytes(self.take(4)?.try_into().unwrap());
            if !f.is_finite() {
                return Err(invalid("nonfinite animation scalar"));
            }
        }
        Ok(v)
    }
    fn name(&mut self) -> Result<Option<String>> {
        let n = self.u32()?;
        if n == u32::MAX {
            return Ok(None);
        }
        cap(n as usize, MAX_NAME_BYTES, "animation name")?;
        Ok(Some(
            std::str::from_utf8(self.take(n as usize)?)
                .map_err(|_| invalid("animation name UTF-8"))?
                .to_owned(),
        ))
    }
}
/// Decode the typed suffix of INCMOD04. Counts/remaining bytes/aggregate budgets
/// are checked before each allocation; containing model validates references.
pub fn decode_animation(bytes: &[u8]) -> Result<AnimationData> {
    cap(bytes.len(), MAX_ANIMATION_BYTES, "animation bytes")?;
    let mut r = Reader(bytes);
    if r.take(8)? != b"INCANM01" {
        return Err(invalid("animation version"));
    }
    let n = r.count(MAX_NODES, 1)?;
    let mut rest = Vec::with_capacity(n);
    for _ in 0..n {
        rest.push(match r.byte()? {
            0 => None,
            1 => Some(NodeTrs {
                translation: r.floats()?,
                rotation: r.floats()?,
                scale: r.floats()?,
            }),
            _ => return Err(invalid("rest transform tag")),
        });
    }
    let n = r.count(MAX_NODES, 4)?;
    let mut node_skins = Vec::with_capacity(n);
    for _ in 0..n {
        node_skins.push(r.optional()?);
    }
    let n = r.count(MAX_SKINS, 12)?;
    let mut skins = Vec::with_capacity(n);
    let mut total = 0;
    for _ in 0..n {
        let name = r.name()?;
        let skeleton = r.optional()?;
        let n = r.count(MAX_JOINTS_PER_SKIN, 68)?;
        total += n;
        cap(total, MAX_JOINTS, "total joints")?;
        let mut joints = Vec::with_capacity(n);
        let mut inverse_bind = Vec::with_capacity(n);
        for _ in 0..n {
            joints.push(r.index()?);
            inverse_bind.push([r.floats()?, r.floats()?, r.floats()?, r.floats()?]);
        }
        skins.push(Skin {
            name,
            skeleton,
            joints,
            inverse_bind,
        });
    }
    let n = r.count(10_000, 4)?;
    let mut influences = Vec::with_capacity(n);
    let mut total = 0;
    for _ in 0..n {
        let n = r.count(crate::MAX_VERTICES, 24)?;
        total += n;
        cap(total, crate::MAX_VERTICES, "total influences")?;
        let mut stream = Vec::with_capacity(n);
        for _ in 0..n {
            let mut joints = [0; 4];
            for j in &mut joints {
                *j = u16::from_le_bytes(r.take(2)?.try_into().unwrap());
            }
            stream.push(Influences {
                joints,
                weights: r.floats()?,
            });
        }
        influences.push(stream);
    }
    let n = r.count(MAX_CLIPS, 8)?;
    let mut clips = Vec::with_capacity(n);
    let (mut channels, mut keys) = (0, 0);
    for _ in 0..n {
        let name = r.name()?;
        let n = r.count(MAX_CHANNELS, 10)?;
        channels += n;
        cap(channels, MAX_CHANNELS, "total channels")?;
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            let node = r.index()?;
            let property = match r.byte()? {
                0 => Property::Translation,
                1 => Property::Rotation,
                2 => Property::Scale,
                _ => return Err(invalid("animation property tag")),
            };
            let interpolation = match r.byte()? {
                0 => Interpolation::Linear,
                1 => Interpolation::Step,
                _ => return Err(invalid("animation interpolation tag")),
            };
            let n = r.count(MAX_KEYS, 20)?;
            keys += n;
            cap(keys, MAX_KEYS, "total keys")?;
            let mut times = Vec::with_capacity(n);
            let mut values = Vec::with_capacity(n);
            for _ in 0..n {
                times.push(r.floats::<1>()?[0]);
                values.push(r.floats()?);
            }
            out.push(Channel {
                node,
                property,
                interpolation,
                times,
                values,
            });
        }
        clips.push(Clip {
            name,
            channels: out,
        });
    }
    if !r.0.is_empty() {
        return Err(invalid("trailing animation bytes"));
    }
    Ok(AnimationData {
        rest,
        skins,
        node_skins,
        influences,
        clips,
    })
}
