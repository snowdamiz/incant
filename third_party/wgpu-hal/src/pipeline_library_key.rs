//! Canonical framing shared by native descriptor keys and portable CPU controls.
//! Native callers supply semantic fields, never raw C structs, labels or pointers.
use sha2::{Digest, Sha256};

pub struct Encoder(Sha256);
impl Encoder {
    pub fn new(kind: &[u8]) -> Self {
        let mut key = Self(Sha256::new());
        key.bytes(b"incant-dx12-native-descriptor-v1");
        key.bytes(kind);
        key
    }
    pub fn bytes(&mut self, bytes: &[u8]) {
        self.0.update((bytes.len() as u64).to_le_bytes());
        self.0.update(bytes);
    }
    pub fn u32(&mut self, value: u32) {
        self.0.update(value.to_le_bytes());
    }
    pub fn u64(&mut self, value: u64) {
        self.0.update(value.to_le_bytes());
    }
    pub fn boolean(&mut self, value: bool) {
        self.u32(u32::from(value));
    }
    pub fn float(&mut self, value: f32) {
        self.u32(value.to_bits());
    }
    pub fn finish(self) -> [u8; 32] {
        self.0.finalize().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_fields_are_length_delimited_and_domain_separated() {
        let key = |kind: &[u8], a: &[u8], b: &[u8]| {
            let mut key = Encoder::new(kind);
            key.bytes(a);
            key.bytes(b);
            key.finish()
        };
        assert_ne!(key(b"render", b"ab", b"c"), key(b"render", b"a", b"bc"));
        assert_ne!(key(b"render", b"a", b"b"), key(b"compute", b"a", b"b"));
        assert_eq!(key(b"render", b"a", b"b"), key(b"render", b"a", b"b"));
    }

    #[test]
    fn scalar_bits_and_order_do_not_collapse() {
        let key = |a: f32, b: u32| {
            let mut key = Encoder::new(b"render");
            key.float(a);
            key.u32(b);
            key.finish()
        };
        assert_ne!(key(0., 1), key(-0., 1));
        assert_ne!(key(0., 1), key(0., 2));
        let mut first = Encoder::new(b"render");
        first.u64(1);
        first.u32(2);
        let mut second = Encoder::new(b"render");
        second.u32(2);
        second.u64(1);
        assert_ne!(first.finish(), second.finish());
    }
}
