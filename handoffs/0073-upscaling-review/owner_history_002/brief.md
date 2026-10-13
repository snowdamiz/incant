# Corrected owner-history numeric and retained execution

Preparation only. Root must review this packet and issue a new grant before a supervised Claude0073 ACP turn. Verify **Claude Opus 5.5 and Max** explicitly before prompting. No authority is inherited from GPU001, which failed case 3 and remains unchanged. Do not modify the runner, plan, binaries, tests, shader, old reports, or existing dirty brief/untracked inputs.

The new producer is `fd5b86d03a2cabfcc9bd5a8441a137645fb858ae`. Freeze `608ad3628d1d576ef688eb2548c0a4593e1b28a1946c442e6eb5d7a955bcb6f7` binds 2,737 engine inputs and 24 copied sources. The source includes Claude's anchor-only changed-owner history correction and root's stronger case 3 current-only reference. The independent corrected CPU audit binds 213 records/119 payloads, ten successful checks, 33 cooker tests, 152 default CPU tests, and 167 cooked CPU tests. The two frozen copies are executable read-only mode 0555; leave their permissions unchanged.

The only execution command, **after a fresh root grant**, from this checkout is:

```sh
python3 -B handoffs/0073-upscaling-review/owner_history_002/runner.py run /Users/andreyyurlov/.codex/worktrees/native-agent-bridge/incant/.worktrees/0073-upscaling-review/handoffs/0073-upscaling-review/owner_history_002/plan.json
```

Before grant, `check` with the same absolute plan is read-only and verifies the immutable inputs. `grant.template.json` is not authority; root supplies issue time and the actual latest GPU release identity and saves `grant.json` in the fresh `~/.codex/incant-evidence/0073-owner-history-gpu-002` directory. No target directory or Cargo command is used.

The runner owns the nonblocking packet and global GPU locks. Exactly four fixed children are allowed, in this order, stopping on the first failure:

1. Default WGSL numeric test: twelve 16×16 cases, twelve queue submissions and twelve maps.
2. Cooked Metal numeric test: the same twelve cases, submissions, and maps.
3. Default WGSL retained test, only after both numerical children pass: twelve 32×32 renderer frames plus twelve packed readback submissions, twelve maps.
4. Cooked Metal retained test, only after all previous children pass: the same twelve frames and readbacks.

The cap is **four child attempts, 72 explicit queue submissions, 48 maps, 32×32 maximum extent, 90 seconds per child, zero images, zero builds, zero retries**. Numerical children each contain 12 depth-clear passes, 12 owner passes, one history-disabled owner reference pass for case 3, and two unchanged ordinary baseline passes for cases 7/8: 27 numerical render passes each. Retained frames contain the existing graph passes; do not describe 72 submissions as 72 render passes. Each attempted child is charged in full before launch and never refunded.

Both binaries enumerate exactly the required names in their full retained test lists. A child must exit 0, execute exactly one test, emit the correct Metal adapter, and reach its exact final marker. Both numeric children must report cases 1–12, ordinary baselines only for 7/8, and a current-only reference only for 3. Retained children must reach `RETAINED_OWNER frames=12 readback_submits=12 queue_submits=24 maps=12 max_extent=32x32 no_images=true`. The complete AdapterInfo must match across all children. Unsupported early returns, a missing marker, parse or validation failure, a mismatch, timeout, interruption, or process survivor fail closed.

The environment is a small allowlist: system PATH, fresh TMPDIR, Metal backend expectation, and backtrace disabled; HOME is inherited only for native driver operation and its value is not logged. No ambient Incant/WGPU/Rust/Cargo flags are forwarded. Stdout/stderr go to regular retained files. Every child has a process group; success, failure, timeout, SIGINT/TERM/HUP, and other exceptions preserve the original error and kill/reap descendants before release. The runner refuses a used evidence directory. After completion, verify ledger/debits/release hashes and that all child PIDs/groups and ACP descendants are gone; preserve failure and stop without changing expectations.

Commit only new result files in this folder with `Built-by: claude`. Report the exact source, binary, adapter, raw output, debit and release identities; separate attempted budgets from reached markers. Do not touch the parent or child dirty `brief.md`, the three original untracked owner-attachment inputs, historical runner/result/diagnosis, or root engine checkout.

This is numeric kernel/backend and retained host-connection evidence. The retained fixture checks submitted color/depth/record ownership, refused-frame isolation, geometry and actual light-transform change headers, camera/full-extent transitions, four DRS changes, epoch cut, and owner toggling. It does not establish acceptable appearance or general stale-color quality. At the quarter-size 8×8 internal extent, the thin-quad fixture can correctly produce no reusable records; all canonical/finite/depth checks remain mandatory. Existing visual rejection remains open. Future mutable SSAO/effect composition must invalidate static persistence; this private baseline has no such effect setters. Logical caps do not bound queued driver or vendor memory.
