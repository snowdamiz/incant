#![cfg(feature = "scene")]
#[path = "support/scene_format.rs"]
mod format;
use format::scene_format;
use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::{Entity, Project, Scene};
use incant_import::{ImportRequest, ImportSnapshot};
use incant_runtime::{CookedScene, NativeWorld};
use incant_types::{TextWrap, UiKind, UiNode, UiStyle, UiText};
use incant_ui::{UiEvent, UiFonts, UiRuntime, Widget};
use std::{fs, sync::Arc};
const FONT: &[u8] = include_bytes!("../../incant_text/tests/fonts/NotoSans-Regular.ttf");
fn fixture(root: &std::path::Path) -> (CommandBus, String, String, String) {
    let mut bus = CommandBus::new(Project::empty("UI")).unwrap();
    fs::write(root.join("font.ttf"), FONT).unwrap();
    let imported = ImportSnapshot::capture(&bus)
        .prepare(
            root,
            None,
            &[ImportRequest {
                source: "font.ttf".into(),
                texture_usage: None,
            }],
        )
        .unwrap()
        .commit(&mut bus, Actor::import("UI fixture"), "Import font")
        .unwrap();
    let font = imported.imports[0].asset.id.clone();
    let scene = Scene::new("Main");
    let sid = scene.id.clone();
    let mut root = Entity::new("Root");
    let rid = root.id.clone();
    let mut input = Entity::new("Input");
    let eid = input.id.clone();
    input.parent = Some(rid.clone());
    let node = |kind| UiNode {
        kind,
        style: UiStyle::default(),
        order: 0,
        disabled: false,
    };
    root.components.insert(
        "UiNode".into(),
        serde_json::to_value(node(UiKind::<String>::Root)).unwrap(),
    );
    input.components.insert(
        "UiNode".into(),
        serde_json::to_value(node(UiKind::TextInput {
            text: UiText {
                content: "Name".into(),
                fonts: vec![font],
                font_size: 18.0,
                line_height: 24.0,
                wrap: TextWrap::WordOrGlyph,
            },
        }))
        .unwrap(),
    );
    bus.execute(
        vec![
            Command::CreateScene { scene },
            Command::CreateEntity {
                scene_id: sid.clone(),
                entity: root,
            },
            Command::CreateEntity {
                scene_id: sid.clone(),
                entity: input,
            },
        ],
        Actor::agent("test", "UI"),
        "Create UI",
        None,
    )
    .unwrap();
    (bus, sid, rid, eid)
}
#[test]
fn command_bus_cooked_world_and_packaged_fonts_drive_actual_layout_and_input_after_deletion() {
    let temp = tempfile::tempdir().unwrap();
    let (mut bus, sid, _, eid) = fixture(temp.path());
    let authored = bus.project().canonical_text().unwrap();
    let bytes = incant_cook::cook_scene(bus.project(), &sid).unwrap();
    assert_eq!(&bytes[8..12], &scene_format(13).to_le_bytes());
    bus.undo().unwrap();
    assert!(bus.project().scenes.is_empty());
    bus.redo().unwrap();
    assert_eq!(bus.project().canonical_text().unwrap(), authored);
    let scene = CookedScene::from_bytes(&bytes).unwrap();
    let fonts = incant_assets_runtime::FontCatalog::load(
        &temp.path().join(".incant/cache/fonts"),
        scene.font_assets(),
    )
    .unwrap();
    let sources = fonts
        .into_fonts()
        .map(|(id, font)| font.into_source(ulid::Ulid::from_bytes(id.0)).into())
        .collect();
    let mut world = NativeWorld::new(scene);
    drop(bus);
    temp.close().unwrap();
    let widgets = world
        .ui_view()
        .unwrap()
        .nodes()
        .map(|n| Widget {
            id: n.id,
            incarnation: n.incarnation,
            parent: n.parent,
            node: Arc::clone(n.node),
        })
        .collect::<Vec<_>>();
    let mut fonts = UiFonts::new(sources, &widgets).unwrap();
    let mut ui = UiRuntime::new(widgets).unwrap();
    let pending = ui.prepare(&[], [320.0, 200.0], &mut fonts).unwrap();
    ui.commit(pending).unwrap();
    let id = incant_cook::stable_id(&eid).unwrap();
    let p = ui
        .prepare(&[UiEvent::Focus(id)], [320.0, 200.0], &mut fonts)
        .unwrap();
    ui.commit(p).unwrap();
    let target = ui.focus().unwrap();
    let p = ui
        .prepare(
            &[UiEvent::Commit {
                target,
                text: " saved".into(),
            }],
            [320.0, 200.0],
            &mut fonts,
        )
        .unwrap();
    world.step().unwrap();
    ui.commit(p).unwrap();
    assert_eq!(ui.text(id).unwrap().text(), "Name saved");
    assert_eq!(world.tick(), 1);
    assert_eq!(ui.actions().len(), 1);
    let frame = ui.layout().unwrap();
    let widget = ui.widgets().iter().find(|w| w.id == id).unwrap();
    let shaped = fonts
        .shape(
            widget,
            ui.text(id).unwrap().text(),
            frame
                .nodes
                .iter()
                .find(|n| n.id == id)
                .unwrap()
                .content
                .size[0],
        )
        .unwrap();
    assert!(shaped.glyphs().len() > 5);
    assert!(shaped.missing_clusters().is_empty());
}
#[test]
fn final_command_validation_keeps_hierarchy_assets_and_undo_atomic() {
    let temp = tempfile::tempdir().unwrap();
    let (mut bus, sid, rid, eid) = fixture(temp.path());
    let before = bus.project().canonical_text().unwrap();
    let revision = bus.revision();
    let value = serde_json::json!({"linear":[0,0,0]});
    assert!(
        bus.execute(
            vec![Command::SetComponent {
                scene_id: sid.clone(),
                entity_id: eid.clone(),
                component: "Velocity".into(),
                value
            }],
            Actor::user("test"),
            "Invalid coexistence",
            None
        )
        .is_err()
    );
    // ReplaceEntity is deliberately not needed: ordinary reparent command must
    // enforce the same final document invariants as every other client.
    let result = bus.execute(
        vec![Command::ReparentEntity {
            scene_id: sid.clone(),
            entity_id: eid.clone(),
            parent: None,
        }],
        Actor::user("test"),
        "Invalid parent",
        None,
    );
    assert!(result.is_err());
    assert_eq!(bus.project().canonical_text().unwrap(), before);
    assert_eq!(bus.revision(), revision);
    let _ = rid;
}
