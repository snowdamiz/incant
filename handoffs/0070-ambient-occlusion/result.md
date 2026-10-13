# 0070 result — combined SSAO + bloom: preparation revision 2

## Status

**The revision 2 runner corrections are ready for root's source review.
Nothing was executed on a GPU.**

This session was preparation only. Nothing was built with Cargo. No target, GPU
process, window, render, capture or grant was used. **I make no efficacy,
appearance or visual-success claim.** No screenshots exist.

- **Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort
  **max**, as configured and verified by root before this prompt. I cannot
  inspect the effort setting from inside the session.
- **Previous result preserved:** it is kept verbatim as
  `preparation-result-v1.md` (commit `767827b4`, byte-identical to `result.md`
  at `2a6d884b`). It still describes the case plan, appearance set and
  observation limits, which have not changed.
- **Scope:** I fixed only root's four defects, and did no new source survey.
  The Rust helper is unchanged; root reviews it separately.
- **Blockers:** none for source review. The decisions for root are at the end.

## Root review revision 2: what changed

### 1. Shared native GPU lock

All device commands take the shared lock: `list`, `suite`, `case`,
`appearance`, `retry` and `release`.

**Order.** Each command:

1. takes `~/.codex/incant-evidence/native-gpu.lock` with `flock(2)`
   `LOCK_EX|LOCK_NB`;
2. then loads the grant;
3. then takes this packet's `runner.lock`;
4. holds both through preflight, reservation, launch, kill and wait,
   completion and unlock (`runner.device_session`).

A busy lock refuses at once. Nothing is reserved, and the packet lock is never
opened.

**Release by close only.** Locks are released by closing the descriptor,
never by `LOCK_UN`. Every device child inherits the shared-lock descriptor.

- An orphan that outlives a SIGKILLed runner therefore still holds the lock,
  so no other packet can render beside it.
- Verified with real processes: the orphan keeps the lock, and the lock frees
  once the orphan is gone.

**Two-packet control.**

- Packet A's child is running. Packet B's `list` refuses with "shared native
  GPU lock", and B's evidence directory has no `ledger.jsonl` and no
  `runner.lock`: it refused before its own lock and before any reservation.
- After A completes, B runs.
- A plain process holding the lock, as another tool would, also makes A
  refuse with no new ledger record.

**The real lock file already exists.** It is 0 bytes, mode 0644, created at
00:59 by another tool. I only listed it with `ls`; every control uses a
scratch copy. On scratch files on this Mac, `flock` and POSIX `lockf` exclude
each other in both directions. So the exclusion holds whichever of the two
that tool uses.

**Scope.** The lock is held per command. Packets can interleave between
commands but never render concurrently.

### 2. Failure latch

Every completion now records `success` and `failure_reasons`. Success requires
all of the following:

- the spawn gate opened;
- exit 0, with no signal, timeout or interruption;
- the group verified absent, with no survivors and the leader reaped;
- no cleanup or recording error;
- the frozen binary unchanged;
- a valid verdict for the child's kind.

**Verdict validity by kind:**

| Kind | Valid only when |
|---|---|
| `listing` | Exact name set, no duplicates, and the libtest count line equals the plan |
| `suite` | Every planned test ran exactly once and reported `ok`, with no extras; `running N tests` and a single `test result: ok. N passed; 0 failed; 0 ignored` summary match |
| helper (`case`, `appearance`) | Well-formed receipts for this case only, with no malformed lines; exactly one budget receipt (declared equals plan, 0 < frames ≤ declared, labels unique and equal to the frame count); exactly one verdict (no failures, a non-empty list of checks, all ok); `index.json` lists exactly the kept files, with matching hashes, byte lengths and PNG count, all within 960×540 |

**Effect of a failure.** Any other outcome latches the packet. A child that
never completed is a failure too. While latched, `gpu_checks` refuses all
device work except a **precise retry**.

**A precise retry** needs a root receipt, `incant-0070-retry-v2`, placed
directly in the evidence directory. The receipt must:

- carry `approved_by: root`, the grant's `source_commit` and a reason;
- name a latched `original_label`, with the same kind and target as that
  child;
- for a suite, list a non-empty, duplicate-free subset of that child's failed
  or missing tests.

**Retry limits:**

- one retry per latched child;
- each receipt is usable once;
- at most 2 retries in total.

**A passing retry does not unlatch** other work; that is decision 1.

**Exit codes:** 0 for success, 1 when a child failed (latched), 2 when
refused.

### 3. Ownership by exact identity

**Spawn gate.**

- The leader first runs this interpreter (`-I -S -B`), blocked on a pipe.
- While it is alive, the runner reads its exact identity with `libproc`: pid,
  kernel start time to the microsecond, and uid. `exec` keeps both the pid and
  the start time.
- The runner records the identity durably, then sends the exact constructed
  environment through the pipe.
- If the runner dies first, the trampoline exits 125 and the program never
  executes.
- A control runs `/usr/bin/env` through the gate and checks that it receives
  exactly the constructed keys and values. A plain inheriting trampoline
  would have added `__CF_USER_TEXT_ENCODING`; that is why the environment
  travels through the pipe.

**Live cleanup** never signals after a reap.

- Exit is observed with `waitid(WNOWAIT)`. While the leader stays unreaped,
  its PID, and therefore the PGID, cannot be reused.
- Cleanup sends SIGTERM, waits the grace period, sends SIGKILL, waits until
  no other member remains, records the survivors' exact identities, and only
  then reaps the leader.
- Members seen while the group runs are recorded durably as `member` records.

**Reconcile** may signal a group only while a live member matches a retained
exact identity.

- Retained identities come from `spawned`, `member`, completion and earlier
  reconcile records.
- Once a member is proven, the other members belong to the same session
  lineage and are adopted. A group cannot be recycled while a proven member
  is alive and in it.
- Otherwise reconcile signals nothing, records the unproven identities, exits
  2, and the label stays blocked for root.
- The old start-time heuristic is gone. So is the old killing of processes by
  frozen-binary path; `release` still refuses while one runs.

**Blocking.** A label is blocked until its group is verified absent: by a
completion that is absent with no survivors, or by a reconcile with
`resolved: true`. Every child, retries included, refuses while anything is
blocked. A failed reconciliation or remaining survivors therefore never
unblock anything.

**Controls:**

- **Recycled PGID:** a live, unrelated group whose members all started after
  the reservation, with a recorded start one microsecond off. Reconcile
  refuses, the process is still alive afterwards, and `case` and `release`
  stay refused. After that process is gone, reconcile resolves without any
  signal, and the latch remains.
- **Survivors:** a real member left in its reaped leader's group blocks
  everything. An unproven reconcile signals nothing and still blocks. Once
  the member's exact identity is recorded, reconcile stops it.

### 4. Budget

The existing-regression ceiling is now **8,192 frames (combined 8,320)**. The
other limits are unchanged:

- 128 new frames;
- 32 PNGs;
- 16 serial children;
- 45 minutes.

The wide sprite test is back in the plan at its counted 6,264 frames, and the
deferral mechanism (`DEFERRED` and `allow_deferred_tests`) is removed.
`make_plan.py` now refuses unless every compiled ignored test runs exactly
once.

| Suite | Tests run | Existing frames | New frames |
|---|---|---|---|
| lib | 37 | 425 | 20 (the four new tests, emitter-position once) |
| model_gpu | 50 | 7,595 (1,331 + wide 6,264) | 0 |
| gpu_host-game | 5 | 40 | 0 |
| gpu_host-ui | 2 | 32 | 0 |
| **Total** | **94** | **8,092** of 8,192 (100 unallocated) | 20 |

**New frames** total exactly 128:

| Part | Frames |
|---|---|
| New tests in the lib suite | 20 |
| Numeric cases | 73 |
| Appearance | 16 |
| Retry reserve | 19 |

**Watchdogs are unchanged** and sum to 2,370 s, inside the 2,700 s limit.
The `model_gpu` watchdog stays at 300 s. The same 50 ignored `model_gpu`
tests, wide test included, finished in 49.43 s and 49.05 s under profile
`ci`. That is about a 6× margin. The measurements come from the
`evidence/test-render-gpu.txt` logs of handoffs 0054 and 0055. They are a
calibration reference, not a measurement of this union.

**No new permission:** this budget correction grants no GPU or target access.

### Root's other notes, applied

- **Half resolution:** this is accepted as canonical SSAO; no plan change.
  `math.rs` gives the AO full-resolution attachments of the *internal* target,
  so at 50% DRS the AO is half resolution and expands with the frame. That is
  exactly what `lifecycle_drs` checks against internal-size references. I
  make no full-resolution AO quality claim.
- **Oversize refusal:** it stays covered by the CPU tests only.
- **Cargo config inspection:** unchanged.
- **New-frame cap and stop-on-failure:** the 128 cap is kept, and the latch
  implements stop-on-failure.

## Identities to pin in the grants

| Item | Value |
|---|---|
| HEAD / tree | `68cfcbc42dc06d961ff20aec7d0951ed8f078a0a` / `214f54b05151e328a6b912982e330e36eb94970c` (this result commit adds only `result.md`, which does not change the helper digest or plan; re-read HEAD before granting) |
| Engine inputs | Byte-identical to `b633f92d`: `git diff` over `crates Cargo.toml Cargo.lock rust-toolchain.toml .cargo third_party tools/cargo` is empty |
| Helper digest | `4f6ea9bb8f47ffe3721c333d24fa7461c375748f0169432792d8746e977609e6` (19 files under `tools/`, now including `mutations.py`) |
| `tools/plan.json` sha256 | `27ea7b4425100c8ec6786c02cc8da21b41c670ec2ff3824ed6c497d9ccf859cf` (format `incant-0070-plan-v2`; reproduced by `make_plan.py --check`) |
| Repository `.cargo/config.toml` | `0d3b42ee7bc4ef237cb8af440c47e506dcc9cc8b24d49d3cd3d3fea76ae5caa1` (unchanged) |
| Toolchain | `rustc` 1.99.0 from `rust-toolchain.toml`; other releases refuse |

## Checks run this session (CPU only)

Commands ran from the worktree root at `68cfcbc4`. `python3` means
`/opt/homebrew/bin/python3 -I -B` (3.14.8), and `tools/` means
`handoffs/0070-ambient-occlusion/tools/`.

| Command | Result |
|---|---|
| `python3 tools/listing.py` | lib 127 / 37, model_gpu 51 / 50, gpu_host-game 6 / 5, gpu_host-ui 3 / 2: **94 ignored** |
| `python3 tools/make_plan.py --check` | `plan.json matches the source-derived plan` |
| `python3 tools/selftest.py` | **26 of 26 controls pass**, exit 0 (19 unit, 7 end-to-end). It also passed 26 of 26 at `c9ec91b1`; the unit run passed 19 of 19 before that |
| `python3 tools/mutations.py` | **4 of 4 re-introduced defects caught**, exit 0 (listed below) |
| `python3 tools/ssao_reference.py --selftest` | Unchanged: flat [0, 0], near slab [5, 5], reversed normal [0, 0] in both projections |
| `python3 tools/analyze.py --selftest` | Unchanged: 43,489 pixels agree per projection; 8,558 / 7,075 occluded; 457 disagreements per projection under the +2-ray perturbation |
| `/opt/homebrew/opt/rustup/bin/rustfmt --edition 2024 --check tools/effects/src/lib.rs tools/effects/src/main.rs` | Exit 0 (no Cargo) |
| `python3 -m py_compile` on the nine helper scripts | All compile. It wrote a gitignored `tools/__pycache__`, which I deleted |

**The 26 controls:**

- **Unit:**
  - porcelain parsing;
  - environment allowlists;
  - toolchain pin;
  - the ceilings 8192/128/8320;
  - blocked and latched state;
  - packet and shared lock exclusivity;
  - exact identity, including zombies;
  - an exact environment through the gate;
  - timeout;
  - SIGTERM escalation;
  - a grandchild killed before the reap;
  - an `on_spawn` failure: the program never runs;
  - SIGINT and SIGTERM to the runner;
  - runner SIGKILL: the orphan is the recorded process and keeps the lock;
  - runner SIGKILL before the gate: never executes;
  - grant refusals;
  - verdict classification: 10 outcome flaws, 5 completion flaws, 7 libtest,
    4 listing and 14 helper-output variants;
  - frozen-binary detection.
- **End-to-end** (real runner commands, stand-in binaries, seven independent
  evidence directories):
  - main flow;
  - two packets;
  - missing-verdict latch with eight imprecise receipts;
  - suite retry precision;
  - runner SIGKILL, then reconcile and release;
  - recycled PGID;
  - survivors.

**The four mutations.** Each is injected into the harness process only, and
each made its control fail:

| Mutation | Caught by |
|---|---|
| Per-packet lock instead of the shared lock | `e2e_two_packets` |
| Failures never latch | `e2e_latch` |
| Ownership by a start time within one second | `e2e_recycled`: "an unrelated group was signalled" |
| Survivors or a failed reconcile unblock | `e2e_survivors` |

**Hygiene:**

- No process from any control or mutation survived (checked with `ps`).
- All scratch directories were removed.
- Every write stayed inside the worktree.
- The probes that informed the design ran on scratch files inside the packet
  directory, which I deleted. They covered the `libproc` layout, `waitid`
  WNOWAIT, the gate environment, and `flock` versus `lockf`.

## Limitations and residual risks

- **Runner SIGKILL stays uncatchable.** Its effects are bounded:
  - The orphan keeps the shared lock, its identity is recorded, and
    everything refuses until reconcile proves and stops it.
  - A SIGKILL before the gate opens means the program never runs.
- **Residual races**, documented rather than solved:
  - **Proof-to-signal window:** reconcile signals in the microseconds after
    its proof. A recycled number would need the sequential PID allocator to
    wrap to the same number in that window.
  - **Escaped processes:** a process that leaves the group with
    `setpgid`/`setsid` escapes group signalling and the group check. While it
    keeps the inherited shared-lock descriptor, every device command refuses.
    If it was observed (membership is polled every 0.5 s), `release` also
    refuses while it lives. One that was never observed and closes the
    descriptor is invisible to the runner.
- **Platform:** macOS only (`libproc` via `ctypes`, `waitid(WNOWAIT)`).
  Elsewhere it refuses before reserving. The leader starts as Python, so each
  child pays one short interpreter start; the whole `normal` control,
  including the gate, took 0.04 s.
- **The 45-minute ceiling is wall-clock from the first child**, so idle time
  between commands counts (decision 4).
- **Retry budget:** a retry must fit the remaining budget.
  - An existing test fits only if it is at most 100 frames.
  - `ssao_core` (20 frames) fits within the 19-frame reserve only if some
    planned child did not run.
- **Still never executed:**
  - The runner's `build` path: its Cargo steps, binary freezing, feature
    extraction and the real Cargo config audit run first under a build
    grant.
  - The Rust helper has never been compiled or run through Clippy.
- **Carried over from v1 unchanged:**
  - the raw AO attachment is not publicly observable, so it is inferred from
    readbacks against the f64 reference;
  - emission is capped at 1;
  - only Metal is expected, with no phone or performance claim;
  - temporal upscaling, MetalFX, GPU timestamps, cooked SSAO and bloom,
    native default exposure, profiling subpass entries, device budgets and
    complete Phase 1 rendering all stay pending.

## Proposed check list for root

1. **CPU, no grant:** the commands above, including `selftest.py` and
   `mutations.py`. Run them only with the helpers committed.
2. **Under a build grant** (`incant-0070-build-grant-v1`):
   `runner.py build --grant <evidence>/build-grant.json`. This step includes
   helper Clippy with `-D warnings`, the toolchain pin, the Cargo config
   audit and the feature identity. Then run `runner.py release-build`.
3. **Under a GPU grant** (`incant-0070-gpu-grant-v1`), in this order:
   1. `list` four times;
   2. `suite` four times;
   3. `case` five times;
   4. `appearance`;
   5. `release`;
   6. then `analyze.py <evidence>`, followed by my appearance review.

   Any failure stops the run until root decides. If a run is lost, use
   `reconcile --grant <grant>`; it accepts the build or the GPU grant.

Grant and retry receipt shapes:

```json
{"format": "incant-0070-gpu-grant-v1", "packet": "0070-ambient-occlusion", "kind": "gpu",
 "source_commit": "<HEAD>", "plan_sha256": "27ea7b44…", "helper_digest": "4f6ea9bb…",
 "evidence_dir": "/Users/andreyyurlov/.codex/incant-evidence/0070", "build_record_sha256": "<build.json>",
 "expected_backend": "Metal"}
{"format": "incant-0070-retry-v2", "packet": "0070-ambient-occlusion", "approved_by": "root",
 "source_commit": "<HEAD>", "original_label": "suite-lib", "kind": "suite", "target": "lib",
 "tests": ["<exact failed test>"], "reason": "<why>"}
```

## Decisions for root

1. **Latch strictness:** a passing precise retry does not unlatch other
   children. Continuing after any failure needs a packet revision, because
   the runner has no resume switch. Confirm, or request a root resume
   receipt.
2. **Reconcile lock:** reconcile takes only the packet lock, because its
   orphan may hold the inherited shared lock. It starts no program. Confirm.
3. **Build lock:** the build lease does not take the shared GPU lock, since
   it renders nothing. Confirm.
4. **Ceiling clock:** the 45-minute ceiling counts wall-clock time from the
   first child, idle time included. Confirm, or switch to summed child time.
5. **Child count:** the four `--list` launches count toward the 16 children.
   This was v1's B2, still unanswered.

## Changed paths

All under `handoffs/0070-ambient-occlusion/`:

- **New:**
  - `preparation-result-v1.md` (verbatim v1);
  - `tools/mutations.py`.
- **Rewritten:**
  - `result.md`;
  - `tools/ledger.py`, `tools/lifecycle.py`, `tools/runner.py`,
    `tools/selftest.py`.
- **Edited:**
  - `tools/common.py` (the shared-lock path);
  - `tools/make_plan.py`;
  - `tools/plan.json`.
- **Unchanged:** `listing.py`, `ssao_reference.py`, `analyze.py` and the Rust
  helper `tools/effects/`.

**Commits:** `767827b4`, `c9ec91b1` and `68cfcbc4`, plus this result, all
with `Built-by: claude`. Root's brief commit `41135eaa` was not touched. No
engine, repository manifest or CI file changed. Nothing was pushed or merged.
