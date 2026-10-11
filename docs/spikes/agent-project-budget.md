# Per-project agent budget accounting

The ledger, typed user-authored project cap and durable Session controller are
connected to native editor admission and reads. Usage is restored before a turn
can start and settled before its terminal result is published. Budget controls
and fresh live/native reopen acceptance remain in progress; this does not yet
complete the user-facing feature.

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

## Native integration

Native reads and starts offload filesystem work to blocking workers. The ledger
is opened before the short admission and authoring locks. The current project ID
and cap remain guarded through the in-memory Session installation or admission;
provider work and storage writes do not run under those locks. The worker checks
the project and cap again before account access and around provider/tool work.

A cap change cancels the observed active turn, including waking an approval wait.
Cancellation is pinned to that turn ID so a delayed read cannot cancel a newer
turn accepted under a new cap. A response already in flight remains charged;
changes to the cap do not authorize subsequent tool edits. Idle restoration is
version-stable. A competing process receives an explicit busy result rather than
a fresh allowance. Account changes and authoring Undo cannot reset consumption.

The integrated CPU checks pass 152 Rust tests (one existing native GPU test is
explicitly ignored) and four-package all-target Clippy. The worker regression
uses a synthetic provider and actual Session, owner and ledger: a cap change
before its call charges zero; a cap change inside the response charges 17 tokens
and refuses the revision-valid returned patch.

Independent source reviews found and closed the delayed-cancellation and
check/install races. The acceptance harness now compares cumulative consumption
after Undo with the exact measured turn sum, then checks that a separate editor
process restores the same usage and uncertainty without a provider call. The
updated live harness has not yet run; earlier live evidence predates this ledger.

[Native integration evidence](evidence/native-agent-project-budget.json).
[Durable Session evidence](durable-agent-session.md).
[Typed project cap evidence](project-agent-budget-settings.md).

## Remaining acceptance

- Complete the bridge command and Claude-owned budget controls, then verify the
  integrated native UI and fresh-process live-account budget recovery.
- Verify monetary estimates against a pricing source and billing-mode semantics;
  token accounting must not be presented as a fabricated dollar estimate.
- Hosted platform checks and the broader agent-quality/phase gates remain open.

[Ledger foundation evidence](evidence/agent-project-budget-ledger.json).
