# Native headless UI routing

`native-play --camera ID --ui-viewport WIDTHxHEIGHT` and the native Play RPCs
accept the existing recorded UI/viewport data. `play.run` uses `ui_viewport` plus
`inputs_script` (the existing incant-input v1/v2 recording); `play.start` accepts
`ui_viewport`, and `play.step` accepts optional `ui` and `viewport` alongside its
existing physical `events`. Dimensions are finite logical pixels in 1..=65536.
The camera remains explicit. Omission keeps the previous non-UI startup contract.

PreparedPlay transfers the UI settings to the same retained native render host.
PlaySession preserves those settings when replacing only the consumer settings.
Both the headless Session and ordinary game player call the same runtime-only
recorded-UI resolver. There is no alternate simulation, input format, authoring
mutation or per-tick source parsing/JSON conversion in this resolver.

Timeline commands resolve against the current retained owner. Commit, preedit
and cancel require the named widget to be focused before this tick. Focus in the
same packet does not authorize a later text command. Text/count/range/identity
bounds are checked before cloning text or changing presentation. A viewport
change is the existing separate presentation transaction; pointer tokens are
resolved against its accepted layout, while invalid focus-target records refuse
before it. Platform receipts keep their separate stale-token policy.

The ordinary UI router filters UI-owned keys before gameplay receives input.
The complete UI packet is staged with the native tick; guest failure terminates
that candidate without exposing partial UI/world/audio state. `play.run` publishes
only a completed candidate, so failed runs preserve the previously completed
session. This change does not make native `play.step` failures retryable after
an accepted guest tick.

Validation passes actual CLI, play.run and incremental play.step parity
for focus, preedit, cancel, commit, resize and filtered Enter; unchanged authored
project/source/journal bytes; failed-candidate preservation; and no-output CLI
refusals. Shared host tests cover resized pointer routing, raw typed packet
limits and accepted input preservation. The affected suites pass 100 tests in
27 groups, with no failures or ignored cases; six-package all-target Clippy,
the input wasm32 build, normal-runtime dependency guard, formatting and generated
contracts pass. Seventy fresh schemas match the checkout; only the tool schema
changes. Independent source and final execution reviews are accepted. The
[evidence packet](evidence/native-headless-ui.json) retains the original fixture
failures, corrected runs and both review receipts. Publisher intake remains pending.

The mode validates and observes canonical CPU input. It does not open a window,
submit GPU frames, exercise platform IME callbacks or establish appearance/device
performance acceptance. Existing checkpoint and unsupported-host limitations
remain unchanged. Editor UI styling and automatic viewport selection are outside
this slice.
