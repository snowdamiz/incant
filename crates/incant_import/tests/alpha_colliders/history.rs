use super::support::{Fixture, entities};
use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::{Origin, Project};
use incant_import::{AlphaColliderError, ImportSnapshot};
use serde_json::json;
use std::fs;

#[test]
fn one_atomic_transaction_preserves_source_ids_provenance_undo_redo_and_saved_reopen() {
    let mut f = Fixture::new(3, &[255, 0, 255, 255, 0, 255], false);
    // Preparation is source-free and creates no authored/history changes.
    fs::remove_file(f.root.path().join("mask.png")).unwrap();
    let before = f.bus.project().clone();
    let revision = f.bus.revision();
    let history = f.bus.history().len();
    let prepared = f.prepare(128).unwrap();
    let ids: Vec<_> = entities(&prepared).iter().map(|e| e.id.clone()).collect();
    let commands = prepared.commands().to_vec();
    assert_eq!(f.bus.project(), &before);
    assert_eq!(f.bus.history().len(), history);
    let result = prepared
        .commit(&mut f.bus, Actor::agent("test-model", "test-conversation"))
        .unwrap();
    assert_eq!(result.revision, revision + 1);
    assert_eq!(f.bus.history().len(), history + 1);
    let transaction = f.bus.history().last().unwrap();
    assert_eq!(transaction.commands, commands);
    let after = f.bus.project().clone();
    assert_eq!(
        after.scenes[&f.scene].entities[&f.entity],
        before.scenes[&f.scene].entities[&f.entity]
    );
    for id in &ids {
        let provenance = after.scenes[&f.scene].entities[id]
            .provenance
            .as_ref()
            .unwrap();
        assert_eq!(provenance.origin, Origin::Agent);
        assert_eq!(provenance.transaction, transaction.id);
        assert_eq!(provenance.model.as_deref(), Some("test-model"));
    }
    f.bus.undo().unwrap();
    assert_eq!(f.bus.project(), &before);
    f.bus.redo().unwrap();
    assert_eq!(f.bus.project(), &after);
    f.reopen();
    assert_eq!(f.bus.project(), &after); // Journal replay before Save.
    f.bus
        .save_project_file(f.bus.revision(), Actor::user("test"))
        .unwrap();
    let saved = Project::from_text(&fs::read_to_string(f.root.path().join("test.incant")).unwrap())
        .unwrap();
    assert_eq!(saved, after);
    f.reopen();
    assert_eq!(f.bus.project(), &after);
    f.bus.undo().unwrap();
    assert_eq!(f.bus.project(), &before);
    f.bus.redo().unwrap();
    assert_eq!(f.bus.project(), &after);
}

#[test]
fn source_asset_revision_and_same_revision_project_swaps_refuse_atomically() {
    for change_asset in [false, true] {
        let mut f = Fixture::new(2, &[255, 0, 0, 255], false);
        let prepared = f.prepare(128).unwrap();
        if change_asset {
            let mut asset = f.bus.project().assets[&f.asset].clone();
            asset.sha256 = "0".repeat(64);
            f.bus
                .execute(
                    vec![Command::UpsertAsset { asset }],
                    Actor::user("test"),
                    "Asset change",
                    None,
                )
                .unwrap();
        } else {
            let mut sprite = f.sprite();
            sprite.flip_x = true;
            f.component("SpriteRenderer", json!(sprite));
        }
        let before = f.bus.project().clone();
        let history = f.bus.history().to_vec();
        let revision = f.bus.revision();
        assert!(matches!(
            prepared.commit(&mut f.bus, Actor::user("test")),
            Err(AlphaColliderError::StaleProject)
        ));
        assert_eq!(f.bus.project(), &before);
        assert_eq!(f.bus.history(), history);
        assert_eq!(f.bus.revision(), revision);
        f.reopen();
        assert_eq!(f.bus.project(), &before);
    }
    let f = Fixture::new(1, &[255], false);
    let original = CommandBus::new(f.bus.project().clone()).unwrap();
    let mut changed = original.project().clone();
    changed.name = "Same ID and revision, different document".into();
    let mut replaced = CommandBus::new(changed.clone()).unwrap();
    let prepared = ImportSnapshot::capture(&original)
        .prepare_alpha_colliders(f.root.path(), None, &f.request(128))
        .unwrap();
    assert_eq!(original.revision(), replaced.revision());
    assert!(matches!(
        prepared.commit(&mut replaced, Actor::user("test")),
        Err(AlphaColliderError::StaleProject)
    ));
    assert_eq!(replaced.project(), &changed);
    assert!(replaced.history().is_empty());
}

#[test]
fn failed_actor_and_later_source_edits_do_not_change_prepared_or_committed_geometry() {
    let mut f = Fixture::new(2, &[255, 0], false);
    let before = f.bus.project().clone();
    let history = f.bus.history().to_vec();
    assert!(matches!(
        f.prepare(128).unwrap().commit(&mut f.bus, Actor::user("")),
        Err(AlphaColliderError::Command(_))
    ));
    assert_eq!(f.bus.project(), &before);
    assert_eq!(f.bus.history(), history);
    let result = f
        .prepare(128)
        .unwrap()
        .commit(&mut f.bus, Actor::user("test"))
        .unwrap();
    let id = &result.derivation.rectangles[0].entity_id;
    let collider = f.bus.project().scenes[&f.scene].entities[id].clone();
    let mut sprite = f.sprite();
    sprite.flip_x = true;
    f.component("SpriteRenderer", json!(sprite));
    assert_eq!(
        f.bus.project().scenes[&f.scene].entities[id],
        collider,
        "authored fixed result has no live linkage"
    );
}
