use incant_script::{ScriptBindings, ScriptInstance, ScriptValue as V};
use incant_types::StableId;
use std::collections::BTreeMap;
#[test]
fn original_canonical_ts_fingerprint_is_unchanged_for_all_property_tags() {
    let props = BTreeMap::from([
        ("array".into(), V::Array(vec![V::Null, V::Bool(true)])),
        ("negative_zero".into(), V::Number(-0.)),
        (
            "object".into(),
            V::Object(BTreeMap::from([("key".into(), V::String("語".into()))])),
        ),
        ("speed".into(), V::Number(10.)),
    ]);
    let bindings = ScriptBindings::new(
        [7; 32],
        vec![StableId(101u128.to_be_bytes())],
        vec![ScriptInstance {
            entity: StableId(1u128.to_be_bytes()),
            source: StableId(101u128.to_be_bytes()),
            props,
        }],
    )
    .unwrap();
    // Computed from the preserved main409 prefix/tags/lengths/f64 bits.
    assert_eq!(
        bindings.fingerprint(),
        [
            103, 22, 15, 56, 174, 27, 48, 42, 106, 10, 54, 94, 122, 173, 214, 164, 103, 176, 254,
            217, 186, 124, 161, 217, 89, 145, 7, 220, 227, 27, 16, 29
        ]
    );
}

#[test]
fn original_owned_property_byte_node_and_depth_boundaries_are_preserved() {
    fn admitted(value: V) -> bool {
        ScriptBindings::new(
            [0; 32],
            vec![StableId([2; 16])],
            vec![ScriptInstance {
                entity: StableId([1; 16]),
                source: StableId([2; 16]),
                props: BTreeMap::from([("x".into(), value)]),
            }],
        )
        .is_ok()
    }
    // Original budget: object 8 + key (8+1) + value 1 + string (8+len).
    assert!(admitted(V::String("x".repeat(1_048_550))));
    assert!(!admitted(V::String("x".repeat(1_048_551))));
    // Object + key + array occupy three nodes before their null elements.
    assert!(admitted(V::Array(vec![V::Null; 65_533])));
    assert!(!admitted(V::Array(vec![V::Null; 65_534])));
    assert!(admitted((0..15).fold(V::Null, |v, _| V::Array(vec![v]))));
    assert!(!admitted((0..16).fold(V::Null, |v, _| V::Array(vec![v]))));
}
