# Published schemas and the shipping source freeze

Checkpoint44's [general run 38691592274](https://github.com/snowdamiz/incant/actions/runs/38691592274)
failed at the unchanged `FreshSession` clean-source admission check. Its frozen
candidate was `fc0afc7b07dbabda3e8eb78415169dbbe9778863`. The workflow generated
schemas directly into tracked `schemas/` at 07:18:44 UTC and started shipping
at 07:29:46 UTC. The schema diff check was later in the job, so it was never
reached. The job did not retain its dirty diff.

The stale published file is reproduced locally: `agent-tools.json` omits the
already implemented `play_run.profile`, `profiler_read`, `skill_list` and
`skill_read` contracts. All other 77 generated files are byte-identical. This
is an actual retained headless executable result, not a mocked schema generator.
Its SHA-256 is
`8efebd8c2481940ab049b62ab5026f26ffe146285e0e1f780f3d260505c59cb7`, from producing
source `069bbe54dc529511138867dfd4fbe50507a38b6b`. The schema generator, agent and
schema-bearing package trees and locked dependencies match the candidate;
[retained identities and comparisons](evidence/ci-shipping-schema-check.json)
bind that reuse. The runtime package also contains an intervening light-control
addition; its 2D query/hit/event schema types remain reexports of the identical
`incant_physics_2d_runtime` package.

The published agent registry is regenerated. CI now calls `check_schemas.py`
before shipping: it generates into a new temporary directory, compares every
published/generated filename and exact file bytes, reports only discrepant
relative names, and removes private outputs on success or failure. Stale,
missing and newly added contracts all fail immediately without rewriting the
source tree. The later tracked-schema diff check remains, and `FreshSession`
still requires a clean tracked checkout and unchanged sources throughout its
build session. No reset, source restoration or freeze exemption is introduced.

Local verification:

- The actual binary rejects a disposable copy of the candidate's complete
  78-file registry with exactly `Stale published schema: agent-tools.json`.
  Every original file hash remains unchanged.
- The actual binary passes the repaired complete registry; every published
  file hash remains unchanged.
- Seven checker controls pass, including stale tool content, same-meaning
  byte drift, missing/extra schemas, child failure cleanup and workflow order.
  The 48 existing shipping controls pass unchanged.
- Generated TypeScript SDK and conventions freshness and `git diff --check`
  pass. The pre-edit Python tool suite passed 521 tests with four skips and
  left the checkout clean; it was not repeated after these focused changes.

No new Rust compilation, shipping-player execution or hosted CI rerun was
performed for this fix. The original hosted dirty diff is unavailable, so
additional independent mutations in that historical runner cannot be excluded.
The demonstrated stale registry alone explains the freeze failure. Checkpoint44
and release promotion remain held until a corrected candidate passes its full
required checks.
