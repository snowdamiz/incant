//! One-way observation of trusted live Parry values. Never deserialize a live
//! Bvh, VecMap, shape or solver from bytes, including in these tests.
use incant_physics_snapshot::*;
use serde_json::Value;

fn budget() -> Budget {
    Budget::new(MAX_ALLOCATION).unwrap()
}
fn word(value: &Value) -> u32 {
    value.as_u64().unwrap().try_into().unwrap()
}
fn node(value: &Value, dim: Dimension) -> Node {
    let mut mins = [0; 3];
    let mut maxs = [0; 3];
    for i in 0..dim as usize {
        mins[i] = (value["mins"][i].as_f64().unwrap() as f32).to_bits();
        maxs[i] = (value["maxs"][i].as_f64().unwrap() as f32).to_bits();
    }
    Node {
        mins,
        children: word(&value["children"]),
        maxs,
        data: word(&value["data"]),
    }
}
fn observe(value: Value, dimension: Dimension) {
    let nodes = value["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| Wide {
            left: node(&v["left"], dimension),
            right: node(&v["right"], dimension),
        })
        .collect();
    let parents = value["parents"]
        .as_array()
        .unwrap()
        .iter()
        .map(word)
        .collect();
    let slots = value["leaf_node_indices"]["v"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| if v.is_null() { None } else { Some(word(v)) })
        .collect();
    let free = value["free_wide_nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(word)
        .collect();
    let record = BvhRecord {
        identity: Identity {
            dimension,
            build: [17; 32],
        },
        nodes,
        parents,
        leaves: LeafMap {
            occupied: word(&value["leaf_node_indices"]["n"]),
            slots,
        },
        free,
        rebuild_frame: word(&value["optimization"]["rebuild_frame_index"]),
        rebuild_start: word(&value["optimization"]["rebuild_start_index"]),
    };
    let identity = record.identity;
    let expected = record.clone();
    let checked = validate(record, &mut budget()).unwrap();
    let bytes = encode(&checked, &mut budget()).unwrap();
    let loaded = decode(&bytes, identity, &mut budget()).unwrap();
    assert_eq!(loaded.record(), &expected);
    assert_eq!(encode(&loaded, &mut budget()).unwrap(), bytes);
}

macro_rules! cases {
    ($module:ident,$parry:ident,$dimension:ident,$vector:expr) => {
        mod $module {
            use super::*;
            use $parry::{
                bounding_volume::Aabb,
                math::Vector,
                partitioning::{Bvh, BvhWorkspace},
            };
            fn aabb(i: u32) -> Aabb {
                let vector = $vector;
                let x = i as f32 * 0.125;
                Aabb::new(vector(x, 0.), vector(x + 1., 1.))
            }
            #[test]
            fn exact_trusted_tree_observations_across_recycled_slots_and_refits() {
                let mut bvh = Bvh::new();
                let mut workspace = BvhWorkspace::default();
                observe(serde_json::to_value(&bvh).unwrap(), Dimension::$dimension);
                // Sparse inserts include the special one/two-leaf storage cases.
                for i in 0..129 {
                    bvh.insert(aabb(i), i * 3 + 5);
                    if i < 2 {
                        observe(serde_json::to_value(&bvh).unwrap(), Dimension::$dimension);
                    }
                }
                bvh.refit(&mut workspace);
                observe(serde_json::to_value(&bvh).unwrap(), Dimension::$dimension);
                for i in (1..128).step_by(3) {
                    bvh.remove(i * 3 + 5);
                }
                observe(serde_json::to_value(&bvh).unwrap(), Dimension::$dimension);
                for i in (1..128).step_by(3) {
                    bvh.insert(aabb(i + 100), i * 3 + 5);
                }
                bvh.refit(&mut workspace);
                observe(serde_json::to_value(&bvh).unwrap(), Dimension::$dimension);
                for _ in 0..40 {
                    bvh.optimize_incremental(&mut workspace);
                    bvh.refit_without_resolve(&mut workspace);
                    observe(serde_json::to_value(&bvh).unwrap(), Dimension::$dimension);
                }
                for i in 0..128 {
                    bvh.remove(i * 3 + 5);
                }
                observe(serde_json::to_value(&bvh).unwrap(), Dimension::$dimension);
                bvh.remove(128 * 3 + 5);
                observe(serde_json::to_value(&bvh).unwrap(), Dimension::$dimension);
            }
        }
    };
}
cases!(planar, parry2d, Planar, |x, y| Vector::new(x, y));
cases!(spatial, parry3d, Spatial, |x, y| Vector::new(x, y, 0.));
