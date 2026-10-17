use incant_module::persistent::*;
#[test]
fn scalar_codec_is_little_endian_checked_and_rejects_nonfinite_values() {
    let mut bytes = [0; 16];
    write_u64(&mut bytes, 1, 0x0102030405060708).unwrap();
    assert_eq!(&bytes[1..9], &[8, 7, 6, 5, 4, 3, 2, 1]);
    assert_eq!(read_u64(&bytes, 1).unwrap(), 0x0102030405060708);
    write_f64(&mut bytes, 8, 1.25).unwrap();
    assert_eq!(read_f64(&bytes, 8).unwrap(), 1.25);
    for offset in [9, usize::MAX] {
        assert!(read_u64(&bytes, offset).is_err());
        assert!(write_u64(&mut bytes, offset, 1).is_err());
    }
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let before = bytes;
        assert!(write_f64(&mut bytes, 0, value).is_err());
        assert_eq!(bytes, before);
        bytes[..8].copy_from_slice(&value.to_bits().to_le_bytes());
        assert!(read_f64(&bytes, 0).is_err());
    }
}
