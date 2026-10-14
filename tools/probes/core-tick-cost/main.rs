use bevy_ecs::prelude::*;
use rquickjs::{Context, Function, Runtime};
use std::{hint::black_box, path::Path, time::Instant};

#[derive(Component)]
struct Identity(u32);
#[derive(Component)]
struct Moved;

fn identities() {
    let mut world = World::new();
    let mut tokens: Vec<_> = (0..10_000)
        .map(|id| (id, world.spawn(Identity(id)).id()))
        .collect();
    let captured_world = world.id();
    let foreign_world = World::new().id();
    for phase in 0..3 {
        if phase == 1 {
            world.entity_mut(tokens[0].1).insert(Moved);
            world.despawn(tokens[1].1);
            world.spawn(Identity(tokens[1].0));
            tokens[2].0 = u32::MAX; // wrong StableId analogue.
        }
        if phase == 2 {
            world = World::new();
            for id in 0..10_000 {
                world.spawn(Identity(id));
            }
        }
        // No state or archetype cache survives this immutable batch boundary.
        let query = world.try_query::<&Identity>().unwrap();
        for foreign in [false, true] {
            let expected = if foreign || phase == 2 {
                0
            } else if phase == 0 {
                10_000
            } else {
                9_998
            };
            for method in ["direct", "query"] {
                let mut times = Vec::new();
                for _ in 0..300 {
                    let started = Instant::now();
                    let mut matches = 0;
                    // Query preparation cost is included in every batch.
                    let batch =
                        (method == "query").then(|| world.try_query::<&Identity>().unwrap());
                    for &(id, entity) in black_box(&tokens) {
                        let token_world = if foreign {
                            foreign_world
                        } else {
                            captured_world
                        };
                        let same_world = black_box(token_world == world.id());
                        let live = if let Some(batch) = &batch {
                            same_world
                                && batch
                                    .get_manual(&world, entity)
                                    .is_ok_and(|value| value.0 == id)
                        } else {
                            same_world
                                && world
                                    .get::<Identity>(entity)
                                    .is_some_and(|value| value.0 == id)
                        };
                        matches += u32::from(black_box(live));
                    }
                    times.push(started.elapsed().as_nanos() as u64);
                    assert_eq!(matches, expected);
                }
                println!(
                    "{}",
                    serde_json::json!({"diagnostic_only":true,"kind":"identity",
                    "phase":phase,"foreign":foreign,"method":method,"ns":times})
                );
            }
            for &(id, entity) in &tokens {
                assert_eq!(
                    world
                        .get::<Identity>(entity)
                        .is_some_and(|value| value.0 == id),
                    query
                        .get_manual(&world, entity)
                        .is_ok_and(|value| value.0 == id)
                );
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::args().nth(1).ok_or("missing root")?;
    for negative in ["omitted", "nanY", "nanZ"] {
        let mutation = Runtime::new()?;
        mutation.set_memory_limit(32 * 1024 * 1024);
        let mutation_context = Context::full(&mutation)?;
        let source = std::fs::read(Path::new(&root).join(format!("{negative}.js")))?;
        mutation_context.with(|ctx| -> rquickjs::Result<()> {
            ctx.eval::<(), _>(source)?;
            let update: Function = ctx.globals().get("update")?;
            let verify: Function = ctx.globals().get("verify")?;
            update.call::<_, ()>((1u32,))?;
            assert!(
                verify.call::<_, ()>((1u32,)).is_err(),
                "negative mutation escaped oracle"
            );
            Ok(())
        })?;
        println!(
            "{}",
            serde_json::json!({"diagnostic_only":true,"kind":"negative-refused", "variant":negative})
        );
    }
    for name in ["baseline", "count", "length", "cursor", "soa", "locals"] {
        let source = std::fs::read(Path::new(&root).join(format!("{name}.js")))?;
        let runtime = Runtime::new()?;
        runtime.set_memory_limit(32 * 1024 * 1024);
        let context = Context::full(&runtime)?;
        let result: Result<_, rquickjs::Error> = context.with(|ctx| {
            ctx.eval::<(), _>(source)?;
            let update: Function = ctx.globals().get("update")?;
            let verify: Function = ctx.globals().get("verify")?;
            let transitions: Function = ctx.globals().get("transitions")?;
            let mut times = Vec::new();
            for tick in 1..=500u32 {
                let started = Instant::now();
                update.call::<_, ()>((tick,))?;
                times.push(started.elapsed().as_nanos() as u64);
                if [1, 64, 500].contains(&tick) {
                    verify.call::<_, ()>((tick,))?;
                }
            }
            transitions.call::<_, ()>(())?;
            Ok(times)
        });
        println!(
            "{}",
            serde_json::json!({"diagnostic_only":true,"kind":"loop","variant":name,
            "ns":result?,"quickjs_heap_used":runtime.memory_usage().memory_used_size})
        );
    }
    identities();
    Ok(())
}
