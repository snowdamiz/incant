# Eighteen bundled agent playbooks

The agent can discover the complete plan catalogue with `skill_list {}` and read
one playbook with `skill_read {"id":"inventory"}`. Each fixed Markdown playbook
contains a concrete authoring/runtime workflow and behavioral verification steps;
common guidance covers schema discovery, bulk scripting, bounded state, approval,
revision/source guards and evidence. No TypeScript helpers are bundled; the plan
makes those optional. The source catalogue is in `sdk/skills/`.

All eighteen IDs are implemented: profile-optimize, third-person, first-person,
platformer-2d, top-down, camera-rigs, inventory, dialogue, health-damage, spawners,
checkpoints, mobile-touch, settings-menu, main-menu, save-load, simple-ai,
day-night and localize-project. Localization guidance requires every drafted
string/locale to remain pending human review in reversible project memory.

The Markdown is compiled into the agent. Neither project memory nor a project
path can replace it. Requests have closed schemas; lookup accepts only exact
catalogue IDs. Reading guidance does not execute helpers, access arbitrary files,
add shell/network privileges or bypass tool approvals. Subsequent mutations still
use existing authoring commands and source/revision fences. The system guidance
asks the agent to consult relevant playbooks without overriding user scope.

Guidance distinguishes actual SDK calls from missing host capabilities. Profiler
access, native module compilation/export, aggregate session saves, complete
archetype spawning and some lighting/input controls still need their real host
interfaces before those workflows can run end-to-end. The playbooks explicitly
report missing capabilities rather than inventing tools. Their presence does not
prove eighteen game mechanics or live agent effectiveness.

## Qualification

Source `907bbaea51a9c129d6811eafd14090d45c58d1ad` passes all 118 agent
CPU tests, strict all-target Clippy, formatting and conventions on this Mac with
Rust 1.99.0, arm64 CI profile, locked/offline dependencies and two build jobs.
Three new behavior controls prove discovery/read bounds, refusal of unknown
IDs/path/execution overrides, immutable project history and ordinary approval
denial after a mocked provider reads a skill. These are fixture-provider tests;
no connected account was called.

Independent review read all eighteen playbooks, common guidance, implementation,
SDK references, tests and qualification logs; it found no scoped issue.
[Retained source identities, commands, logs and review](evidence/builtin-agent-skills/manifest.json)
bind this result to the source commit. Logs are portable; the bounded runner is
retained for provenance and contains the producing machine's paths. No executable
is archived or claimed replayable. No browser, GPU, native window or capture ran.
Live skill-following evaluation, the 200-task corpus, provider/asset-generation
coverage, missing host capabilities and the Phase 3 gate remain open.
