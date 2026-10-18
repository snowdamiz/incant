# Windows TypeScript native dependency cache

Checkpoint57's Windows `authored_startup_uses_shared_cook_and_never_the_engine_fallback`
failed at the unchanged eight-second cook deadline. Its fixed phase receipt puts
6.639193 seconds between `typescript_ready` and `swc_ready`; the source snapshot
completed at 7.105796 seconds, and `typecheck_done` was not reached. This identifies
the package-import interval, not whether carrier decoding, filesystem work, DLL
loading, antivirus or host scheduling caused the outlier. Earlier successful
Windows observations had 0.289–0.575 second SWC import intervals; checkpoint32 and
46 do not acquire a more specific retrospective cause.

The former private per-cook SWC cache made every cook materialize a new native
addon. Windows now obtains LocalAppData through `SHGetKnownFolderPath` and selects
its fixed `swc` child, matching the pinned carrier's ordinary dependency cache.
It does not read an ambient cache override. Canonical, case-aware containment
refuses overlap with the project; existing path components must be local,
no-reparse directories held without delete sharing until the child is reaped.
A redirected network LocalAppData fails explicitly. The carrier independently
checks ancestor security, creates private SID/DACL namespaces, verifies every
cached byte, coordinates writers and repairs corrupt ordinary entries.

`env_clear` remains in force. The child receives neither HOME nor LOCALAPPDATA,
so rejection of this explicit Windows cache cannot silently select another user
cache. Source catalogs, snapshots, temporary files, diagnostics and outputs remain
private to each cook. Every Node process and project compilation is fresh; no
project-generated executable bytes are shared. The eight-second deadline still
includes package loading, with no retry or preparation outside the timer.
Non-Windows cache policy is unchanged. Carrier eviction is best effort; its
three-image policy is not a hard total storage cap while files are busy or staging
files survive a killed process.

This removes definite repeated materialization work. The targeted Windows run
below confirms reuse and repair, but does not establish a general latency gain
or permanent timeout repair. Cold first loading can still fail.
The manual TypeScript diagnostic includes a Windows-only ignored control using
an exclusively owned temporary cache: real compiler cold, warm and corrupt-repair
outputs must match byte-for-byte; changed project source must produce changed
output while reusing the verified native image. It never corrupts the actual
user cache. The exact checkpoint57 authored-startup test also runs once,
separately from the older fallback/template consumers: six successful production cooks are
expected across those harnesses, plus the four disposable diagnostic cooks.
Fixed phase rows and the real test outcome are retained. Resolver,
reparse/containment, and ambient-setting controls remain ordinary tests. The
Windows diagnostic runs all eleven ordinary controls and the one ignored cache
control explicitly. Junction fixtures need no symlink privilege; failed fixture
creation fails the test. After successful setup, the independent diagnostic
steps run even if an earlier consumer fails, preserving the overall job failure.
Local checks and hosted Windows results are recorded separately below.

Local macOS qualification on 2026-10-18 passed five common TypeScript cook
controls, the exact checkpoint57 authored-startup test with all ten compiler
phases, and strict `incant_cook --features typescript --all-targets` Clippy.
The three actual tool sessions exited zero in 89.818273 seconds total; all 55
observed process/group identities were absent afterward, and both test
executables were copied before releasing the shared target. Commands used the
CI profile, offline locked resolution, two jobs, and the retained renderer/sha2
profile overrides. This qualifies the shared invocation and non-Windows
regression path; it does not execute the new Windows resolver or carrier-cache
controls. Hosted Windows cold/warm/repair results are recorded separately below.
All ten fast development checks also passed, with the intentionally dirty
source contents preserved across the run. An initial runner selected Xcode
Python 3.9 and failed on unsupported Python APIs; that failed attempt is retained,
and the corrected runner used pinned Python 3.14 without source changes.
[Local result and exact command identities](evidence/windows-typescript-dependency-cache-local.json)
record these boundaries; they are not Windows or performance evidence.

The first hosted diagnostic at exact `d90689b3`,
[run 39111023784](https://github.com/snowdamiz/incant/actions/runs/39111023784/job/117627006340),
failed during Windows compilation. All four independent commands exited 101:
`CompilerPaths.engine` required `&Path`, while the Windows launch-directory
branch supplied an owned `PathBuf` at `typescript.rs:107`. No tests, authored
cooks or cache phase observations executed. The non-Windows branch used
`&PathBuf`, which explains why the earlier local checks did not catch this.
The correction borrows `launch_engine.as_path()` on both branches, preserving
the path lifetime, cache policy, fresh compilation and eight-second deadline.
The corrected common code now passes five native cook controls and strict
all-target Clippy on this Mac, as well as all ten development checks. The
[local records](evidence/windows-launch-path-local.json) identify exact source
`d8c7db25` and retain the raw command results. The subsequent hosted execution is
recorded below. This compile failure neither reproduces nor repairs the original timeout. [The scoped receipt and inert raw records](evidence/windows-typescript-dependency-cache-hosted-001.json)
retain all four compiler failures and the exact hosted source identity.

The corrected [Windows diagnostic](https://github.com/snowdamiz/incant/actions/runs/39155781027/job/117758598082)
at exact source `70a54db1` succeeded on 2026-10-18. All four commands exited zero:
six fallback/template controls, the exact checkpoint57 startup control, eleven
ordinary resolver/path/environment controls, and the isolated cache control.
The latter is intentionally ignored in the ordinary suite and explicitly executed
in its own command; it is not counted as a passing ignored test.

The five successful fallback/template cooks took 1.560–2.142 seconds, alongside
one expected refusal. The previously failing authored-startup cook took 1.304
seconds, including a 31.234 ms SWC import interval. In the disposable cache,
cold, warm, corrupt-repair and changed-source cooks took 1.787, 1.363, 1.630 and
1.366 seconds respectively. Their SWC import intervals were 415.408, 38.440,
301.887 and 33.933 ms. Cold/warm/repair outputs matched exactly; changed project
source recompiled to changed output while retaining the validated native image.
The normal user cache was not corrupted or altered by that diagnostic. The
eight-second production deadline remains unchanged.

[The hosted receipt and inert raw records](evidence/windows-typescript-dependency-cache-hosted-002.json)
retain all nineteen passing controls, exact source/toolchain, phase observations,
exit receipts and workflow result. This establishes the selected Windows
behavior and one successful timing observation, not a repeated controlled
performance comparison or a permanent timeout repair. The original outlier's
underlying cause remains unestablished. Comprehensive release qualification is
separate; the earlier compile failure and checkpoint57 timeout remain recorded.

Source basis: pinned SWC 1.16.13, commit
[`1391a346e1ba8ef0cbaaa7c1b90c391ad6d150c5`](https://github.com/swc-project/swc/tree/1391a346e1ba8ef0cbaaa7c1b90c391ad6d150c5),
[cache verification/publication](https://github.com/swc-project/swc/blob/1391a346e1ba8ef0cbaaa7c1b90c391ad6d150c5/crates/swc_native_addon/src/cache.rs#L265-L314)
and [Windows cache security](https://github.com/swc-project/swc/blob/1391a346e1ba8ef0cbaaa7c1b90c391ad6d150c5/crates/swc_native_addon/src/platform/windows.rs#L129-L155).
