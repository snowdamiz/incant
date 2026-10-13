use crate::{codec::Reader, *};
pub(super) fn decode(
    bytes: &[u8],
    id: Identity,
    budget: &mut Budget,
) -> Result<Vec<ValidatedGeometry>> {
    if bytes.len() < 4 {
        return Err(Error::Length);
    }
    let mut r = Reader { bytes, position: 0 };
    let n = r.word()? as usize;
    if n > MAX_ITEMS {
        return Err(Error::Limit);
    }
    let header = 4 + 4 * n;
    if header > bytes.len() {
        return Err(Error::Length);
    }
    // Preflight every nested length before allocating the geometry collection.
    let mut total = header;
    for _ in 0..n {
        total = total
            .checked_add(r.word()? as usize)
            .filter(|v| *v <= MAX_BYTES)
            .ok_or(Error::Limit)?;
    }
    if total != bytes.len() {
        return Err(Error::Length);
    }
    budget.charge(n * size_of::<ValidatedGeometry>())?;
    let mut out = vector(n)?;
    r.position = 4;
    let mut start = header;
    for _ in 0..n {
        let len = r.word()? as usize;
        out.push(decode_geometry(&bytes[start..start + len], id, budget)?);
        start += len;
    }
    Ok(out)
}
pub(super) fn encode(values: &[ValidatedGeometry], budget: &mut Budget) -> Result<Vec<u8>> {
    let header = 4 + 4 * values.len();
    budget.charge(header)?;
    let mut out = vector(header)?;
    out.resize(header, 0);
    out[..4].copy_from_slice(&(values.len() as u32).to_le_bytes());
    for (i, value) in values.iter().enumerate() {
        let b = encode_geometry(value, budget)?;
        out.len()
            .checked_add(b.len())
            .filter(|v| *v <= MAX_BYTES)
            .ok_or(Error::Limit)?;
        budget.charge(b.len())?;
        out.try_reserve_exact(b.len())
            .map_err(|_| Error::Allocation)?;
        out[4 + 4 * i..8 + 4 * i].copy_from_slice(&(b.len() as u32).to_le_bytes());
        out.extend_from_slice(&b);
    }
    Ok(out)
}
