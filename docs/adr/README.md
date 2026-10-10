# Architecture decisions

These records cover every row of PLAN.md §2.1. Every record is decided by director
instruction (PLAN.md revision 4 for the performance architecture, revision 5 for the
decision record in section 12). A decision, an implementation or a passing local test
does not approve the Phase 0 gate, which stays with the director.

| Record | Stack row | Current evidence |
|---|---|---|
| 0001 | Engine core | Bevy fixed-step tests; macOS, WASM, iOS simulator probe |
| 0002 | Rendering | Real wgpu device and PNG readback on Apple M5 Pro |
| 0003 | Physics | Rapier deterministic variant implemented, serial; parallel solver, fast variant and Jolt benchmark pending |
| 0004 | Audio | Kira proposal; deferred to Phase 1 |
| 0005 | Game UI | Taffy proposal; deferred to Phase 1 |
| 0006 | Gameplay scripting | Legacy document-backed path measured; runtime-world migration, bytecode, wasm2c modules and browser host pending |
| 0007 | Native extensions | C ABI boundary exercised by iOS probe; plugins deferred |
| 0008 | Project document | Canonical JSON, schemas, Loro and Automerge benchmark; per-scene documents and 100,000-entity gate pending |
| 0009 | Editor shell | React/Tauri and Claude connected neutral shell integrated; final native follow-ups deferred |
| 0010 | Viewport | macOS native composition reviewed; Windows/Linux builds and GPU readback passed under director CI acceptance |
| 0011 | Agent runtime | Typed local tools and Responses; real saved-account eval scored 19/20 |
| 0012 | Accounts service | WorkOS identity and AWS hosting decided; no service deployed |
| 0013 | Asset generation | Provider selection rule decided; implementation in Phase 3 |
| 0014 | CI/CD | All six hosted builds and probe execution passed; scheduled nightly history pending |
