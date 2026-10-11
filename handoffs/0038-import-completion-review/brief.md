# Focused import completion review

Use Claude Opus 5.5 with thinking Max, explicitly selected and confirmed by the
ACP runner. Review the small completion-copy change in this frozen source; do
not redesign the editor. Astra owns transport/history correctness. Root has
independently reviewed the implementation and all local functional checks pass.

## Defect and corrected behavior

In your prior native0036 review, unchanged reimport incorrectly promised that
Undo would reverse it. Native commit actually appended no transaction, so Undo
reversed the preceding action. The new native receipt carries the commit revision
and optional transaction ID captured under the same lock as the mutation. Null
means unchanged. The later snapshot may include concurrent changes; never infer
the result from the snapshot alone. The UI now shows:

- Explicit unchanged receipt: “Already up to date. Nothing changed.” No Undo hint
  and no newly-imported marker. Both first-import and reimport requests can no-op.
- Changed receipt: existing Imported/Reimported message. Its Undo hint appears
  only while that exact transaction is currently applied atop history; undo or a
  newer transaction hides it, redo can restore it.
- A bridge without a receipt: “Import completed.” No assumed Undo transaction.
- Failures retain the existing validation/error, retry and focus behavior.

Review whether these states are plain, accurate and readable at normal1440×874
and minimum1000×650 windows and a280px Inspector. Drive the actual import/reimport
UI through account-free typed browser doubles, with realistic existing history.
Capture unchanged, changed, subsequent undo/newer edit, unknown receipt and one
failure. Verify live completion feedback and controls remain accessible and not
duplicated awkwardly. Use truthful labels: a browser double is not native command
execution. Native journal/Undo/Redo semantics have real Rust test coverage; this
packet is presentation review.

## Scope and safety

Read `docs/spikes/editor-import-receipts.md`, the new evidence JSON and
`editor/ui/DESIGN.md`. Preserve the director's completed plain design, compact
account dialog, avatar-only control, searchable shortcuts and uncluttered
Inspector. No decorative additions or broad cleanup.

Use headless browser-only capture. Another Claude session owns a separately
scoped native window on this Mac: do not launch/focus/type into/capture/close any
native app or use system-wide input. No Cargo or GPU benchmarks. No credentials,
accounts or network-backed services. Use only temporary account-free fixtures.

You may add a focused capture helper, screenshots, result and presentation tests
inside this packet. If wording has a concrete problem, you may narrowly adjust
only completion text in `AssetsContext.tsx`/`AssetInspector.tsx` and matching UI
assertions; do not change receipt conditions, history semantics, Rust, bridge,
Cargo, CI or styles. Report other functional findings for Astra. No new runtime
dependencies. Run full UI tests, strict TypeScript and UI build if source changes.

Existing helpers in handoff0035 may be reused, but do not rerun its entire review.
Use installed Playwright/Chrome and the real current UI build. Record commands,
observed states, exact source and screenshot hashes, limitations and model/effort
in `handoffs/0038-import-completion-review/result.md`. Commit final evidence with
`Built-by: claude`. Do not publish, push or merge.
