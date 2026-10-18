# Reuse checked font engines for compatible HUD updates

Text-only gameplay HUD updates previously reconstructed every font-shaping engine
when the UI generation changed. The native UI binding now retains its checked
font catalog and shaping engines if every requested ordered fallback chain is
already present. Text, focus, layout and disabled-state changes continue through
the existing prepared UI reconciliation and atomic publication path.

A new or reordered fallback chain still builds a separate candidate with the
existing font validation and bounds. It publishes only with the accepted UI
state. Existing immutable shaped text remains valid if preparation fails; only
private scratch/cache state can change. Unused retained chains stay within the
original 32-chain bound. Packaged font replacement is not introduced here.

## Qualification

On main405 base `0063b97c`, 48 `incant_ui` tests and 20 composed native UI host
tests pass in the CI profile. The new controls compare multilingual glyphs,
lines, extents and missing-cluster results against fresh engines, retain accepted
shaped-text identity, and exercise reordered, missing, invalid and over-limit
candidates. Existing host controls cover HUD edits while typing, IME, focus,
reparenting, recorded input, failed updates and complete-tree removal.

Both packages pass strict all-target Clippy; all ten local development commands
pass, including formatting, generated contracts, tool tests and the editor build
and tests. An independent
source review and a separate test-generic delta review found no remaining blocker.
The first test attempt failed to compile because its helper used the document's
default string-ID text type; the corrected helper uses runtime stable IDs. That
failed attempt remains in the [qualification evidence](evidence/ui-font-reuse/qualification.json).

Exact test commands use the pinned Rust 1.99 toolchain, locked/offline resolution,
`--profile ci -j2`, `profile.ci.package.incant_render.opt-level=2` and
`profile.ci.package.sha2.opt-level=3`:

- `cargo test -p incant_ui --tests`
- `cargo test -p incant_render_game --test ui`
- `cargo clippy -p incant_ui -p incant_render_game --all-targets -- -D warnings`

This removes repeated font-engine construction on the compatible path. It does
not establish a shipping frame-time saving, resolve the ScriptTick budget, or
claim native visual acceptance. The long HUD-update frames observed in a
separate scheduler experiment motivated the change, but their causal attribution
and a matched shipping measurement remain open. Computer use and screen capture
are currently paused by the director; no native appearance check was performed
for this change.
