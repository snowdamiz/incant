//! Private envelope: fixed header plus unchanged ICWORLD1 payload. All sizing
//! precedes access/allocation. This adds no authenticity or public save format.
use super::super::allocation::*;
use incant_physics_snapshot::MAX_BYTES;
const HEADER: usize = 16;
const MAGIC: [u8; 8] = *b"ICNATIV1";
pub(super) struct AssemblyFrame<'a> {
    inner: &'a [u8],
    impulse_epoch: u32,
}
impl<'a> AssemblyFrame<'a> {
    pub(super) fn decode(bytes: &'a [u8]) -> ConstructionResult<Self> {
        if bytes.len() < HEADER || bytes.len() > MAX_BYTES + HEADER || bytes[..8] != MAGIC {
            return Err(ConstructionError::Payload);
        }
        let len = u32::from_le_bytes(
            bytes[8..12]
                .try_into()
                .map_err(|_| ConstructionError::Payload)?,
        ) as usize;
        if len > MAX_BYTES || len.checked_add(HEADER) != Some(bytes.len()) {
            return Err(ConstructionError::Payload);
        }
        let impulse_epoch = u32::from_le_bytes(
            bytes[12..16]
                .try_into()
                .map_err(|_| ConstructionError::Payload)?,
        );
        Ok(Self {
            inner: &bytes[HEADER..],
            impulse_epoch,
        })
    }
    pub(super) fn inner(&self) -> &'a [u8] {
        self.inner
    }
    pub(super) fn impulse_epoch(&self) -> u32 {
        self.impulse_epoch
    }
    // Trusted test capture only. Live field inspection precedes encoding, and
    // the entire envelope (not a detached caller epoch) is subsequently decoded.
    #[cfg(test)]
    pub(super) fn encode(
        inner: &[u8],
        epoch: u32,
        budget: &mut AllocationBudget,
    ) -> ConstructionResult<alloc::vec::Vec<u8>> {
        let len = inner
            .len()
            .checked_add(HEADER)
            .filter(|n| *n <= MAX_BYTES + HEADER)
            .ok_or(ConstructionError::Limit)?;
        let mut p = budget.admit(AllocationPlan::array::<u8>(len)?)?;
        let mut out = p.vector(len)?;
        out.extend_from_slice(&MAGIC);
        out.extend_from_slice(&(inner.len() as u32).to_le_bytes());
        out.extend_from_slice(&epoch.to_le_bytes());
        out.extend_from_slice(inner);
        p.finish()?;
        Ok(out)
    }
}
