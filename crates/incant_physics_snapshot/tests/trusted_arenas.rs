//! One-way observations only. No live Arena/Coarena is deserialized from bytes.
use incant_physics_snapshot::*;
use serde_json::Value;

fn budget() -> Budget {
    Budget::new(MAX_ALLOCATION).unwrap()
}
fn word(value: &Value) -> u32 {
    value.as_u64().unwrap().try_into().unwrap()
}
fn optional(value: &Value) -> Option<u32> {
    (!value.is_null()).then(|| word(value))
}

fn observe(
    a: Value,
    c: Value,
    dimension: Dimension,
) -> (ValidatedArena, CoarenaRecord, Vec<u32>, Vec<u32>) {
    let identity = Identity {
        dimension,
        build: [31; 32],
    };
    let mut payload = Vec::new();
    let slots = a["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| {
            if let Some(value) = item.get("Free") {
                ArenaSlot::Free {
                    next: optional(&value["next_free"]),
                }
            } else {
                let value = &item["Occupied"];
                let row = payload.len() as u32;
                payload.push(word(&value["value"]));
                ArenaSlot::Occupied {
                    generation: word(&value["generation"]),
                    row,
                }
            }
        })
        .collect();
    let record = ArenaRecord {
        identity,
        generation: word(&a["generation"]),
        free_head: optional(&a["free_list_head"]),
        live: word(&a["len"]),
        slots,
    };
    let checked = validate_arena(record.clone(), &mut budget()).unwrap();
    let bytes = encode_arena(&checked, &mut budget()).unwrap();
    let loaded = decode_arena(&bytes, identity, &mut budget()).unwrap();
    assert_eq!(loaded.record(), &record);
    assert_eq!(encode_arena(&loaded, &mut budget()).unwrap(), bytes);
    let mut copayload = Vec::new();
    let rows = c["data"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(index, item)| {
            copayload.push(word(&item[1]));
            CoarenaRow {
                generation: word(&item[0]),
                row: index as u32,
            }
        })
        .collect();
    let co = CoarenaRecord { identity, rows };
    let checked_co = validate_coarena(co.clone(), &loaded, &mut budget()).unwrap();
    let bytes = encode_coarena(&checked_co, &mut budget()).unwrap();
    let loaded_co = decode_coarena(&bytes, &loaded, &mut budget()).unwrap();
    assert_eq!(loaded_co.record(), &co);
    assert_eq!(encode_coarena(&loaded_co, &mut budget()).unwrap(), bytes);
    (loaded, co, payload, copayload)
}

macro_rules! cases {
    ($module:ident,$rapier:ident,$dimension:ident) => {
        mod $module {
            use super::*;
            use $rapier::data::{Arena, Coarena, Index};
            fn check(arena: &Arena<u32>, co: &Coarena<u32>) {
                let (layout, co_record, payload, copayload) = observe(
                    serde_json::to_value(arena).unwrap(),
                    serde_json::to_value(co).unwrap(),
                    Dimension::$dimension,
                );
                let checked_co = validate_coarena(co_record, &layout, &mut budget()).unwrap();
                for (index, value) in arena.iter() {
                    let (slot, generation) = index.into_raw_parts();
                    let row = layout.lookup(Handle { slot, generation }).unwrap();
                    assert_eq!(payload[row as usize], *value);
                }
                for (slot, item) in checked_co.record().rows.iter().enumerate() {
                    assert_eq!(
                        copayload[item.row as usize],
                        *co.get_unknown_gen(slot as u32).unwrap()
                    );
                    if item.generation != u32::MAX {
                        let index = Index::from_raw_parts(slot as u32, item.generation);
                        assert_eq!(
                            checked_co.state(slot as u32),
                            Some(if arena.get(index).is_some() {
                                RowState::Current
                            } else {
                                RowState::Stale
                            })
                        );
                        let row = checked_co
                            .lookup(Handle {
                                slot: slot as u32,
                                generation: item.generation,
                            })
                            .unwrap();
                        assert_eq!(copayload[row as usize], *co.get(index).unwrap());
                    } else {
                        assert_eq!(checked_co.state(slot as u32), Some(RowState::Invalidated));
                        // Pinned get only compares generations, so MAX matches
                        // an invalidated row. Our admitted-handle boundary is
                        // deliberately stricter; unknown-gen keeps its payload.
                        let sentinel = Index::from_raw_parts(slot as u32, u32::MAX);
                        assert_eq!(co.get(sentinel), co.get_unknown_gen(slot as u32));
                        assert!(co.get(sentinel).is_some());
                        assert_eq!(
                            checked_co.lookup(Handle {
                                slot: slot as u32,
                                generation: u32::MAX,
                            }),
                            None
                        );
                        let row = checked_co.unknown_generation_row(slot as u32).unwrap();
                        assert_eq!(copayload[row as usize], *co.get(sentinel).unwrap());
                    }
                }
            }
            #[test]
            fn exact_pinned_arena_and_coarena_removal_reuse_reserve_and_clear() {
                let mut arena = Arena::with_capacity(4);
                let mut co = Coarena::new();
                check(&arena, &co);
                let mut handles = Vec::new();
                for i in 0..4 {
                    let h = arena.insert(i + 10);
                    handles.push(h);
                    if i != 1 {
                        co.insert(h, i + 100);
                    }
                }
                assert!(arena.try_insert(99).is_err());
                check(&arena, &co);
                arena.reserve(3);
                check(&arena, &co);
                // A removed owner can leave a meaningful stale Coarena payload.
                assert_eq!(arena.remove(handles[2]), Some(12));
                check(&arena, &co);
                // Invalidated payloads are supplied by the caller, not always zero.
                assert_eq!(co.remove(handles[0], 999), Some(100));
                assert_eq!(arena.remove(handles[0]), Some(10));
                check(&arena, &co);
                let first = arena.insert(200);
                assert_eq!(first.into_raw_parts(), (0, 2));
                check(&arena, &co);
                let second = arena.insert(201);
                assert_eq!(second.into_raw_parts(), (2, 2));
                check(&arena, &co);
                co.ensure_element_exist(second, 777);
                check(&arena, &co);
                for value in 300..335 {
                    let h = arena.insert(value);
                    co.insert(h, value + 1000);
                }
                check(&arena, &co);
                arena.retain(|h, _| h.into_raw_parts().0 % 3 == 0);
                check(&arena, &co);
                // clear intentionally does not advance Rapier's global generation.
                // The inert representation preserves this exact existing behavior.
                let before = word(&serde_json::to_value(&arena).unwrap()["generation"]);
                arena.clear();
                assert_eq!(
                    word(&serde_json::to_value(&arena).unwrap()["generation"]),
                    before
                );
                check(&arena, &co);
                let h = arena.insert(444);
                assert_eq!(h.into_raw_parts().1, before);
                check(&arena, &co);
            }
        }
    };
}
cases!(planar, rapier2d, Planar);
cases!(spatial, rapier3d, Spatial);
