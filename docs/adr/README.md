# Architecture decisions

These records cover every row of PLAN.md §2.1. Status is an engineering proposal
pending the director's Phase 0 review, unless a record explicitly says otherwise.
An implementation or a passing local test does not approve the architecture gate.

| Record | Stack row | Current evidence |
|---|---|---|
| 0001 | Engine core | Bevy fixed-step tests; macOS, WASM, iOS simulator probe |
| 0002 | Rendering | Real wgpu device and PNG readback on Apple M5 Pro |
| 0003 | Physics | Jolt proposal; deferred to Phase 1 |
| 0004 | Audio | Kira proposal; deferred to Phase 1 |
| 0005 | Game UI | Taffy proposal; deferred to Phase 1 |
| 0006 | Gameplay scripting | SWC, QuickJS sandbox, hot reload and measured ECS workload |
| 0007 | Native extensions | C ABI boundary exercised by iOS probe; plugins deferred |
| 0008 | Project document | Canonical JSON, schemas, Loro and Automerge benchmark |
| 0009 | Editor shell | React/Tauri and Claude connected neutral shell integrated; final native follow-ups deferred |
| 0010 | Viewport | macOS native composition reviewed; Windows/Linux builds and GPU readback passed under director CI acceptance |
| 0011 | Agent runtime | Typed local tools and Responses; real saved-account eval scored 19/20 |
| 0012 | Accounts service | Optional cloud metadata service design; no service deployed |
| 0013 | Asset generation | Provider interface design; implementation deferred to Phase 3 |
| 0014 | CI/CD | All six hosted builds and probe execution passed; scheduled nightly history pending |
