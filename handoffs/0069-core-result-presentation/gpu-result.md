# 0069 GPU-only review, attempt 001: refused at admission

## Status

**Blocked before execution.** The single authorized `gpu_only.py run` refused at
its first admission step:

```
frozen source changed: handoffs/0069-core-result-presentation/brief.md
```

What did not happen:

- No GPU regression, capture process, GPU frame or image exists.
- No lock, lease, evidence directory or child process was created, so nothing
  needed releasing.
- The GPU interval is unused.

There is no appearance verdict, because nothing was rendered. I changed no gate,
source, wrapper or authority, and I did not retry.

## Model and effort

- **This session:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort **max**.
- **Root's ACP record** (`~/.codex/incant-evidence/claude-0069-gpu-only-acp.log`):
  "Claude session verified: Opus 5.5; thinking: max". I confirmed it before
  writing this result; no session values are copied here.

## Inputs verified before running

| Input | SHA-256 | Matches the brief or authority |
| --- | --- | --- |
| `gpu_only.py` (commit `0351d93e`) | `e3530e5d…` | yes |
| `~/.codex/incant-evidence/0069-gpu-review-ready.json` | `6b8064c8…` | yes; every field read |
| Predecessor release, `0073-upscaling-visual-002/gpu-released.json` | `d0998c95…` | yes |
| CPU manifest `cpu-evidence/manifest.json` | `1d5f8a89…` | yes |
| CPU `release.json` | `bc2cabe2…` | yes |
| Generation-5 freeze | `acb68a62…` | yes |
| Root review `core0069-gpu-wrapper-root-20261013.json` | `137b6194…` | read |

- The evidence path `~/.codex/incant-evidence/0069-gpu-001` did not exist.
- `python3` resolves to Homebrew 3.14.8, equal to the generation-5 identity.

## What ran

1. **Read-only preflight**, `python3 -B …/gpu_only.py check`, as the proposal
   allows. It refused with the same message.
2. **Read-only diagnosis.**
   - Of the 16,977 files inventoried in `source-final-003.json` (head
     `15a59060`), exactly one differs: `brief.md`.
   - Generation 5 pinned `brief.md` at 14,158 bytes, SHA-256 `100088d8…`. That
     version is unchanged at `15a59060` and at root's reviewed wrapper commit
     `0351d93e`.
   - The working tree has 16,573 bytes, SHA-256 `e36dee4a…`, from root's
     scheduling commit `399fea51`.
3. **The authorized command, once, in the foreground**, from the worktree root:

   ```
   python3 -B handoffs/0069-core-result-presentation/gpu_only.py run --evidence /Users/andreyyurlov/.codex/incant-evidence/0069-gpu-001 --authority /Users/andreyyurlov/.codex/incant-evidence/0069-gpu-review-ready.json
   ```

   - It ran from 08:36:55Z to 08:36:56Z and exited with status 1. `inputs()` →
     `checked_freeze()` refused.
   - That happens before the authority check, before seeding the evidence
     directory, and before any lock, lease, debit or child.
   - Its exact output is retained in `gpu-attempt-001/`.
4. **Afterwards:**
   - The evidence path is still absent, and no 0069 process exists.
   - No process has the shared `native-gpu.lock` open.
   - The CPU release, ledger, authority and generation-5 freeze hashes are
     unchanged.

## Cause

Generation 5 freezes the bytes of every tracked file at `15a59060`, including
this packet's `brief.md`. The wrapper admits new packet files but compares every
inventoried one.

- Root's review preflighted at `0351d93e`, where the brief still had its frozen
  bytes.
- Root's later scheduling commit `399fea51` appended this GPU revision to the
  brief, which made the wrapper refuse.

No engine source, compiled input, package, recording or tool differs.

## Repair options for root (not applied)

1. **Restore the brief and move the revision.** Put `brief.md` back to its
   generation-5 bytes (`100088d8…`, 14,158 B) and carry the GPU revision in a new
   packet file, which the wrapper already admits.
   - `gpu_only.py` and the authority stay byte-identical: neither hashes the
     brief.
   - `0069-gpu-001` is still unused.
2. **Exempt the brief in the wrapper.** Have the wrapper exclude root-owned packet
   documents such as `brief.md` from the frozen-content comparison, as it already
   admits new packet files. This changes the wrapper's hash, so it needs a fresh
   review and authority.

Either way, the same single run can then go ahead in a new session. I stopped
here, as the brief directs.

## Appearance

No frames were rendered, so there is no verdict. Still pending for the GPU run:

- contrast and readability at normal and narrow sizes;
- Arabic RTL, Simplified Chinese and pseudo-locale;
- won, lost and restart states;
- the label padding decision.

## Retained evidence (this commit)

- `gpu-attempt-001/`: `started-utc.txt`, `finished-utc.txt`, `exit-status.txt`
  (`1`), `stdout.txt` (empty) and `stderr.txt` (the refusal traceback).
- `gpu-provenance.json`: links the authority, wrapper, root review, predecessor
  release, ACP record, CPU result, manifest, review, release, ledger, freeze and
  source inventory, plus the brief's expected and actual identities.
- This file.

Not committed by me:

- `cpu-result.md` and `cpu-evidence/` belong to the CPU lane; they are untracked
  and untouched.
- `root-helper-review.json` is root's.

## Limitations and notes

- No GPU work happened, so this attempt makes no regression, frame, image,
  appearance, phone, performance or phase-gate claim.
- A `clippy-driver` build on `/Users/andreyyurlov/.codex/incant-core-presentation-target`
  was running during the checks. That target belongs to 0072; 0069 did not start
  or own the process.
- `tools/__pycache__/lease.cpython-314.pyc` was created at 03:50, before this
  session, by an earlier import without `-B`. I left it in place; the frozen tool
  identity counts files only, so the directory doesn't affect it.
