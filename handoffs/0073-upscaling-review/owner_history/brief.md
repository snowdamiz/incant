# Owner-history numerical kernel check — preparation only

This packet grants **no GPU or ACP execution**. Root must review the final plan,
runner, source/build evidence and current predecessor GPU release, then issue a
fresh grant. Resume the existing Claude0073 session only after that instruction;
verify **Claude Opus 5.5 and thinking Max through ACP before prompting**. Never
inherit effort. A default model setting is insufficient.

Use only `runner.py run /absolute/path/to/owner_history/plan.json`. No build,
recook, screenshot, image loading, full renderer frames, extra probes, retries,
or source edits. `runner.py check PLAN` is read-only. `grant.template.json` is
explicitly non-authorizing. Keep previous packets, raw failures and dirty briefs
unchanged. Root/publisher own all shared branches.

The frozen plan binds root's final source freeze (22 sources / 2,736 build
inputs), both CI test executables and their successful build receipts. Its
`producer` identifies the Git HEAD at compilation; the complete dirty source
identity is the separately pinned freeze, not that HEAD alone. The independent
CPU audit rehashed the source closure and retained artifacts: 33 cooker tests,
152 default and 167 cooked tests; 57 freshly generated old shader payloads exact,
nine manifest producer-stamp-only changes, and 67 prior records explicitly not
rebuilt. The old failed signed-index cooker runs remain retained. Root imported
Claude's index correction as `2bccac77` and rebuilt both exact binaries.

The runner reuses only process/I/O functions from the SHA-pinned, previously
reviewed sibling `owner/owner_runner.py`; it does not use its obsolete plan or
launch path. Both files must be present byte-exact. The supplied plan requires
one precise test:

`upscaling::system::owner_device_tests::actual_owner_history_obeys_provenance_and_preserves_ordinary_fallback`

Execution is sequential, WGSL then cooked-native Metal. Each invocation executes
12 fixture cases at 16×16, including 12 native depth clears, 12 owner resolves and
two ordinary-resolve comparisons (cases 7/8). Copies are batched into those same
12 command encoders: **12 explicit queue submissions and 12 maps per process;
24 submissions/maps total**, two attempted children maximum, 90 seconds per
child. Texture uploads are queue writes, not extra explicit submit calls.
There are no image files or complete renderer frames. Both processes must report
the same actual Metal adapter identity. A refused capability or early pass is a
failure: all 12 ordered case messages and the exact final marker are mandatory.

The test asserts finite/valid color, depth and canonical owner records for all
texels, exact clear/outside outputs, independent selected color/depth/record
expectations, and byte-exact original fallback color/depth in cases 7/8. It covers
fresh/outside history, static persistence, changed support, class overflow,
anchor/depth rejection, global/camera invalidation, clamp contributors, positive
support renormalization and malformed/clear data. These are kernel/backend
controls, not full retained scheduling or appearance evidence.

A fresh evidence directory is fixed in the plan. The runner takes both packet
and global GPU locks nonblocking, checks predecessor release and every pin,
charges before each child and never refunds. It stops on first failure. Regular
output files avoid pipe backpressure. Each process has its own group; timeout,
interrupt and exceptions kill/reap it before analysis. `release.json` is emitted
only after no owned process/group survives, otherwise `release-blocked.json`
records the failure. Root's supervisor must reap the enclosing ACP handle and
verify its release; a process still running is not a released lease. SIGKILL of
the supervisor cannot be made self-cleaning and requires external recovery.

Child environment is the fixed minimal allowlist plus HOME passed through
unchanged, with no inherited INCANT/WGPU/RUST/Cargo flags. No account data or
HOME value is logged. After the bounded turn, report the exact two exit statuses,
adapter, case/submit/map totals, all result/release SHA-256s and survivor state;
commit packet result only with `Built-by: claude`. Do not retry a failed case or
adjust assertions. Root decides the next action.

Prior 0072/0073 quality failures and C7/C9/P1 remain preserved. This evidence, even
if passing, does not accept full retained history, visual quality, performance,
MetalFX history ownership, fixed TAA ownership, mutable SSAO/effect integration,
or reference-phone budgets. No new broad renderer claim.
