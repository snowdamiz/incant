use super::*;
use crate::ALL;
use std::collections::BTreeSet;

fn root() -> (tempfile::TempDir, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    (temp, root)
}

#[test]
fn every_starter_is_a_real_pinned_project_with_reversible_source_recipe_and_reopening() {
    let (_temp, root) = root();
    let mut all_ids = BTreeSet::new();
    for template in ALL {
        let created = create(&root.join(template.name()), template, "Starter").unwrap();
        let mut bus =
            CommandBus::open_project_file_with_journal(&created.project, &created.journal).unwrap();
        assert_eq!(bus.revision(), created.revision);
        assert_eq!(bus.history().last().unwrap().id, created.recipe_transaction);
        assert_eq!(
            bus.project().engine_version,
            Some(incant_doc::EngineRequirement::exact_current())
        );
        assert!(all_ids.insert(created.project_id.clone()));
        for id in created.roles.values() {
            assert!(all_ids.insert(id.clone()), "reused identity");
        }
        let complete = bus.project().clone();
        let source = created
            .roles
            .get("script")
            .map(|id| bus.read_script(id).unwrap());
        if let Some(source) = &source {
            assert!(source.source.as_ref().unwrap().contains("NativeBehavior"));
            assert_eq!(source.matches_history, Some(true));
        }
        assert!(
            complete.scenes[&created.roles["scene"]]
                .entities
                .values()
                .all(|entity| entity
                    .provenance
                    .as_ref()
                    .is_some_and(|p| p.transaction == created.recipe_transaction))
        );
        bus.undo().unwrap();
        assert!(bus.project().scenes.is_empty());
        assert!(bus.project().scripts.is_empty());
        assert!(
            !created
                .project
                .parent()
                .unwrap()
                .join("controller.ts")
                .exists()
        );
        bus.redo().unwrap();
        assert_eq!(bus.project(), &complete);
        if let Some(source) = &source {
            assert_eq!(
                bus.read_script(&source.script_id).unwrap().source,
                source.source
            );
        }
        let revision = bus.revision();
        bus.save_project_file(revision, Actor::user("template-test"))
            .unwrap();
        drop(bus);
        let bus =
            CommandBus::open_project_file_with_journal(&created.project, &created.journal).unwrap();
        assert_eq!(bus.project(), &complete);
        assert_eq!(bus.revision(), revision);
        let text = complete.canonical_text().unwrap();
        drop(bus);
        assert_eq!(fs::read_to_string(&created.project).unwrap(), text);
    }
}

#[test]
fn refuses_existing_destination_unknown_kind_and_invalid_name_without_writing() {
    let (_temp, root) = root();
    assert!("empty\n".parse::<Template>().is_err());
    assert!("unknown".parse::<Template>().is_err());
    let path = root.join("occupied");
    fs::create_dir(&path).unwrap();
    fs::write(path.join("keep"), b"external bytes").unwrap();
    assert!(matches!(
        create(&path, Template::Empty, "New"),
        Err(TemplateError::Destination)
    ));
    assert_eq!(fs::read(path.join("keep")).unwrap(), b"external bytes");
    assert_eq!(fs::read_dir(&path).unwrap().count(), 1);
    let fresh = root.join("fresh");
    assert!(matches!(
        create(&fresh, Template::Empty, "\n"),
        Err(TemplateError::Name)
    ));
    assert!(!fresh.exists());
}

#[test]
fn checks_complete_ancestors_before_creating_a_destination() {
    let (_temp, root) = root();
    let fresh = root.join("fresh");
    assert_eq!(destination(&fresh).unwrap(), fresh);
    assert!(!fresh.exists());

    let file = root.join("file");
    fs::write(&file, b"external ancestor").unwrap();
    assert!(matches!(
        destination(&file.join("child")),
        Err(TemplateError::Destination)
    ));
    assert_eq!(fs::read(&file).unwrap(), b"external ancestor");
    assert!(matches!(
        destination(&root.join("missing").join("child")),
        Err(TemplateError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound
    ));
    assert!(matches!(
        destination(&root.join("missing").join("..").join("child")),
        Err(TemplateError::Destination)
    ));
    assert!(!root.join("missing").exists());
}

#[cfg(windows)]
#[test]
fn canonical_windows_destination_uses_a_rooted_verbatim_anchor() {
    use std::path::Prefix;
    let (_temp, root) = root();
    assert!(matches!(
        root.components().next(),
        Some(Component::Prefix(prefix))
            if matches!(prefix.kind(), Prefix::VerbatimDisk(_) | Prefix::VerbatimUNC(..))
    ));
    let fresh = root.join("canonical-starter");
    // canonicalize supplies the same verbatim path form as the hosted gameplay
    // fixtures. The old component walk queried its unrooted prefix and failed
    // before reaching exclusive directory creation.
    assert_eq!(destination(&fresh).unwrap(), fresh);
    let created = create(&fresh, Template::Empty, "Windows starter").unwrap();
    assert_eq!(created.project, fresh.join(PROJECT));
    let bus =
        CommandBus::open_project_file_with_journal(&created.project, &created.journal).unwrap();
    assert_eq!(bus.revision(), created.revision);
    assert_eq!(bus.project().id, created.project_id);
}

#[test]
fn failure_before_publication_retains_raw_inputs_and_journal_without_a_project() {
    let (_temp, root) = root();
    let path = root.join("incomplete");
    FAIL_PUBLICATION.with(|flag| flag.set(true));
    let result = create(&path, Template::ThirdPerson, "Failure");
    FAIL_PUBLICATION.with(|flag| flag.set(false));
    assert!(matches!(result, Err(TemplateError::Incomplete { .. })));
    assert!(!path.join(PROJECT).exists());
    assert!(path.join(JOURNAL).is_file());
    assert!(path.join("controller.ts").is_file());
    assert_eq!(fs::read(path.join("template-cube.gltf")).unwrap(), MODEL);
    assert!(create(&path, Template::Empty, "Retry").is_err());
}

#[test]
fn final_publication_refuses_a_colliding_file_without_replacing_its_bytes() {
    let (_temp, root) = root();
    let path = root.join(PROJECT);
    fs::write(&path, b"external project").unwrap();
    assert!(publish(&root, "candidate project").is_err());
    assert_eq!(fs::read(&path).unwrap(), b"external project");
    assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
}

#[cfg(unix)]
#[test]
fn refuses_destination_and_parent_symlinks_without_following_them() {
    use std::os::unix::fs::symlink;
    let (_temp, root) = root();
    let outside = root.join("outside");
    fs::create_dir(&outside).unwrap();
    let alias = root.join("alias");
    symlink(&outside, &alias).unwrap();
    assert!(matches!(
        create(&alias, Template::Empty, "No"),
        Err(TemplateError::Destination)
    ));
    assert!(matches!(
        create(&alias.join("child"), Template::Empty, "No"),
        Err(TemplateError::Destination)
    ));
    assert_eq!(fs::read_dir(&outside).unwrap().count(), 0);
    let project = root.join(PROJECT);
    let external = outside.join("external");
    fs::write(&external, b"external bytes").unwrap();
    symlink(&external, &project).unwrap();
    assert!(publish(&root, "candidate project").is_err());
    assert!(
        fs::symlink_metadata(&project)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(fs::read(&external).unwrap(), b"external bytes");
    fs::remove_file(&project).unwrap();
    fs::hard_link(&external, &project).unwrap();
    assert!(publish(&root, "candidate project").is_err());
    assert_eq!(fs::read(&external).unwrap(), b"external bytes");
}

#[test]
fn plan_is_explicit_data_and_requires_the_imported_asset_role() {
    assert!(plan(Template::ThirdPerson, None).is_err());
    let p = plan(Template::Empty, None).unwrap();
    let encoded = serde_json::to_string(&p.commands).unwrap();
    let commands: Vec<incant_cmd::Command> = serde_json::from_str(&encoded).unwrap();
    let initial = Project::empty("Recipe replay");
    let mut a = CommandBus::new(initial.clone()).unwrap();
    let mut b = CommandBus::new(initial).unwrap();
    let actor = Actor::user("test");
    a.execute(commands.clone(), actor.clone(), "Apply", Some(0))
        .unwrap();
    b.execute(commands, actor, "Apply", Some(0)).unwrap();
    // Transaction IDs/provenance differ, but every supplied identity and component is exact.
    for bus in [&a, &b] {
        assert_eq!(
            bus.project().scenes[&p.roles["scene"]].entities[&p.roles["camera"]].components,
            a.project().scenes[&p.roles["scene"]].entities[&p.roles["camera"]].components
        );
    }
}
