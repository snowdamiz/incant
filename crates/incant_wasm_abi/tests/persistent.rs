use incant_wasm_abi::persistent::*;
#[test]
fn opt_in_metadata_has_fixed_version_size_and_nonzero_schema() {
    for len in [1, MAX_BYTES as u32] {
        let value = Declaration::new(len, [3; 32]).unwrap();
        assert_eq!(Declaration::decode(&value.encode()), Some(value));
    }
    for len in [0, MAX_BYTES as u32 + 1, u32::MAX] {
        assert!(Declaration::new(len, [1; 32]).is_none());
    }
    assert!(Declaration::new(16, [0; 32]).is_none());
    let declaration = Declaration::new(16, [1; 32]).unwrap();
    let mut section = vec![15];
    section.extend(SECTION.as_bytes());
    section.extend(declaration.encode());
    let mut module = b"\0asm\x01\0\0\0".to_vec();
    assert_eq!(declaration_in_module(&module), Ok(None));
    module.extend([0, section.len() as u8]);
    module.extend(&section);
    assert_eq!(declaration_in_module(&module), Ok(Some(declaration)));
    for end in 9..module.len() {
        assert!(declaration_in_module(&module[..end]).is_err());
    }
    let mut duplicate = module.clone();
    duplicate.extend(&module[8..]);
    assert!(declaration_in_module(&duplicate).is_err());
    *module.last_mut().unwrap() = 1;
    assert!(declaration_in_module(&module).is_err());
    assert!(declaration_in_module(b"\0asm\x01\0\0\0\0\xff\xff\xff\xff\x10").is_err());
}
