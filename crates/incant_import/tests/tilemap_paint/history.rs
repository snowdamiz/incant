use super::support::{Fixture, frame};
use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::{Origin, Project};
use incant_import::{ImportSnapshot, TilemapBrush as B, TilemapPaintError as E};
use std::fs;

#[test]
fn source_free_stroke_is_one_provenanced_transaction_with_exact_undo_redo_reopen() {
    let mut f = Fixture::new([2, 2]);
    for path in ["tile.png", "tiles.atlas.json"] {
        fs::remove_file(f.root.path().join(path)).unwrap();
    }
    let before = f.bus.project().clone();
    let rev = f.bus.revision();
    let history = f.bus.history().len();
    let p = f
        .prepare(&f.request(vec![[1, 1], [0, 0]], B::Paint { frame: frame(4) }))
        .unwrap();
    assert_eq!(f.bus.project(), &before);
    assert_eq!(p.commands().len(), 1);
    let commands = p.commands().to_vec();
    let result = p
        .commit(&mut f.bus, Actor::agent("test-model", "test-turn"))
        .unwrap();
    assert!(result.changed);
    assert_eq!(result.revision, rev + 1);
    assert_eq!(f.bus.history().len(), history + 1);
    assert_eq!(f.bus.history().last().unwrap().commands, commands);
    let after = f.bus.project().clone();
    let entity = &after.scenes[&f.scene].entities[&f.entity];
    assert_eq!(
        entity.components["Transform"],
        before.scenes[&f.scene].entities[&f.entity].components["Transform"]
    );
    let p = entity.provenance.as_ref().unwrap();
    assert_eq!(p.origin, Origin::Agent);
    assert_eq!(p.model.as_deref(), Some("test-model"));
    assert_eq!(p.transaction, f.bus.history().last().unwrap().id);
    f.bus.undo().unwrap();
    assert_eq!(f.bus.project(), &before);
    f.bus.redo().unwrap();
    assert_eq!(f.bus.project(), &after);
    f.reopen();
    assert_eq!(f.bus.project(), &after);
    f.bus
        .save_project_file(f.bus.revision(), Actor::user("test"))
        .unwrap();
    f.reopen();
    assert_eq!(f.bus.project(), &after);
    f.bus.undo().unwrap();
    assert_eq!(f.bus.project(), &before);
    f.bus.redo().unwrap();
    assert_eq!(f.bus.project(), &after);
}

#[test]
fn no_op_preserves_revision_history_and_redo_but_stale_no_op_still_refuses() {
    let mut f = Fixture::new([1, 1]);
    let p = f
        .prepare(&f.request(vec![[0, 0]], B::Paint { frame: frame(0) }))
        .unwrap();
    p.commit(&mut f.bus, Actor::user("test")).unwrap();
    f.bus.undo().unwrap();
    let rev = f.bus.revision();
    let history = f.bus.history().to_vec();
    let redo: Vec<_> = f.bus.redo_history().cloned().collect();
    let p = f.prepare(&f.request(vec![[0, 0]], B::Erase)).unwrap();
    assert!(p.commands().is_empty());
    let result = p.commit(&mut f.bus, Actor::user("test")).unwrap();
    assert!(!result.changed);
    assert!(result.changes.is_empty());
    assert_eq!(result.revision, rev);
    assert_eq!(f.bus.history(), history);
    assert_eq!(f.bus.redo_history().cloned().collect::<Vec<_>>(), redo);
    let stale = f.prepare(&f.request(vec![[0, 0]], B::Erase)).unwrap();
    f.bus.redo().unwrap();
    let after = f.bus.project().clone();
    assert!(matches!(
        stale.commit(&mut f.bus, Actor::user("test")),
        Err(E::StaleProject)
    ));
    assert_eq!(f.bus.project(), &after);
}

#[test]
fn changed_asset_deleted_selection_and_same_revision_swaps_refuse_without_publication() {
    for kind in 0..3 {
        let mut f = Fixture::new([1, 1]);
        let p = f
            .prepare(&f.request(vec![[0, 0]], B::Paint { frame: frame(0) }))
            .unwrap();
        let command = match kind {
            0 => {
                let mut asset = f.bus.project().assets[&f.texture].clone();
                asset.sha256 = "0".repeat(64);
                Command::UpsertAsset { asset }
            }
            1 => Command::DeleteEntity {
                scene_id: f.scene.clone(),
                entity_id: f.entity.clone(),
            },
            _ => Command::RenameEntity {
                scene_id: f.scene.clone(),
                entity_id: f.entity.clone(),
                name: "Changed".into(),
            },
        };
        f.bus
            .execute(vec![command], Actor::user("test"), "Concurrent edit", None)
            .unwrap();
        let before = f.bus.project().clone();
        let history = f.bus.history().to_vec();
        let rev = f.bus.revision();
        assert!(matches!(
            p.commit(&mut f.bus, Actor::user("test")),
            Err(E::StaleProject)
        ));
        assert_eq!(f.bus.project(), &before);
        assert_eq!(f.bus.history(), history);
        assert_eq!(f.bus.revision(), rev);
    }
    let f = Fixture::new([1, 1]);
    let original = CommandBus::new(f.bus.project().clone()).unwrap();
    let mut request = f.request(vec![[0, 0]], B::Erase);
    request.expected_revision = 0;
    let p = ImportSnapshot::capture(&original)
        .prepare_tilemap_paint(f.root.path(), None, &request)
        .unwrap();
    let mut altered = original.project().clone();
    altered.name = "Other same-revision document".into();
    let mut other = CommandBus::new(altered.clone()).unwrap();
    assert!(matches!(
        p.commit(&mut other, Actor::user("test")),
        Err(E::StaleProject)
    ));
    assert_eq!(other.project(), &altered);
    let mut other = CommandBus::new(Project::empty("Other identity")).unwrap();
    let p = ImportSnapshot::capture(&original)
        .prepare_tilemap_paint(f.root.path(), None, &request)
        .unwrap();
    assert!(matches!(
        p.commit(&mut other, Actor::user("test")),
        Err(E::StaleProject)
    ));
}

#[test]
fn invalid_actor_refuses_the_whole_prepared_edit() {
    let mut f = Fixture::new([1, 1]);
    let before = f.bus.project().clone();
    let history = f.bus.history().to_vec();
    let p = f
        .prepare(&f.request(vec![[0, 0]], B::Paint { frame: frame(0) }))
        .unwrap();
    assert!(matches!(
        p.commit(&mut f.bus, Actor::user("")),
        Err(E::Command(_))
    ));
    assert_eq!(f.bus.project(), &before);
    assert_eq!(f.bus.history(), history);
}
