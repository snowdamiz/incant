//! Bounded host envelope; checksum is integrity, not source trust. Only the
//! Windows private store may supply bytes to the native library constructor.
use sha2::{Digest, Sha256};
pub(super) const MAX_BYTES: usize = 64 * 1024 * 1024;
pub(super) const HEADER: usize = 8 + 32 + 8 + 32;
const MAGIC: &[u8; 8] = b"INCDXPL1";

pub(super) fn identity(fields: &[&str]) -> [u8; 32] {
    let mut digest = Sha256::new();
    for field in std::iter::once("incant-dx12-library-v1").chain(fields.iter().copied()) {
        digest.update((field.len() as u64).to_le_bytes());
        digest.update(field.as_bytes());
    }
    digest.finalize().into()
}
pub(super) fn encode(identity: &[u8; 32], bytes: &[u8]) -> Option<Vec<u8>> {
    if bytes.is_empty() || bytes.len() > MAX_BYTES {
        return None;
    }
    Some(
        [
            MAGIC.as_slice(),
            identity,
            &(bytes.len() as u64).to_le_bytes(),
            &Sha256::digest(bytes),
            bytes,
        ]
        .concat(),
    )
}
pub(super) fn decode<'a>(identity: &[u8; 32], bytes: &'a [u8]) -> Option<&'a [u8]> {
    if bytes.len() < HEADER || &bytes[..8] != MAGIC || &bytes[8..40] != identity {
        return None;
    }
    let count = u64::from_le_bytes(bytes[40..48].try_into().ok()?);
    if count == 0
        || count > MAX_BYTES as u64
        || bytes.len() != HEADER.checked_add(count as usize)?
    {
        return None;
    }
    let payload = &bytes[HEADER..];
    (bytes[48..HEADER] == Sha256::digest(payload)[..]).then_some(payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_source_adapter_driver_and_field_boundaries_select_identity() {
        let first = identity(&["source", "adapter", "driver", "wgpu29"]);
        for fields in [
            ["source2", "adapter", "driver", "wgpu29"],
            ["source", "adapter2", "driver", "wgpu29"],
            ["source", "adapter", "driver2", "wgpu29"],
        ] {
            assert_ne!(first, identity(&fields));
        }
        assert_ne!(identity(&["ab", "c"]), identity(&["a", "bc"]));
    }
    #[test]
    fn bounded_envelope_refuses_wrong_identity_mutation_truncation_and_declared_overflow() {
        let id = identity(&["source", "adapter", "driver"]);
        let bytes = encode(&id, b"driver-produced bytes").unwrap();
        assert_eq!(
            decode(&id, &bytes),
            Some(b"driver-produced bytes".as_slice())
        );
        assert!(decode(&[0; 32], &bytes).is_none());
        for index in [0, 8, 40, 48, bytes.len() - 1] {
            let mut bad = bytes.clone();
            bad[index] ^= 1;
            assert!(decode(&id, &bad).is_none());
        }
        assert!(decode(&id, &bytes[..bytes.len() - 1]).is_none());
        let mut overflow = bytes.clone();
        overflow[40..48].copy_from_slice(&u64::MAX.to_le_bytes());
        assert!(decode(&id, &overflow).is_none());
        assert!(encode(&id, &[]).is_none());
    }
}
