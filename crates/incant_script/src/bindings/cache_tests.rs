use super::*;
use incant_runtime::{CookedEntity, CookedScene, FrameCommands, StructuralCommand};
use rquickjs::{Context, Runtime};

fn id(n: u8) -> StableId {
    StableId([n; 16])
}
fn entity(n: u8) -> CookedEntity {
    CookedEntity {
        id: id(n),
        parent: None,
        transform: Default::default(),
        velocity: None,
    }
}
fn fixture() -> (NativeWorld, ScriptBindings) {
    let scene = CookedScene::new(id(99), 60, vec![entity(1), entity(2), entity(3)]).unwrap();
    let bindings = ScriptBindings::new(
        Sha256::digest(scene.to_bytes()).into(),
        vec![id(101), id(102)],
        (1..=3)
            .map(|n| ScriptInstance {
                entity: id(n),
                source: id(if n == 3 { 102 } else { 101 }),
                props: BTreeMap::new(),
            })
            .collect(),
    )
    .unwrap();
    (NativeWorld::new(scene), bindings)
}
fn reuse(world: &mut NativeWorld, n: u8) {
    let mut commands = FrameCommands::default();
    commands.push(StructuralCommand::Despawn(id(n))).unwrap();
    world.apply(&mut commands).unwrap();
    commands.push(StructuralCommand::Spawn(entity(n))).unwrap();
    world.apply(&mut commands).unwrap();
}

#[test]
fn paired_cache_survives_archetype_changes_but_not_entity_lifetime_reuse() {
    let runtime = Runtime::new().unwrap();
    let context = Context::full(&runtime).unwrap();
    let (mut world, bindings) = fixture();
    context.with(|ctx| {
        let freeze: Function = ctx.eval("Object.freeze").unwrap();
        let mut project = BoundProject::new(&ctx, &freeze, &bindings, &world).unwrap();
        let mut retired = Vec::new();
        let (a, ids) = project.groups[0]
            .snapshot(&ctx, &freeze, &world, &mut retired)
            .unwrap();
        let (b, other_ids) = project.groups[1]
            .snapshot(&ctx, &freeze, &world, &mut retired)
            .unwrap();
        let mut commands = FrameCommands::default();
        commands
            .push(StructuralCommand::SetParent {
                entity: id(1),
                parent: Some(id(3)),
            })
            .unwrap();
        commands
            .push(StructuralCommand::SetVelocity {
                entity: id(1),
                velocity: Some(incant_types::Velocity { linear: [0.; 3] }),
            })
            .unwrap();
        world.apply(&mut commands).unwrap();
        let (same, same_ids) = project.groups[0]
            .snapshot(&ctx, &freeze, &world, &mut retired)
            .unwrap();
        assert_eq!(a.as_value(), same.as_value());
        assert!(Arc::ptr_eq(&ids, &same_ids));
        assert!(retired.is_empty());
        reuse(&mut world, 1);
        let (changed, remaining) = project.groups[0]
            .snapshot(&ctx, &freeze, &world, &mut retired)
            .unwrap();
        assert_ne!(a.as_value(), changed.as_value());
        assert!(!Arc::ptr_eq(&ids, &remaining));
        assert_eq!(&*ids, &[id(1), id(2)]);
        assert_eq!(&*remaining, &[id(2)]);
        assert_eq!(a.len(), 2);
        assert_eq!(changed.len(), 1);
        assert_eq!(
            retired,
            vec![crate::persistence::Key {
                scope: crate::schedule::Scope::Source(id(101)),
                entity: Some(id(1))
            }]
        );
        let (other, unchanged) = project.groups[1]
            .snapshot(&ctx, &freeze, &world, &mut retired)
            .unwrap();
        assert_eq!(b.as_value(), other.as_value());
        assert!(Arc::ptr_eq(&other_ids, &unchanged));
        // The old public ID has a new lifetime but can never be reattached.
        retired.clear();
        let (_, again) = project.groups[0]
            .snapshot(&ctx, &freeze, &world, &mut retired)
            .unwrap();
        assert!(Arc::ptr_eq(&remaining, &again));
        assert!(retired.is_empty());
    });
}

#[test]
fn failed_rebuild_drops_stale_pair_and_publishes_no_partial_snapshot() {
    let runtime = Runtime::new().unwrap();
    let context = Context::full(&runtime).unwrap();
    let (mut world, bindings) = fixture();
    context.with(|ctx| {
        let freeze: Function = ctx.eval("Object.freeze").unwrap();
        let reject: Function = ctx
            .eval("()=>{throw Error('injected freeze failure')}")
            .unwrap();
        let mut project = BoundProject::new(&ctx, &freeze, &bindings, &world).unwrap();
        let group = &mut project.groups[0];
        let mut retired = Vec::new();
        // The first construction also cannot publish a partial pair.
        assert!(group.snapshot(&ctx, &reject, &world, &mut retired).is_err());
        let _ = ctx.catch();
        assert!(group.snapshot.is_none());
        let (old, old_ids) = group.snapshot(&ctx, &freeze, &world, &mut retired).unwrap();
        reuse(&mut world, 1);
        assert!(group.snapshot(&ctx, &reject, &world, &mut retired).is_err());
        let _ = ctx.catch();
        assert!(group.snapshot.is_none());
        assert_eq!(old.len(), 2);
        assert_eq!(&*old_ids, &[id(1), id(2)]);
        let (new, ids) = group.snapshot(&ctx, &freeze, &world, &mut retired).unwrap();
        assert_eq!(new.len(), 1);
        assert_eq!(&*ids, &[id(2)]);
        assert_eq!(retired.len(), 1);
    });
}

#[test]
fn empty_and_restored_groups_cache_only_their_admitted_active_membership() {
    let runtime = Runtime::new().unwrap();
    let context = Context::full(&runtime).unwrap();
    let (mut world, bindings) = fixture();
    context.with(|ctx| {
        let freeze: Function = ctx.eval("Object.freeze").unwrap();
        let mut original = BoundProject::new(&ctx, &freeze, &bindings, &world).unwrap();
        let mut retired = Vec::new();
        let (old, _) = original.groups[0]
            .snapshot(&ctx, &freeze, &world, &mut retired)
            .unwrap();
        reuse(&mut world, 1);
        reuse(&mut world, 2);
        let (empty, empty_ids) = original.groups[0]
            .snapshot(&ctx, &freeze, &world, &mut retired)
            .unwrap();
        assert!(empty.is_empty() && empty_ids.is_empty());
        let (same, same_ids) = original.groups[0]
            .snapshot(&ctx, &freeze, &world, &mut retired)
            .unwrap();
        assert_eq!(empty.as_value(), same.as_value());
        assert!(Arc::ptr_eq(&empty_ids, &same_ids));
        assert_eq!(old.len(), 2);
        assert_eq!(retired.len(), 2);
        let mut restored =
            BoundProject::restore(&ctx, &freeze, &bindings, &world, &[id(3)]).unwrap();
        let (inactive, ids) = restored.groups[0]
            .snapshot(&ctx, &freeze, &world, &mut retired)
            .unwrap();
        assert!(inactive.is_empty() && ids.is_empty());
        assert_ne!(empty.as_value(), inactive.as_value());
        assert_eq!(restored.active_ids(&world).unwrap(), vec![id(3)]);
    });
}

#[test]
fn cache_build_and_retirement_rebuild_never_call_inherited_array_accessors() {
    let runtime = Runtime::new().unwrap();
    let context = Context::full(&runtime).unwrap();
    let (mut world, bindings) = fixture();
    context.with(|ctx| {
        let freeze: Function = ctx.eval("Object.freeze").unwrap();
        let mut project = BoundProject::new(&ctx, &freeze, &bindings, &world).unwrap();
        ctx.eval::<(), _>(r#"Object.defineProperty(Array.prototype,'0',{
            get(){throw Error('inherited get')},set(){throw Error('inherited set')},configurable:true
        });Object.freeze=()=>{throw Error('guest freeze')};"#).unwrap();
        let mut retired = Vec::new();
        let group = &mut project.groups[0];
        let (old, _) = group.snapshot(&ctx, &freeze, &world, &mut retired).unwrap();
        let (same, _) = group.snapshot(&ctx, &freeze, &world, &mut retired).unwrap();
        assert_eq!(old.as_value(), same.as_value());
        reuse(&mut world, 1);
        let (new, ids) = group.snapshot(&ctx, &freeze, &world, &mut retired).unwrap();
        let first: Object = new.get(0).unwrap();
        assert_eq!(first.get::<_, String>("entity").unwrap(), ulid::Ulid::from_bytes(id(2).0).to_string());
        assert_eq!(&*ids, &[id(2)]);
        ctx.globals().set("saved", new).unwrap();
        assert!(ctx.eval::<bool, _>("Object.isFrozen(saved) && Object.isFrozen(saved[0])").unwrap());
    });
}
