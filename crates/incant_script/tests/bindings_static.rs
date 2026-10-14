use incant_script::{
    EmbeddedScriptInstance as Instance, EmbeddedScriptValue as Value, MAX_SCRIPT_INSTANCES,
    MAX_SCRIPT_SOURCES, ScriptBindings, ScriptError, ScriptInstance, ScriptValue,
};
use incant_types::StableId;
use std::collections::BTreeMap;

fn id(value: u128) -> StableId {
    StableId(value.to_be_bytes())
}

fn materialize(props: &[(&str, Value<'_>)]) -> Result<ScriptBindings, ScriptError> {
    ScriptBindings::from_embedded(
        [31; 32],
        &[id(2)],
        &[Instance {
            entity: id(1),
            source: id(2),
            props,
        }],
    )
}

#[test]
fn all_property_kinds_preserve_owned_catalog_and_exact_float_bits() {
    let props = [
        ("", Value::Null),
        (
            "__proto__",
            Value::Object(&[("constructor", Value::Bool(true))]),
        ),
        ("array", Value::Array(&[Value::Bool(false), Value::Null])),
        ("empty-array", Value::Array(&[])),
        ("empty-object", Value::Object(&[])),
        ("float", Value::NumberBits(f64::MAX.to_bits())),
        ("negative-zero", Value::NumberBits((-0.0_f64).to_bits())),
        ("positive-zero", Value::NumberBits(0)),
        ("smallest", Value::NumberBits(1)),
        (
            "text",
            Value::String("quote\" slash\\ nul\0 newline\n Unicode🦀"),
        ),
    ];
    let owned = BTreeMap::from([
        (String::new(), ScriptValue::Null),
        (
            "__proto__".into(),
            ScriptValue::Object(BTreeMap::from([(
                "constructor".into(),
                ScriptValue::Bool(true),
            )])),
        ),
        (
            "array".into(),
            ScriptValue::Array(vec![ScriptValue::Bool(false), ScriptValue::Null]),
        ),
        ("empty-array".into(), ScriptValue::Array(vec![])),
        ("empty-object".into(), ScriptValue::Object(BTreeMap::new())),
        ("float".into(), ScriptValue::Number(f64::MAX)),
        ("negative-zero".into(), ScriptValue::Number(-0.0)),
        ("positive-zero".into(), ScriptValue::Number(0.0)),
        ("smallest".into(), ScriptValue::Number(f64::from_bits(1))),
        (
            "text".into(),
            ScriptValue::String("quote\" slash\\ nul\0 newline\n Unicode🦀".into()),
        ),
    ]);
    let expected = ScriptBindings::new(
        [31; 32],
        vec![id(2)],
        vec![ScriptInstance {
            entity: id(1),
            source: id(2),
            props: owned,
        }],
    )
    .unwrap();
    let actual = materialize(&props).unwrap();
    assert_eq!(actual.sources(), expected.sources());
    assert_eq!(actual.instances(), expected.instances());
    // The fingerprint includes raw f64 bits, unlike float PartialEq.
    assert_eq!(actual.fingerprint(), expected.fingerprint());
    for (name, bits) in [
        ("negative-zero", 1_u64 << 63),
        ("positive-zero", 0),
        ("smallest", 1),
    ] {
        let ScriptValue::Number(value) = actual.instances()[0].props[name] else {
            panic!("number lost")
        };
        assert_eq!(value.to_bits(), bits);
    }
}

#[test]
fn ten_thousand_rows_match_original_constructor_after_canonical_ordering() {
    let props: Vec<_> = (0..10_000)
        .map(|n| [("ordinal", Value::NumberBits((n as f64).to_bits()))])
        .collect();
    let borrowed: Vec<_> = props
        .iter()
        .enumerate()
        .rev()
        .map(|(n, props)| Instance {
            entity: id(n as u128),
            source: id(40_000 + (n % 2) as u128),
            props,
        })
        .collect();
    let owned: Vec<_> = (0..10_000)
        .map(|n| ScriptInstance {
            entity: id(n),
            source: id(40_000 + n % 2),
            props: BTreeMap::from([("ordinal".into(), ScriptValue::Number(n as f64))]),
        })
        .collect();
    let sources = [id(40_001), id(40_000)];
    let expected = ScriptBindings::new([77; 32], sources.to_vec(), owned).unwrap();
    let actual = ScriptBindings::from_embedded([77; 32], &sources, &borrowed).unwrap();
    assert_eq!(actual.sources(), expected.sources());
    assert_eq!(actual.instances(), expected.instances());
    assert_eq!(actual.fingerprint(), expected.fingerprint());
}

#[test]
fn borrowed_shape_and_identity_refusals_do_not_collapse_invalid_data() {
    for props in [
        vec![("a", Value::Null), ("a", Value::Bool(false))],
        vec![("b", Value::Null), ("a", Value::Null)],
        vec![(
            "nested",
            Value::Object(&[("x", Value::Null), ("x", Value::Null)]),
        )],
    ] {
        assert!(matches!(
            materialize(&props),
            Err(ScriptError::ScriptBindings)
        ));
    }
    for number in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(matches!(
            materialize(&[("number", Value::NumberBits(number.to_bits()))]),
            Err(ScriptError::ScriptBindings)
        ));
    }
    let row = Instance {
        entity: id(1),
        source: id(2),
        props: &[],
    };
    for sources in [vec![id(2), id(2)], vec![id(3)], vec![id(2), id(3)]] {
        assert!(matches!(
            ScriptBindings::from_embedded([0; 32], &sources, &[row]),
            Err(ScriptError::ScriptBindings)
        ));
    }
    assert!(matches!(
        ScriptBindings::from_embedded([0; 32], &[id(2)], &[row, row]),
        Err(ScriptError::ScriptBindings)
    ));
}

#[test]
fn borrowed_counts_and_aggregate_property_limits_match_existing_limits() {
    let row = Instance {
        entity: id(1),
        source: id(2),
        props: &[],
    };
    for (sources, rows) in [
        (vec![], vec![row]),
        (vec![id(2)], vec![]),
        (vec![id(2); MAX_SCRIPT_SOURCES + 1], vec![row]),
        (vec![id(2)], vec![row; MAX_SCRIPT_INSTANCES + 1]),
    ] {
        assert!(matches!(
            ScriptBindings::from_embedded([0; 32], &sources, &rows),
            Err(ScriptError::InputLimit)
        ));
    }
    let large_text = "x".repeat(1024 * 1024);
    assert!(matches!(
        materialize(&[("x", Value::String(&large_text))]),
        Err(ScriptError::InputLimit)
    ));
    let many_nodes = vec![Value::Null; 65_536];
    assert!(matches!(
        materialize(&[("x", Value::Array(&many_nodes))]),
        Err(ScriptError::InputLimit)
    ));
    // Each row is valid on its own; the shared budget must reject their sum.
    let text = "x".repeat(600_000);
    let props = [("x", Value::String(&text))];
    assert!(materialize(&props).is_ok());
    let rows = [
        Instance {
            props: &props,
            ..row
        },
        Instance {
            entity: id(3),
            props: &props,
            ..row
        },
    ];
    assert!(matches!(
        ScriptBindings::from_embedded([0; 32], &[id(2)], &rows),
        Err(ScriptError::InputLimit)
    ));
}

fn nested(depth: usize, borrowed: Value<'_>, owned: ScriptValue) {
    if depth > 0 {
        let values = [borrowed];
        nested(
            depth - 1,
            Value::Array(&values),
            ScriptValue::Array(vec![owned]),
        );
    } else {
        let a = materialize(&[("value", borrowed)]);
        let b = ScriptBindings::new(
            [31; 32],
            vec![id(2)],
            vec![ScriptInstance {
                entity: id(1),
                source: id(2),
                props: BTreeMap::from([("value".into(), owned)]),
            }],
        );
        match (a, b) {
            (Ok(a), Ok(b)) => assert_eq!(a.fingerprint(), b.fingerprint()),
            (Err(ScriptError::InputLimit), Err(ScriptError::InputLimit)) => (),
            _ => panic!("borrowed and owned depth budgets differ"),
        }
    }
}

#[test]
fn borrowed_nested_arrays_and_objects_obey_the_owned_depth_boundary() {
    for depth in [0, 14, 15, 16, 17] {
        nested(depth, Value::Null, ScriptValue::Null);
        nested(
            depth,
            Value::Object(&[("key", Value::Null)]),
            ScriptValue::Object(BTreeMap::from([("key".into(), ScriptValue::Null)])),
        );
    }
}

#[cfg(feature = "compiler")]
#[test]
fn borrowed_initial_props_reach_the_real_host_as_frozen_data() {
    use incant_runtime::{CookedEntity, CookedScene, NativeWorld};
    use incant_script::{CompiledScript, ScriptHost};
    use incant_types::Transform;
    use sha2::{Digest, Sha256};

    let scene = CookedScene::new(
        id(90),
        60,
        vec![CookedEntity {
            id: id(1),
            parent: None,
            transform: Transform::default(),
            velocity: None,
        }],
    )
    .unwrap();
    let bindings = ScriptBindings::from_embedded(
        Sha256::digest(scene.to_bytes()).into(),
        &[id(2)],
        &[Instance {
            entity: id(1),
            source: id(2),
            props: &[
                ("amount", Value::NumberBits(3.5_f64.to_bits())),
                ("negativeZero", Value::NumberBits((-0.0_f64).to_bits())),
            ],
        }],
    )
    .unwrap();
    let script = format!(
        "exports.default={{sources:[{{id:{:?},behavior:{{update(ctx){{const p=ctx.instances[0].props;if(!Object.isFrozen(p)||!Object.is(p.negativeZero,-0))throw Error('props changed');for(const c of ctx.chunks)c.transforms[0]+=p.amount;}}}}}}]}};",
        ulid::Ulid::from(2_u128).to_string()
    );
    let compiled =
        CompiledScript::compile_project("borrowed-props.cjs", &script, bindings).unwrap();
    let mut world = NativeWorld::new(scene);
    let mut host = ScriptHost::for_world(compiled, &world).unwrap();
    host.tick(&mut world, 1.0 / 60.0).unwrap();
    host.tick(&mut world, 1.0 / 60.0).unwrap();
    assert_eq!(world.inspect(id(1)).unwrap().transform.translation[0], 7.0);
}
