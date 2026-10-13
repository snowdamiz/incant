# Optional Core result-label binding

This sample-only correction answers the numerical requests in the accepted
0065 presentation packet. The authored HUD may add one label bound as
`hud.prompt`, with translations at string-table key `hud.prompt`. The existing
12-widget HUD remains supported. No label, wording, style, font or layout is
added to the production presentation by this change; Claude owns that work.

On a win or loss, the optional label is reparented beside the corresponding title
under the existing Result container. While playing it is parked under the same
Inactive results container as the inactive titles. The controller validates
required IDs, unknown and duplicate roles, live widget existence and widget kind.
A present prompt also requires its localization key. It never synthesizes text.
The objective becomes empty on the transition to won, even when the pickup count
has not changed. A fresh whole-session restart restores the initial objective and
parks the prompt. Runtime mutations remain typed native commands; authoring and
fixture changes use public atomic incant_cmd transactions with Undo/Redo/reopen.

The sample builder recognizes an explicitly authored `binding: "prompt"` label
using `hud.prompt`. Binding, observation and package checks admit exactly the
existing widget inventory plus that optional declaration. Unknown extra labels,
missing required roles, wrong kinds and a fixed-count-only false pass are refused.
The updated oracle observes the won transition independently of pickup events.

`samples/core/verify_result_hud.py prepare` makes two fresh owned fixture variants
from an existing approved scene: the unchanged 12-widget layout, and a 13-widget
correctness fixture that copies the existing controls label and translations.
It does not approve that fixture's appearance. Both controllers are freshly
compiled from the new source through public native Play. The separate `ship`
stage exports both ordinary packages and checks R/Start restart, fresh numeric
UI/parents and exact PCM after removing private project sources, maps and builds.
Historical controller and rendered evidence remain immutable and are not relabeled
as execution of this changed controller.

Validation completed on the safe 0065 base `8b82ad4d`; the rejected historical
`f517265a` is not an ancestor. Twenty focused Python tests and strict TypeScript
pass. A freshly built headless binary runs 28 successful native processes across
the two variants, with 396 numeric observation checks, four locales and actual
HUD/audio readback. Seven malformed binding/localization cases fail with exact
diagnostics and no PCM output; two additional native host restart refusals retain
the existing capability boundary. Public fixture transactions, Undo/Redo and
journal reopen pass without modifying the original 0065 project.

Two fresh ordinary shipping exports run ten processes each after their copied
authoring sources, source maps and private build storage are removed. All 426
shipping observation checks pass. Each variant covers win, loss, locked exit,
idle, a post-win tail, short loss and four R/Start/held-input reset routes. The
won objective is empty, prompt parents match the result state, and restart
restores the initial objective and inactive prompt. Complete gameplay records
match native execution; restart records and concatenated PCM match fresh games
exactly. Both exports audit the selected runtime dependency graph and actual
shipping compiler/physics artifacts. All 1,808 frozen engine inputs and all
nine source/probe inputs stayed unchanged through the run.

The initial attempt to reuse the retained 0065 headless binary failed before any
gameplay tick because its embedded development checkout no longer had pinned
Node dependencies. That refusal is retained. The successful proof uses a fresh
parent-checkout binary; no original child checkout or old controller evidence was
changed. The initial retention draft also remains outside Git; the committed
packet keeps relevant source, logs and receipts, with actual PCM, binaries and
captured assets hash-bound at their durable original paths.

[Evidence manifest](evidence/core-result-hud.json) retains 575 lossless records
in 291 deduplicated payloads and identifies 407 external artifacts. Independent
source and final producer execution review are accepted; current-main integration
is documented separately in [the main192 check](core-result-hud-main192.md).
The production prompt node/localization/style remains Claude-owned and unadded.
This is CPU correctness work, with no GPU/window, device, performance or phase gate.
