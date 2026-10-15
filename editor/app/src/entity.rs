//! Host identity allocation; the shared command bus remains the only writer.
use crate::{Editor, engine_read};
use incant_cmd::{Actor, CommandBus, CommandError};
use serde_json::{Value, json};
use std::sync::Arc;

fn duplicate(
    bus: &mut CommandBus,
    scene: &str,
    entity: &str,
    revision: u64,
) -> Result<Value, CommandError> {
    if bus.revision() != revision {
        return Err(CommandError::Conflict {
            expected: revision,
            actual: bus.revision(),
        });
    }
    let (command, created) = bus.prepare_duplicate_subtree(scene, entity)?;
    let project = bus.project().id.clone();
    let transaction = bus
        .execute(
            vec![command],
            Actor::user("editor"),
            "Duplicate subtree",
            Some(revision),
        )?
        .id
        .clone();
    Ok(
        json!({"project":project,"entity":created,"revision":bus.revision(),"transaction":transaction}),
    )
}

#[tauri::command]
pub async fn engine_duplicate_subtree(
    state: tauri::State<'_, Arc<Editor>>,
    scene_id: String,
    entity_id: String,
    expected_revision: u64,
) -> Result<Value, String> {
    let bus = state.bus.clone();
    let receipt = tauri::async_runtime::spawn_blocking(move || {
        let mut load = bus.lock().map_err(|_| "engine lock failed".to_string())?;
        duplicate(load.bus_mut()?, &scene_id, &entity_id, expected_revision)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;
    Ok(json!({"receipt":receipt,"snapshot":engine_read(state)?}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use incant_doc::{Entity, Project, Scene};

    #[test]
    fn host_copy_is_one_reversible_user_transaction_and_stale_requests_write_nothing() {
        let mut project = Project::empty("Copy");
        let mut scene = Scene::new("Scene");
        let root = Entity::new("Root");
        let mut child = Entity::new("Child");
        child.parent = Some(root.id.clone());
        scene.entities.insert(root.id.clone(), root.clone());
        scene.entities.insert(child.id.clone(), child);
        project.scenes.insert(scene.id.clone(), scene.clone());
        let mut bus = CommandBus::new(project.clone()).unwrap();
        assert!(duplicate(&mut bus, &scene.id, &root.id, 1).is_err());
        assert_eq!(bus.project(), &project);
        assert_eq!(bus.revision(), 0);
        let receipt = duplicate(&mut bus, &scene.id, &root.id, 0).unwrap();
        let copy = receipt["entity"].as_str().unwrap();
        assert_ne!(copy, root.id);
        assert_eq!(bus.project().scenes[&scene.id].entities.len(), 4);
        assert!(
            bus.project().scenes[&scene.id]
                .entities
                .values()
                .any(|e| e.parent.as_deref() == Some(copy))
        );
        assert_eq!(receipt["revision"], 1);
        assert_eq!(receipt["project"], project.id);
        assert_eq!(receipt["transaction"], bus.history()[0].id);
        assert_eq!(bus.history()[0].actor, Actor::user("editor"));
        let after = bus.project().clone();
        bus.undo().unwrap();
        assert_eq!(bus.project(), &project);
        bus.redo().unwrap();
        assert_eq!(bus.project(), &after);
    }
}
