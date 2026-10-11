# Per-project agent budget accounting

The usage ledger is a local authoring-service foundation. It is not yet connected
to native turn admission or budget controls, so the existing editor session
allowance still applies. The typed project cap is a separate command-system
change; completing these foundations does not complete the user-facing feature.

The ledger lives in the OS configuration directory under
`Incant/agent-usage/<project ULID>.json`, outside projects, build directories and
credentials. Its identity is the project ID, not the checkout, binary or selected
account. Project Undo/Redo must never undo consumption. It records only a format
version, project ID, cumulative charged tokens, uncertainty and a pending
reservation. It contains no prompts, model output, credentials or account data.

An exclusive OS lock belongs to one project and spans the entire lease. Opening
an already owned ledger fails promptly; another project remains available. Before
returning a lease, the ledger atomically writes and syncs a reservation covering
the remaining allowance. Normal completion replaces it with the caller's trusted
cumulative charge. A dropped lease, panic or process exit without a trustworthy
receipt retains the whole reservation, explicitly marked uncertain. Increasing
the cap allows further work without clearing that charge or prior uncertainty.

Caps are nonnegative JavaScript-safe integers; zero disables new turns. Measured
overruns remain charged. Usage beyond the representable limit exhausts every
possible cap and is marked uncertain. Invalid receipts cannot refund the
reservation. Damaged, oversized, foreign-project or unsupported records are
retained and refused instead of silently reset. Files and directories are private
on Unix; Windows uses the current user's configuration directory permissions.
Symlink destinations are refused. This is local storage, not a tamper-proof billing
system against a user who can modify their own files.

Thirteen ledger tests pass, including real fresh subprocesses that retain the
lock during a turn, finish normally and exit abruptly without running destructors.
All 94 `incant_agent` tests and all-target Clippy pass on macOS. Independent review
accepted the frozen implementation. The evidence binds exact source and log
hashes. These are local process-crash checks, not power-cut, hosted cross-platform,
live-provider billing or phase-gate evidence.

## Integration still required

- Read the user-controlled cap and stable project identity through the shared
  document; agents cannot raise their own cap.
- Reserve and complete usage on a worker around actual provider work, including
  panic, cancellation, failed admission and failed persistence. Do not perform
  filesystem or lock waits on the GUI thread.
- Restore usage before authorizing a turn, fence project/cap changes, publish
  cumulative consumption and uncertainty, and preserve physical single-flight
  ownership until the provider worker has actually stopped.
- Add Claude-owned budget controls and actual native reopen acceptance. Monetary
  estimates need a verified pricing source and billing-mode semantics; token
  accounting must not be presented as a fabricated dollar estimate.

[Implementation evidence](evidence/agent-project-budget-ledger.json).
