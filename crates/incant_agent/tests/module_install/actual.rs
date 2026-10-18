//! Must be explicitly selected in coordinated native qualification. Being
//! compiled or ignored does not prove the compiler→install→rebuild route.
use super::*;
use std::collections::BTreeSet;
use std::path::Path;
const GUEST: &str = "#![no_std]\nmod helper;\nuse incant_module::{Tick,Result};\n#[panic_handler] fn panic(_: &core::panic::PanicInfo<'_>)->!{core::arch::wasm32::unreachable()}\nincant_module::module!(on_update);\nfn on_update(_: &mut Tick<'_>, _:f64)->Result<()>{let _=helper::VALUE;Ok(())}\n";
#[test]
#[ignore = "actual configured bounded Local worker; requires explicit native CPU grant"]
fn actual_local_compile_install_edit_rebuild_reinstall_and_candidate_free_reopen() {
    let mut f = Fixture::new();
    fs::write(f.root.path().join("logic/main.rs"), GUEST).unwrap();
    fs::write(
        f.root.path().join("logic/helper.rs"),
        "pub const VALUE:u32=7;",
    )
    .unwrap();
    let config: incant_module_build::backend::Config = serde_json::from_slice(
        &fs::read(Path::new(
            &std::env::var_os("INCANT_MODULE_BUILD_CONFIG").expect("trusted host configuration"),
        ))
        .unwrap(),
    )
    .unwrap();
    let before = fs::read_dir(&config.storage)
        .unwrap()
        .map(|p| p.unwrap().path())
        .collect::<BTreeSet<_>>();
    f.jobs = std::sync::Arc::new(incant_module_build::Service::new(
        incant_module_build::backend::Local::from_environment().unwrap(),
    ));
    f.jobs.bind(&f.project.id, f.root.path()).unwrap();
    let first = f.build();
    assert!(
        f.run(
            &first,
            ApprovalMode::Always,
            &AtomicBool::new(false),
            &mut |_, _| true
        )
        .unwrap()
        .tool_calls[0]
            .succeeded
    );
    let first_asset = f
        .bus
        .lock()
        .unwrap()
        .project()
        .assets
        .values()
        .next()
        .unwrap()
        .clone();
    write(
        &f,
        &format!("{GUEST}\n// Second registered source revision\n"),
    );
    let second = f.build(); // Local registered capture must admit only main.wasm as output.
    assert_ne!(first["candidate_id"], second["candidate_id"]);
    assert!(
        f.run(
            &second,
            ApprovalMode::Always,
            &AtomicBool::new(false),
            &mut |_, _| true
        )
        .unwrap()
        .tool_calls[0]
            .succeeded
    );
    {
        let mut bus = f.bus.lock().unwrap();
        assert_eq!(bus.project().assets.len(), 1);
        assert_eq!(
            bus.project().assets.values().next().unwrap().id,
            first_asset.id
        );
        let ordinary = ImportSnapshot::capture(&bus)
            .prepare(
                f.root.path(),
                None,
                &[ImportRequest {
                    source: first_asset.path.clone(),
                    texture_usage: None,
                }],
            )
            .unwrap();
        assert!(!ordinary.outcomes()[0].changed);
        let revision = bus.revision();
        bus.save_project_file(revision, Actor::user("save"))
            .unwrap();
    }
    // Remove only candidate files from jobs created by this test. Journal-owned
    // blobs, SDK caches, prior jobs and all worker receipts remain untouched.
    for job in fs::read_dir(&config.storage)
        .unwrap()
        .map(|p| p.unwrap().path())
        .filter(|p| !before.contains(p))
    {
        let candidate = job.join("module.wasm");
        if candidate.is_file() {
            fs::remove_file(candidate).unwrap();
        }
    }
    f.jobs.close();
    f.reopen();
    let mut bus = f.bus.lock().unwrap();
    bus.undo().unwrap();
    bus.redo().unwrap();
    assert!(f.root.path().join(&first_asset.path).is_file());
    assert_eq!(
        bus.project().assets.values().next().unwrap().id,
        first_asset.id
    );
}
