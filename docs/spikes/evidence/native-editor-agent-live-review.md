# Native editor agent numerical acceptance harness — independent review

Reviewed read-only on root `57837a525e69257deb7d2f39d190ceca2224297d` plus the four-file opt-in harness diff in `/Users/andreyyurlov/.codex/worktrees/native-agent-play/incant`. Exact SHA-256 inventory: `/tmp/incant-native-agent-probe-reviewed-sources.json`. No builds, live-provider calls, GPU, window interaction, screenshots or source edits performed by this reviewer.

## Correction closure

The earlier P2 recovery-evidence finding is resolved in the current source: the live post-Undo and fresh-open receipts include revision, applied count, redo count and ordered transaction IDs; the wrapper requires exactly two redo entries and identical recovered summaries. A loader ignoring the journal now fails. Restore-only no longer independently asserts recovery. This remains a fixture-specific numerical test, not general recovery conformance.

The iterator compile correction is now inspected: `journal_state` uses `bus.redo_history().count()`, matching the public iterator contract. The three other input hashes are unchanged; no behavioral redesign or further finding. Parent retains the earlier failed compilation log.

Positive measured `charged_tokens` is now required on each completed live turn, alongside the pre-existing exact-accounting checks.

## Known correction inspected

The earlier camera mismatch is resolved in the reviewed source: stage 2 now requires exact fixture Camera `00000000000000000000000003` in both the prompt and approval predicate, with its positive allowlist regression. The reused author helper creates that camera; native `PreparedPlay` correctly refuses spatial content without explicit camera selection. No weakening of that native startup boundary is needed.

## Other checks

- Feature/module/start hooks are opt-in; the normal editor path is unchanged. The coexistence guard prevents the Play probe parser from consuming the agent probe arguments.
- Always-approval is narrowed to the disposable fixture's exact memory update, bounded Play request and retained tick query. Extra mutation operations, input scripts, screenshots and turn-4 replacement Play are refused. Existing typed dispatch remains authoritative for argument validation.
- Actual completion requires physical worker exit, successful required tool calls, exact known accounting and observed approval. Live stream receipt is limited to text observed before physical completion; it is not a model-quality or WebView dispatch claim.
- The cross-turn state query cannot succeed against an absent native session, so it exercises the persistent host fix. Failed-candidate preservation is separately covered by the persistent-owner unit/integration tests, not by this four-turn live harness.
- Play preservation compares the complete authoring document/revision/history tuple. Both edits must increment revisions and history as expected, carry agent ULID provenance, and both actual Undo commands restore the initial document.
- Safe receipts omit model text, tool argument documents, raw provider errors and account metadata. The wrapper does not write or forward captured native/provider stdout/stderr. Report summaries retain only bounded tool/camera/transaction identifiers and numerical usage.
- Failure cancels the active turn, and the probe shuts down the service before process exit. The external managed-process wrapper owns the timeout and descendant teardown. No engine-agent shell capability is introduced.

Outcome: no remaining behavioral blocker found after the camera/recovery corrections. Actual live/native execution remains pending and is not established by this source review.
