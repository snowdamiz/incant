//! Build-time typed fixture for the real default native gameplay library.
//! This writes cooked runtime data; no document/CRDT smoke or host font discovery.
#![forbid(unsafe_code)]
use incant_assets_runtime::sha256;
use incant_runtime::{CookedEntity, CookedScene, StableId};
use incant_types::{Transform, Velocity};
use serde_json::json;
use std::{collections::BTreeMap, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let root = PathBuf::from(
        args.next()
            .ok_or("expected fresh absolute output directory")?,
    );
    if args.next().is_some() || !root.is_absolute() {
        return Err("expected exactly one absolute output directory".into());
    }
    fs::create_dir(&root)?;
    let package = root.join("package");
    fs::create_dir(&package)?;
    let scene = CookedScene::new(
        StableId(99_u128.to_be_bytes()),
        60,
        vec![CookedEntity {
            id: StableId(1_u128.to_be_bytes()),
            parent: None,
            transform: Transform::default(),
            velocity: Some(Velocity { linear: [0.; 3] }),
        }],
    )?;
    let replay = |events: serde_json::Value| json!({"format":"incant-input","version":1,"start_tick":0,"tick_rate":60,"ticks":3,"frames":events});
    let files = [
        ("world.scene", scene.to_bytes()),
        ("clips.bin", [b"NCLIPS01".as_slice(), &[0; 4]].concat()),
        (
            "input-actions.json",
            serde_json::to_vec(&json!({
                "right":{"kind":"button","bindings":[{"type":"key","code":"KeyD"}]},
                "fail":{"kind":"button","bindings":[{"type":"key","code":"KeyF"}]}
            }))?,
        ),
        (
            "empty-input.json",
            serde_json::to_vec(&replay(json!([
                {"tick":1,"events":[{"type":"key","code":"KeyD","down":true}]}
            ])))?,
        ),
    ];
    let mut hashes = BTreeMap::new();
    for (name, bytes) in files {
        hashes.insert(name, sha256(&bytes));
        fs::write(package.join(name), bytes)?;
    }
    fs::write(
        package.join("package.json"),
        serde_json::to_vec(&json!({
            "format":"incant-native-package","version":1,"camera":null,"files":hashes
        }))?,
    )?;
    let observation = |x: f64| {
        json!({"format":"incant-native-observation","version":1,
        "entities":["00000000000000000000000001"],"checks":[{
            "name":"actual cooked native movement","tick":3,"expect":{"type":"vector",
            "entity":"00000000000000000000000001","property":"translation",
            "value":[x,0,0],"tolerance":1e-12}}]})
    };
    for (name, data) in [
        ("observe.json", observation(0.1)),
        ("observe-release.json", observation(0.05)),
        ("observe-wrong.json", observation(100.0)),
        (
            "failure.json",
            replay(json!([{"tick":2,"events":[{"type":"key","code":"KeyF","down":true}]}])),
        ),
        (
            "release.json",
            replay(json!([
                {"tick":1,"events":[{"type":"key","code":"KeyD","down":true}]},
                {"tick":2,"events":[{"type":"key","code":"KeyD","down":false}]}
            ])),
        ),
    ] {
        fs::write(root.join(name), serde_json::to_vec(&data)?)?;
    }
    println!(
        "{}",
        json!({"format":"incant-mobile-runtime-fixture","version":1,
        "scene_sha256":hashes["world.scene"],"package":package,"tick_rate":60,"ticks":3,
        "position_on_held_input":[0.1,0,0],"position_on_released_input":[0.05,0,0]})
    );
    Ok(())
}
