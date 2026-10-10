# CI scope selection proposal

No coverage selection is implemented by this proposal. Runtime/build changes retain every currently applicable check. The first measured warm PR has nine applicable contexts (general macOS, two desktop engine jobs and six platform jobs); the existing credential/physics path filters correctly exclude unchanged sources. The completed PR #51 Windows critical path is 21m19s execution plus 1s queue, versus cold PR #49 34m21s. [Committed warm evidence](evidence/shared-cache-warm-pr51-2026-10-10.json) keeps all queue, cache and test details separate.

## First small change: strict prose-only PRs

Keep `Incant checks / checks` always started and preserve its check name. Add a conservative classifier of the complete base-to-head diff. Only known root documentation and `docs/**/*.md` qualify for prose-only coverage. Retain generated-convention checks, links/reference validation and an explicit coverage summary; guard the expensive UI/Rust/tool/platform build steps for this scope. Every mixed or unknown path selects the existing full job. Main/scheduled/dispatch producers retain their existing behavior; only PR execution is selected.

Desktop/platform/credential/physics workflows already omit prose-only PRs through their current path filters, so this first change need not create a new required context, alter those filters or move the native job graph. It removes the general job's remaining unrelated Rust work for strict prose PRs. A minimal implementation should be reviewed in report-only mode first, compared with actual changed files, then enable the guards after observable classification tests pass.

Do not allowlist all of `docs/**`: JSON physics references/evidence can drive real tests. `docs/conventions` changes still need generated-instruction validation. Rust doc comments live in `.rs` and require normal Rust testing. Image/assets, scripts, arbitrary handoffs and unknown documentation extensions take the conservative full path.

## UI-only scope

Strict UI source/tests/styles under `editor/ui` can retain UI tests/build/type checks and generated bridge/schema validation while avoiding unrelated full workspace integration binaries. Mobile and credential workflows already exclude most such source-only changes. Keep real native desktop packaging/integration: Tauri embeds `editor/ui/dist`, and native code embeds the generated bridge JavaScript. Mock UI tests alone cannot certify that boundary.

Before enabling this scope, inventory the current native/UI integration proof and preserve it explicitly. Continue the existing workspace-selected build commands for the needed editor/headless tools; changing to package-scoped Cargo selection recreates the dependency-feature mismatch already removed. Do not invent new cache families without measuring their footprint. The proposal is to avoid unrelated test binaries, not to claim the native package can skip compilation or that visual review is complete.

Escalate UI plus any bridge, schema, SDK, Tauri config, brand/package asset, Cargo/npm manifest/lock, `.cargo`, toolchain, workflow or classifier change to current full coverage. Mixed paths take the union. Runtime/editor Rust changes remain full. The exact UI allowlist and required native proof should be a separate reviewed change after the prose classifier; do not combine an unmeasured UI coverage reduction with the small tool-cache optimization.

## Correctness requirements

- Read the complete Git diff from a fetched base/merge-base to the pinned PR head. Renames inspect both old and new paths; deletions and unknown paths are conservative. Diff errors, absent/truncated data and classifier changes select full coverage or fail explicitly.
- Keep existing required job/context names. Prefer lightweight explicit not-applicable execution inside an always-started context over adding workflow-level path filters that could strand a required check. Explain the selected scope in the job summary.
- Any aggregate coverage gate must fail for a selected job that fails, cancels, unexpectedly skips or is missing. Never treat an empty/skipped dependency set as proof that native checks ran. Validate actual matrix/skip behavior before adopting such a gate.
- Test docs+Rust, UI+bridge, nested dependency files, deletions, cross-scope renames, JSON physics references, malformed diff and unknown paths. Show full coverage for each representative runtime change, not just the happy docs/UI cases.
- Keep default-branch/scheduled cache producers independent of PR classification. Source-only PRs after long idle/eviction can be cold until a producer completes. Recovery must remain a finite, coordinated run, not duplicate dispatches.

Current GitHub protection reads returned no required-status branch protection or effective branch rules. Repository AGENTS review/check requirements still apply; this is not authorization to weaken coverage or change protection settings.

## Remaining native Windows latency

Path selection cannot accelerate a native runtime PR that correctly selects the full engine checks. The completed warm PR #51 Windows job spent 9m56s in Cargo's workspace build phase (20 workspace and 22 other compile events), approximately 85s on normal test execution, 147s on GPU verification, 88s on the custom-protocol tool build and 99s on the fresh shipping proof. The other compile events include the vendored `rerecast`, four newly added Rayon/crossbeam crates and 17 existing registry crates. The inherited feature/dependency delta explains the new graph, but ordinary logs do not expose each fingerprint invalidation reason or separate compiler/backend/linker time. Do not attribute every event to one cause without more data.

The next Windows investigation should record Cargo compilation timings and repeated integration-test link work on a completed compatible-cache PR before changing test layout or compiler configuration. The remaining shipping build deliberately verifies another profile and fresh symbols; reusing ordinary release artifacts invalidates that proof. The reviewed pinned CLI reuse removes measured 79s web and 30s Android installation work without touching engine/GPU/credential/shipping coverage, but does not shorten the Windows critical path by itself.
