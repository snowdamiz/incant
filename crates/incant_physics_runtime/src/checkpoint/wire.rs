//! Dimension-local wire primitives: explicit little endian and cumulative budgets.
//! No serde, layout casts, normalization, or implicit allocation from input counts.
use super::{Budget, CheckpointError, RecordError};
use incant_types::{BodyMotion, RigidBody, StableId};
pub(super) const MAX_BYTES: usize = 32 * 1024 * 1024;
pub(super) type Result<T> = std::result::Result<T, CheckpointError>;
fn bad() -> CheckpointError {
    CheckpointError::Owner("persisted physics framing")
}
pub(super) struct Writer {
    bytes: Option<Vec<u8>>,
    length: usize,
}
impl Writer {
    pub fn raw(&mut self, bytes: &[u8]) -> Result<()> {
        self.length = self
            .length
            .checked_add(bytes.len())
            .filter(|n| *n <= MAX_BYTES)
            .ok_or_else(bad)?;
        if let Some(output) = &mut self.bytes {
            if self.length > output.capacity() {
                return Err(bad());
            }
            output.extend_from_slice(bytes);
        }
        Ok(())
    }
    pub fn byte(&mut self, value: u8) -> Result<()> {
        self.raw(&[value])
    }
    pub fn boolean(&mut self, value: bool) -> Result<()> {
        self.byte(u8::from(value))
    }
    pub fn u32(&mut self, value: u32) -> Result<()> {
        self.raw(&value.to_le_bytes())
    }
    pub fn count(&mut self, value: usize) -> Result<()> {
        self.u32(value.try_into().map_err(|_| bad())?)
    }
    pub fn scalar(&mut self, value: f64) -> Result<()> {
        self.raw(&value.to_le_bytes())
    }
    pub fn floats<const N: usize>(&mut self, value: &[f64; N]) -> Result<()> {
        for &v in value {
            self.scalar(v)?;
        }
        Ok(())
    }
    pub fn id(&mut self, value: StableId) -> Result<()> {
        self.raw(&value.0)
    }
    pub fn handle(&mut self, value: (u32, u32)) -> Result<()> {
        self.u32(value.0)?;
        self.u32(value.1)
    }
    pub fn blob(&mut self, bytes: &[u8]) -> Result<()> {
        self.count(bytes.len())?;
        self.raw(bytes)
    }
    pub fn body(&mut self, value: &Option<RigidBody>) -> Result<()> {
        let Some(value) = value else {
            return self.byte(0);
        };
        self.byte(match value.motion {
            BodyMotion::Fixed => 1,
            BodyMotion::Dynamic => 2,
            BodyMotion::Kinematic => 3,
        })?;
        self.scalar(value.gravity_scale)?;
        self.scalar(value.linear_damping)?;
        self.scalar(value.angular_damping)?;
        self.boolean(value.can_sleep)?;
        self.boolean(value.ccd)
    }
}
// The callback is private, deterministic and only borrows already bounded rows.
// Size first; the second pass cannot grow the reserved allocation.
pub(super) fn encode(
    budget: &mut Budget,
    mut emit: impl FnMut(&mut Writer) -> Result<()>,
) -> Result<Vec<u8>> {
    let mut sizing = Writer {
        bytes: None,
        length: 0,
    };
    emit(&mut sizing)?;
    let bytes = budget.vector(sizing.length)?;
    let mut writing = Writer {
        bytes: Some(bytes),
        length: 0,
    };
    emit(&mut writing)?;
    if writing.length != sizing.length {
        return Err(bad());
    }
    Ok(writing.bytes.unwrap())
}
pub(super) struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}
impl<'a> Reader<'a> {
    pub fn new(bytes: &'a [u8], magic: &[u8; 8]) -> Result<Self> {
        if bytes.len() > MAX_BYTES {
            return Err(bad());
        }
        let mut result = Self { bytes, offset: 0 };
        if result.take(8)? != magic {
            return Err(bad());
        }
        Ok(result)
    }
    pub fn take(&mut self, count: usize) -> Result<&'a [u8]> {
        let end = self
            .offset
            .checked_add(count)
            .filter(|n| *n <= self.bytes.len())
            .ok_or_else(bad)?;
        let result = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(result)
    }
    pub fn byte(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    pub fn boolean(&mut self) -> Result<bool> {
        match self.byte()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(bad()),
        }
    }
    pub fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    pub fn count(&mut self, max: usize, minimum_stride: usize) -> Result<usize> {
        let count = self.u32()? as usize;
        if count > max
            || count
                .checked_mul(minimum_stride)
                .is_none_or(|n| n > self.bytes.len() - self.offset)
        {
            return Err(bad());
        }
        Ok(count)
    }
    pub fn scalar(&mut self) -> Result<f64> {
        let value = f64::from_le_bytes(self.take(8)?.try_into().unwrap());
        if !value.is_finite() {
            return Err(bad());
        }
        Ok(value)
    }
    pub fn floats<const N: usize>(&mut self) -> Result<[f64; N]> {
        let mut result = [0.0; N];
        for value in &mut result {
            *value = self.scalar()?;
        }
        Ok(result)
    }
    pub fn id(&mut self) -> Result<StableId> {
        Ok(StableId(self.take(16)?.try_into().unwrap()))
    }
    pub fn handle(&mut self) -> Result<(u32, u32)> {
        Ok((self.u32()?, self.u32()?))
    }
    pub fn blob(&mut self) -> Result<&'a [u8]> {
        let n = self.count(MAX_BYTES, 1)?;
        self.take(n)
    }
    pub fn body(&mut self) -> Result<Option<RigidBody>> {
        let motion = match self.byte()? {
            0 => return Ok(None),
            1 => BodyMotion::Fixed,
            2 => BodyMotion::Dynamic,
            3 => BodyMotion::Kinematic,
            _ => return Err(bad()),
        };
        Ok(Some(RigidBody {
            motion,
            gravity_scale: self.scalar()?,
            linear_damping: self.scalar()?,
            angular_damping: self.scalar()?,
            can_sleep: self.boolean()?,
            ccd: self.boolean()?,
        }))
    }
    pub fn finish(self) -> Result<()> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(bad())
        }
    }
    pub fn text26(&mut self, budget: &mut Budget) -> Result<String> {
        let text = std::str::from_utf8(self.take(26)?).map_err(|_| bad())?;
        budget.charge(26)?;
        let mut result = String::new();
        result
            .try_reserve_exact(26)
            .map_err(|_| RecordError::Allocation)?;
        result.push_str(text);
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const MAGIC: &[u8; 8] = b"WIRETEST";
    #[test]
    fn exact_bits_and_cumulative_output_budget() {
        let mut budget = Budget::new(64).unwrap();
        let bytes = encode(&mut budget, |out| {
            out.raw(MAGIC)?;
            out.scalar(-0.0)?;
            out.boolean(true)
        })
        .unwrap();
        assert_eq!(budget.remaining(), 47);
        let mut input = Reader::new(&bytes, MAGIC).unwrap();
        assert_eq!(input.scalar().unwrap().to_bits(), (-0.0f64).to_bits());
        assert!(input.boolean().unwrap());
        input.finish().unwrap();
        assert!(
            encode(&mut Budget::new(16).unwrap(), |out| {
                out.raw(MAGIC)?;
                out.scalar(0.0)?;
                out.byte(1)
            })
            .is_err()
        );
    }
    #[test]
    fn malformed_counts_scalars_flags_and_trailing_bytes_refuse() {
        let make = |data: &[u8]| {
            let mut v = MAGIC.to_vec();
            v.extend(data);
            v
        };
        assert!(
            Reader::new(&make(&u32::MAX.to_le_bytes()), MAGIC)
                .unwrap()
                .count(1024, 8)
                .is_err()
        );
        assert!(
            Reader::new(&make(&2u32.to_le_bytes()), MAGIC)
                .unwrap()
                .count(2, 8)
                .is_err()
        );
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(
                Reader::new(&make(&value.to_le_bytes()), MAGIC)
                    .unwrap()
                    .scalar()
                    .is_err()
            );
        }
        assert!(Reader::new(&make(&[2]), MAGIC).unwrap().boolean().is_err());
        assert!(Reader::new(&make(&[4]), MAGIC).unwrap().body().is_err());
        assert!(Reader::new(&make(&[0]), MAGIC).unwrap().finish().is_err());
        for n in 0..8 {
            assert!(Reader::new(&MAGIC[..n], MAGIC).is_err());
        }
    }
}
