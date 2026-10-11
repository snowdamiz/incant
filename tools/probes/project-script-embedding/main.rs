//! No compiler/authoring dependencies. Only engine-produced embedded Rust runs.
use incant_input::{InputEvent, InputRuntime, KeyCode};
use incant_runtime::{CookedEntity, CookedScene, NativeWorld, StableId};
use incant_script::{NativeSession, ScriptError, ScriptHost};
use serde_json::json;
use std::io::Read;
mod embedded {
    include!(concat!(env!("OUT_DIR"), "/behavior.rs"));
}
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn scene(path: &str) -> Result<CookedScene> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 1024 * 1024 {
        return Err("scene limit".into());
    }
    Ok(CookedScene::from_bytes(&bytes)?)
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let [mode, path, value] = args.as_slice() else {
        return Err("expected mode scene argument".into());
    };
    let cooked = scene(path)?;
    match mode.as_str() {
        "run" => {
            let ticks: u64 = value.parse()?;
            if ticks > 64 {
                return Err("tick limit".into());
            }
            let mut session = NativeSession::new(cooked, embedded::load()?, Default::default())?;
            for _ in 0..ticks {
                session.tick(&[])?;
            }
            let world = session.world()?;
            let translations: Vec<_> = world
                .snapshot()
                .iter()
                .map(|e| e.transform.translation)
                .collect();
            println!(
                "{}",
                json!({"tick":world.tick(),"translations":translations,
                "logs":session.logs()?,"source_parsing":false,"panic_abort":cfg!(panic="abort")})
            );
        }
        "rollback" => {
            let key = match value.as_str() {
                "eval" => KeyCode::KeyE,
                "function" => KeyCode::KeyF,
                "throw" => KeyCode::KeyX,
                _ => return Err("unknown failure".into()),
            };
            let mut world = NativeWorld::new(cooked.clone());
            let mut host = ScriptHost::for_world(embedded::load()?, &world)?;
            host.tick(&mut world, 0.25)?;
            let before = world.snapshot();
            let mut input = InputRuntime::default();
            let events = [InputEvent::Key {
                code: key,
                down: true,
            }];
            input.advance(&events, 0.25)?;
            let error = host
                .tick_with_input(&mut world, 0.25, &input)
                .expect_err("guest must reject");
            assert_eq!(
                world.snapshot(),
                before,
                "prior group numeric prefix escaped"
            );
            assert!(matches!(host.logs(), Err(ScriptError::FailedSession)));
            assert!(matches!(
                host.tick(&mut world, 0.25),
                Err(ScriptError::FailedSession)
            ));
            let mut session = NativeSession::new(cooked, embedded::load()?, Default::default())?;
            session.tick(&[])?;
            assert!(session.tick(&events).is_err());
            assert!(
                session.world().is_err() && session.input().is_err() && session.logs().is_err()
            );
            println!(
                "{}",
                json!({"failure":value,"rollback":true,"terminal":true,"error":error.to_string()})
            );
        }
        "pairing" => {
            let entities = (1u8..=3)
                .map(|number| {
                    let mut id = [0; 16];
                    id[15] = number;
                    CookedEntity {
                        id: StableId(id),
                        parent: None,
                        transform: Default::default(),
                        velocity: None,
                    }
                })
                .collect();
            let other = CookedScene::new(StableId([42; 16]), 4, entities)?;
            assert!(matches!(
                ScriptHost::for_world(embedded::load()?, &NativeWorld::new(other)),
                Err(ScriptError::ScriptBindings)
            ));
            assert!(matches!(
                ScriptHost::new(embedded::load()?),
                Err(ScriptError::ScriptBindings)
            ));
            println!("{}", json!({"pairing_refused":true}));
        }
        _ => return Err("unknown mode".into()),
    }
    Ok(())
}
