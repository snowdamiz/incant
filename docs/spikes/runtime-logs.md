# Structured game-script logs

Phase 1 increment, 2026-10-09. The sandbox SDK exposes
`api.log(message, level = "info")`; levels are debug, info, warn and error. This
returns structured data to the host without installing a console, filesystem,
network API or other host capability. Logging is output, not a document mutation.
Script commands still commit through the shared command bus.

Each successful script transaction publishes its messages in order. Failed
commands or script exceptions publish neither new script state nor messages.
Unread messages survive a successful hot reload and are consumed with take_logs.
The host rejects more than 64 messages per tick, 4096 UTF-8 bytes per message,
or 256 pending messages/256 KiB of pending message data before committing edits.

Both `incant script` and `incant play` return logs with fixed tick, simulation
time, level and message. Playback can additionally write a new JSONL file using
`--log-output`. This works without a GPU, or alongside PNG captures, including
inside the new capture directory. Existing files are never replaced. Capture
reserves report.json and frame-*.png names, case-insensitively, in that directory
to prevent output collisions. Names and parent directories are checked before
GPU startup; the selected log file is opened with create_new.

The runner validates each whole log batch against 10,000 records and 8 MiB of
encoded JSONL before publishing it. Embedded newlines stay escaped inside one
record. Earlier successful ticks remain in the log file if a later script fails;
no completed report is emitted. A disk failure may leave an incomplete final
JSONL record; readers should keep only complete records. Successful runs sync the
log file before returning and atomically publish their optional report.json.

## Verification

Three ScriptHost behavior tests check levels/order, hot-reload preservation,
consumption, pending/message limits and rollback on command/script failure.
Two CLI subprocess tests check log-only operation, timestamps, both runner
commands, authored/journal isolation, newline encoding, existing-file protection,
reserved-output rejection, retained prior output on failure, count limits and
encoded-byte limits. The real GPU playback test also checks a JSONL file next to
its PNGs and report, with matching records in the successful report.

Before material integration, all 122 workspace Rust behavior tests, all three
explicit GPU tests, workspace Clippy, SDK type checking, 282 UI/bridge tests and
UI build pass. A separate actual CLI run compiles the kinematic TypeScript
sample with the bundled SWC compiler and runs 120 ticks: 120 shared-bus commands,
messages at ticks 60 and 120, unchanged authored bytes, no GPU or account.
The integrated `832ae5d` tree passes 124 Rust behavior tests, all seven explicit
GPU tests and workspace Clippy. The UI tree is unchanged from its 282-test/build
verification. Hosted checks are pending.

This does not add a script exception debugger, stack traces, failed-tick logs,
engine-system telemetry or editor console wiring. Simulation timestamps are not
wall-clock latency measurements. No production performance gate is claimed.
