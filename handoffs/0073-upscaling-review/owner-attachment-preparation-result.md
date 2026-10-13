# 0073 owner attachment — preparation result

**Status: PREPARED, CPU only.**
- No GPU authority exists, and none was used.
- Nothing ran the ignored test.
- There was no build, window, image or appearance action, and no engine source
  was changed.
- The preparation stops here. Root reviews the runner and alone schedules any
  GPU run.

**Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max.
Root's ACP start record `temporal-owner-attachment-preparation-001/acp-start.json`
reads `"model": "Opus 5.5", "effort": "max", "verified": true,
"gpu_authorized": false`. Its `acp.log` header reads `Claude session verified:
Opus 5.5; thinking: max`. No subagents, memory writes, pushes or credentials.

## Inputs (root's, read only, left uncommitted)

| File | SHA-256 |
|---|---|
| `owner-attachment-prep.md` | `0332bad7a8a627b01b925474e94824cf462bc0c450d8c2718334463872390d9e` |
| `owner-attachment-plan.json` | `d7da3f1ccbe73b18a0f6d90f7da9d6c4ac586be351b04541afe7ae619c6b6512` |
| `owner-attachment-cpu-enumeration.json` | `68c5434e926678071f4291896f6897137b4010b3885ab8043db7d9785136cfba` |
| `brief.md` (dirty, unstaged, unchanged) | `11944b7ad06cd8f45042677bcb9ce8fb0c121546d80d187a820a5a98de921a30` |

## Delivered helpers

| File | SHA-256 | Purpose |
|---|---|---|
| `owner/owner_runner.py` | `b9e12895389a046e1c0a0bd4c43f01a215505c793be4106b31fafc18aa2aff34` | Supervised runner: `check`, `list`, `run` |
| `owner/test_owner_runner.py` | `571929ffc5d358af49f45ea911b97828e76d8742fea84d463a22f05cd63b6865` | CPU guard and process-lifecycle tests |
| `owner/owner-grant.template.json` | `e69f5cc3d75f33df04ec036b550da038271e9bd2b70bb67e0c2092e6241c7a9f` | Grant root fills in; its placeholders are refused |

The runner adapts this packet's reviewed `visual/tools/common.py` and `gpu.py`,
which come from handoff 0068's root-reviewed tools. It uses the same signal
blocking, process-group launch, SIGKILL-and-reap and fsynced ledger patterns.
It has no other dependencies.

## Identities verified (`check`, CPU only, writes nothing)

Receipt: `0073-owner-preparation-cpu-001/check-001.json` (`6a94c67f…`).

| Identity | Verified value |
|---|---|
| Plan, plus every reviewed field: argv, budget, environment, acceptance, binary | `d7da3f1c…` |
| Source freeze `temporal-owner-attachment-001/source-003/freeze.json` | `95e4b830…` |
| Root's CPU review `temporal-owner-attachment-final-cpu-root-20261013.json` | `d232a224…`; its manifest and freeze fields equal the plan's |
| Manifest blob `613ee59a:docs/spikes/evidence/temporal-owner-attachment.json` (`git cat-file`, read only) | `eff50118…` |
| Root's CPU enumeration `temporal-owner-attachment-preparation-001/result.json` | `68c5434e…` |
| Binary `…/binaries/incant_render-288759d68012f619` | `fc3cd756…`; 33,339,024 B; mode 0755; regular file; no `target` path component |
| Producer commit `613ee59a…` | Present: "feat(render): add checked integer motion owner attachment" |
| Grant | Absent: preparation only |

## CPU enumeration (`list`)

Receipt: `0073-owner-preparation-cpu-001/list-001/receipt.json` (`9d55d5a4…`).
- **Command:** root's exact argv: the frozen binary,
  `--list --exact models::retained::motion_owner_tests::exact_integer_owner_matches_visibility_and_leaves_motion_depth_color_unchanged`.
- **Process:** PID 17392, its own group, exit 0 after 0.013 s, reaped, group
  empty, no survivors.
- **Environment:** the plan's four variables, with a fresh TMPDIR, plus HOME
  inherited unchanged (value not recorded).
- **Output:** stdout `0b2289fa…`, identical to root's enumeration. It lists only
  the one test.
- **Binary:** unchanged afterwards (`fc3cd756…`).
- **Execution:** no test executed and no GPU use.

## CPU tests

**Command:**

```
python3 -I -B handoffs/0073-upscaling-review/owner/test_owner_runner.py
```

**Result:** `Ran 23 tests in 3.935s` and `OK`. The log is
`0073-owner-preparation-cpu-001/tests-002.log` (`2091076b…`).

**Guards (12 tests).** Each refusal charges nothing: no debit, no stdout.
- **Executable:** changed bytes, mode 0644, mode 0775, a symlink, a path
  inside `target`.
- **Exact filter:** a changed test name, or `--exact` removed.
- **Budget:** 25 submissions, a 120 s timeout, 1 retry.
- **Plan fields:** an extra `WGPU_BACKEND`, a changed final marker. Each plan
  mutation is refused both by the pinned plan hash and, when re-pinned, by the
  reviewed field values.
- **Identities:** a changed source freeze, a mismatched review.
- **Authority:** 11 malformed grants, non-JSON, the template itself, and no grant
  at all. With no grant, nothing is touched.
- **Predecessor release:** absent, wrong SHA-256, not a release, or listing live
  processes.
- **State:** a non-fresh evidence directory, a busy packet or global lock, a
  process already running the binary.
- **Read-only check:** `check` reports an absent grant.

**Lifecycle (10 tests), with ordinary small processes standing in for the
binary:**
- **Accepted child:** reaped, group empty, released, environment constructed. A
  second run is refused: no retry.
- **Unaccepted outputs:** early return without the final marker, Vulkan instead
  of Metal, the marker twice, a failed test, 2 passed, a PNG written. Each is
  reaped and released, and none is accepted.
- **Timeout:** the group is killed at 1.05 s.
- **Grandchild in the group:** killed with the group.
- **Escaped child:** a `setsid` grandchild is detected as a survivor. Release is
  blocked (`release-blocked.json`) and the runner fails closed.
- **Injected `BaseException`:** recorded, child reaped, released, then the same
  exception re-raised.
- **SIGTERM:** held pending, recorded as `interrupted_by: SIGTERM`, child
  reaped, then delivered.
- **Exec failure:** recorded without a PID.
- **Environment:** inherited INCANT/WGPU/RUST/Cargo variables are excluded.
- **`list`:** CPU-only enumeration.

**PID proof (1 test):** the final test checks every learned PID.

**Earlier aborted run:** `tests-001` stopped during sandbox setup, before any
process launched: `shutil.copy2` could not copy `/bin/sleep`'s protected flags.
It was fixed with a content-only copy. Its directories are kept as evidence.

## Owned PIDs, all reaped

- **CPU tests (17):** 17013, 17024, 17027, 17031, 17032, 17051, 17052, 17057,
  17063, 17097, 17100, 17103, 17106, 17109, 17125, 17128, 17131.
  - These are 12 runner children, plus one grandchild, the escaped survivor,
    the busy-binary sleeper, the signal sender and the listing child.
  - Every one was verified gone with `kill(pid, 0)` returning
    `ProcessLookupError` (`tests-002/pids-final.json`, `b3a4d199…`:
    31 records, `alive: []`).
  - Direct children were reaped with `wait`. The grandchild and the escaped
    process were killed, and launchd reaped them.
- **Enumeration (1):** 17392, reaped.
- **Helper subprocesses:** `ps` and `git` ran through `subprocess.run`, which
  always waits.
- **Final scan:** no process referencing the frozen binary, the sandbox binary
  or the CPU evidence directory remains.
- **Real lock:** `native-gpu.lock` was never opened (mtime unchanged since 00:59).

## The future GPU run, for root's review

`owner_runner.py run` does nothing unless the grant described below exists.

**Grant:**
- Root first verifies the 0072 warmup release.
- Root then creates `~/.codex/incant-evidence/0073-owner-attachment-gpu-001/`
  with a `grant.json` filled from the template, containing:
  - `issued_at`;
  - `runner_sha256` = `b9e12895…` after review;
  - the predecessor release path and SHA-256.
- The runner refuses any grant that does not bind exactly this runner, plan,
  binary, evidence directory, one launch and 24 submissions.

**Sequence of one invocation:**
1. Take the packet lock and `native-gpu.lock`, both non-blocking. If either is
   busy, refuse with nothing charged.
2. Verify the grant, the plan and all identities, the predecessor release (no
   live process fields), a fresh directory, and that no process runs the binary.
3. Append the predebit to `debits.jsonl`: one launch, 24 submissions, fsynced,
   never refunded.
4. Launch the plan's argv once:
   - in a new process group;
   - with the plan's environment, a fresh TMPDIR and HOME inherited unchanged;
   - with stdin closed and regular stdout/stderr files;
   - with a 90 s timeout.
5. SIGKILL the group and reap it, then check survivors by process group and by
   any argument naming the binary.
6. Append `ledger.jsonl`: argv, PID/PGID, exit, adapter line, checks, hashes and
   OS.
7. Write `release.json` only when no survivor remains; otherwise write
   `release-blocked.json` and exit nonzero.

**Acceptance:** all of these must hold:
- exit 0, with no timeout, interrupt or exception;
- `running 1 test`;
- exactly one `test result: ok. 1 passed; 0 failed; 0 ignored`, and no
  failures section;
- on stderr, exactly two `MOTION_OWNER` lines: first an `AdapterInfo` line
  containing `backend: Metal`, then the exact final marker;
- the binary unchanged;
- no image file anywhere in the evidence directory;
- the group empty, with no survivors.

An unsupported early return has no final marker, so it is a failure.

**Not authorized:** retries, the default WGSL binary, extra cases or
readbacks. The runner itself never authorizes a GPU run.

## Limitations

- **Coverage of the tests:** every lifecycle test uses sandbox scripts. The real
  binary ran only `--list`, so the runner's GPU path is untested on the real
  binary by design.
- **Predecessor-release check:** it requires `released_at` or
  `released: true`, and empty alive fields (`owned_processes`,
  `owned_processes_alive`, `ledger_pids_alive`, `process_groups_alive`,
  `survivors`). The real 0072 receipt format was not available to test against,
  so root should confirm it matches.
- **Survivor detection:** it covers the child's process group and any process
  whose arguments contain the binary path. A descendant that escapes the session
  *and* re-executes under an unrelated path would not be seen.
- **HOME:** "never override HOME" is read as passing HOME through unchanged. The
  value is never recorded.
- **Not covered:** this run would establish numeric owner-attachment behaviour
  only. History consumption, native owner packing, temporal appearance,
  physical GPU memory, performance and phone gates remain open, as do the
  a5ae3179 visual FAIL, C7 and C9 as written, and P1.
