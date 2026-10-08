//! Equivalent 10,000-entity field microbenchmark; excludes disk/network and schema
//! validation so CRDT storage/merge costs can be compared directly.
use automerge::{AutoCommit, ObjType, ROOT, ReadDoc, transaction::Transactable};
use loro::{ExportMode, LoroDoc, ToJson};
use serde_json::json;
use std::time::Instant;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let count = 10_000;
    let start = Instant::now();
    let loro = LoroDoc::new();
    let entities = loro.get_map("entities");
    for index in 0..count {
        let id = format!("entity-{index}");
        let e = entities.ensure_mergeable_map(&id)?;
        e.insert("id", id.clone())?;
        e.insert("name", format!("Entity {index}"))?;
        e.insert("x", index as f64)?;
        e.insert("y", 0.0)?;
        e.insert("z", 0.0)?;
    }
    loro.commit();
    let init_loro = start.elapsed();
    let start = Instant::now();
    let right = loro.fork();
    entities
        .ensure_mergeable_map("entity-5000")?
        .insert("name", "Renamed")?;
    right
        .get_map("entities")
        .ensure_mergeable_map("entity-5000")?
        .insert("x", 123.0)?;
    loro.commit();
    right.commit();
    let a = loro.export(ExportMode::Snapshot)?;
    let b = right.export(ExportMode::Snapshot)?;
    loro.import(&b)?;
    right.import(&a)?;
    let merge_loro = start.elapsed();
    let left_value = loro.get_deep_value().to_json_value();
    assert_eq!(left_value, right.get_deep_value().to_json_value());
    assert_eq!(left_value["entities"]["entity-5000"]["name"], "Renamed");
    assert_eq!(left_value["entities"]["entity-5000"]["x"], 123.);
    let start = Instant::now();
    let mut auto = AutoCommit::new();
    let entities = auto.put_object(ROOT, "entities", ObjType::Map)?;
    for index in 0..count {
        let id = format!("entity-{index}");
        let e = auto.put_object(&entities, id.clone(), ObjType::Map)?;
        auto.put(&e, "id", id)?;
        auto.put(&e, "name", format!("Entity {index}"))?;
        auto.put(&e, "x", index as f64)?;
        auto.put(&e, "y", 0.0)?;
        auto.put(&e, "z", 0.0)?;
    }
    auto.commit();
    let init_auto = start.elapsed();
    let start = Instant::now();
    let mut right = auto.fork();
    let (_, entity) = auto
        .get(&entities, "entity-5000")?
        .ok_or("missing entity")?;
    auto.put(&entity, "name", "Renamed")?;
    right.put(&entity, "x", 123.0)?;
    auto.commit();
    right.commit();
    let aa = auto.save();
    let ab = right.save();
    let mut ra = AutoCommit::load(&aa)?;
    let mut rb = AutoCommit::load(&ab)?;
    auto.merge(&mut rb)?;
    right.merge(&mut ra)?;
    let merge_auto = start.elapsed();
    assert_eq!(auto.get(&entity, "name")?, right.get(&entity, "name")?);
    assert_eq!(auto.get(&entity, "x")?, right.get(&entity, "x")?);
    println!(
        "{}",
        serde_json::to_string_pretty(
            &json!({"entities":count,"fixture":"five scalar fields per entity, two concurrent field edits, full snapshot exchange","loro":{"version":"1.16.2","initial_ms":init_loro.as_secs_f64()*1000.,"fork_edit_export_merge_ms":merge_loro.as_secs_f64()*1000.,"snapshot_bytes":a.len()},"automerge":{"version":"0.12.0","initial_ms":init_auto.as_secs_f64()*1000.,"fork_edit_export_merge_ms":merge_auto.as_secs_f64()*1000.,"snapshot_bytes":aa.len()}})
        )?
    );
    Ok(())
}
