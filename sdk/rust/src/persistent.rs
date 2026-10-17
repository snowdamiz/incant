//! Explicit host-owned bytes. Guest globals and heap are ephemeral on load.
pub use incant_wasm_abi::persistent::{Declaration, MAX_BYTES};

/// Declare one immutable persistent store. The host admits this data section
/// before startup; capture never calls guest code. Use a new nonzero schema ID
/// when changing the byte layout. No heap object or Rust struct is serialized.
#[macro_export]
macro_rules! persistent_state {
    (schema = $schema:expr, bytes = $bytes:expr) => {
        const _: () = {
            const DECLARATION: $crate::persistent::Declaration =
                match $crate::persistent::Declaration::new($bytes, $schema) {
                    Some(value) => value,
                    None => panic!("invalid persistent state declaration"),
                };
            #[cfg(target_arch = "wasm32")]
            #[used]
            #[unsafe(link_section = "incant_state_v1")]
            static INCANT_PERSISTENT_DECLARATION: [u8; 56] = DECLARATION.encode();
        };
    };
}

/// Typed values must be decoded, not transmuted from untrusted saved bytes.
pub fn read_u64(bytes: &[u8], offset: usize) -> crate::Result<u64> {
    let end = offset.checked_add(8).ok_or(crate::Error::Bounds)?;
    Ok(u64::from_le_bytes(
        bytes
            .get(offset..end)
            .ok_or(crate::Error::Bounds)?
            .try_into()
            .map_err(|_| crate::Error::Bounds)?,
    ))
}
pub fn write_u64(bytes: &mut [u8], offset: usize, value: u64) -> crate::Result<()> {
    let end = offset.checked_add(8).ok_or(crate::Error::Bounds)?;
    bytes
        .get_mut(offset..end)
        .ok_or(crate::Error::Bounds)?
        .copy_from_slice(&value.to_le_bytes());
    Ok(())
}
pub fn read_f64(bytes: &[u8], offset: usize) -> crate::Result<f64> {
    let value = f64::from_bits(read_u64(bytes, offset)?);
    if value.is_finite() {
        Ok(value)
    } else {
        Err(crate::Error::Malformed)
    }
}
pub fn write_f64(bytes: &mut [u8], offset: usize, value: f64) -> crate::Result<()> {
    if !value.is_finite() {
        return Err(crate::Error::Malformed);
    }
    write_u64(bytes, offset, value.to_bits())
}
