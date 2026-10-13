# Implementation status

[PLAN.md](PLAN.md) revision 8 is the source of truth. Phase 1 is in progress;
the full engine and Driftwake release are not complete. Calendar targets and
phase durations have been removed. Work proceeds continuously and in parallel
where dependencies allow; exit gates still require their stated evidence.
Service-backed features use local containers or in-process substitutes during
development, behind production-shaped contracts. Real provider integrations,
security/device evidence and release gates remain real; local substitutes do not
satisfy those gates.

The director authorized Phase 1 with remaining Phase 0 items deferred, sensible
engineering progress without repeated advancement questions, and agent merges
after scoped review and passing checks. These authorizations do not approve a
phase gate. Section 12 decisions are settled. Current computer use and screen
capture are authorized; Claude Opus 5.5 with explicitly verified Max effort owns
visual work, and Astra owns correctness and integration.

## Default-off SSAO — curated current-main qualification

The complete diffuse SSAO/R3/C2 producer has independent numerical and Claude
Metal appearance acceptance (`22ac4dc7`). Current integration preserves its
sampling/filter shaders, bounds and default-off behavior, and explicitly joins
jittered reconstruction, split model drawing and all four Models timing passes.
Current source review and thirteen checks pass (168 default and 200 overlapping
cooked CPU cases). The exact current composition completed eight bounded Metal
frames under Claude: default SSAO admission and expected cooked refusal both
passed, with all children reaped. PR230 merged after independent final review and
the required exact-head fast check. Original
appearance acceptance remains producer-scoped.
Authored N6 remains failed at seven codes; revised N6T/N8 margins are 0.79/0.97
codes. Cooked SSAO, non-Metal appearance, phone and performance gaps remain open.
[Current composition](docs/spikes/ssao-main229.md).

## Integer motion owner attachment — current-main integration

The reviewed owner/change foundation gains an explicit R32Uint attachment with
stable integer IDs and change bits at opaque/MASK fragments. The curated source
uses the existing main renderer, excludes private upscale/MetalFX ancestry, and
refuses fixed temporal history while attached. Default output and old shader
sources remain unchanged. Source review is accepted; all seventeen current checks pass with 146 default
and 175 overlapping cooked CPU cases and exact old/new shader payload identity.
Final integration and checkpoint 24 documentation review is accepted (`6029ae16`);
PR229 merged after the required fast check. Original producer numerical Metal evidence covers twelve renders
plus twelve readback submissions only; no appearance or history-consumption
claim. [Current integration](docs/spikes/temporal-owner-main228.md).

## Native physics numerical admission — main227 CPU qualification

A default-off pinned-kernel preflight checks primitive mass/pose/extent and
historical fixed-target CCD AABB coherence without constructing live owners.
It preserves float bits, applies native force arithmetic, and charges a checked
cumulative logical work model. Both 214-case deterministic/fast8 suites and
scoped strict Clippy, wasm, dependency and formatting checks pass. Independent
source and final producer reviews are accepted. Current main227 rebuilt suites
also pass 214 cases per mode and all fifteen scoped checks; final integration
review and the required fast check passed; PR228 merged. [Current composition](docs/spikes/physics-numerics-main227.md).
Whole-solver derived
arithmetic, exact private construction, capture authentication and stable IDs/
events remain required; `PhysicsUnsupported` is unchanged.
[Scope and evidence](docs/spikes/physics-native-admission.md).

## Joined physics owner boundary — source and CPU qualification accepted

The [joined inert frame](docs/spikes/physics-world-boundary.md) composes 22
canonical sections under one aggregate allocation budget and exact borrowed
witness joins. Its checked decoder invokes a scoped callback only after all
sections pass; it never constructs a live solver. The separately reviewed
default-off trusted native adapter preserves the retained CCD verdict while
replacing scratch. Native Clone owners continue identically for 280 paired
steps per numerical mode, with warm contacts, sleeping/waking, sensor events,
removal/reuse and CCD controls. Both 202-case CPU modes, ten parallel controls
and all ten scoped checks pass. Source and final producer evidence reviews are
accepted; current-main composition passes 202 CPU cases in each numerical mode, ten
two-worker controls each and thirteen scoped checks. Final composition review is
independently accepted; PR227 merged after the required fast check. [Current composition](docs/spikes/physics-world-main225.md).
Derived geometry/mass/work admission, authenticated capture and the
whole native constructor remain required. `PhysicsUnsupported` is unchanged.



## Temporal owner foundation — current-main CPU qualification

The [owner/change provenance foundation](docs/spikes/temporal-owner-provenance-verification.md)
assigns stable placement IDs across handedness batches and compares actual accepted
transforms and joint palettes, publishing change membership only after submission.
Its source and producer CPU evidence are independently accepted. The isolated
eight-file Rust delta passes 143 default and 172 cooked CPU cases and twelve
current-main checks without importing private upscaling or MetalFX ancestry.
The disjoint main225 journal-test join preserves renderer source. Final current-main
review and the required fast check passed; PR226 merged. Shader bytes, defaults
and memory ceilings are unchanged;
integer shader transport subsequently merged in PR229; history consumers remain open.
[Current composition](docs/spikes/temporal-owner-main225.md).
No new GPU, appearance, performance or device result is claimed.

## Core result HUD — current-main qualification

The accepted 0069 label halos, localized result/restart presentation and approved
font subset have accepted main212 CPU/shipping evidence and a disjoint actual
main215 tool/test-only ancestry qualification. All selected UI/text/sample
sources equal Claude's accepted producer; 221 affected Rust CPU tests, 20 Python
tests, four public locale builds, 14 native positive processes and ten fresh
source-deleted shipping runs pass, including four exact whole-session PCM
continuations. Original g5/g6 GPU acceptance remains attached to those binaries;
no new GPU/device/performance claim is made. Final archive review is accepted;
current-main qualification is independently accepted and PR216 merged after
the required fast check. [Scope and evidence](docs/spikes/core0069-main212.md).

## Inert collider properties — main216 integration

Accepted collider records and exact owner joins are composed on main216 with
unchanged physics source, vendor inputs, lock and selected dependency graph.
The two retained executable suites pass 34 cases again; the complete 135-case
producer suite and strict Clippy/wasm checks remain exact-source evidence.
Current-main integration review and the required fast check passed; PR217 is
merged. Live physics saves remain refused.
[Scope and relevance](docs/spikes/physics-collider-main216.md).

## Inert broad-phase owner — source and CPU evidence accepted

Checked broad-phase records preserve exact pair/adjacency and update history,
joined to existing collider/narrow owners. A distinct pending-tree witness admits
only the demonstrated deferred ancestor flag/bound state; strict BVH validation
is unchanged. Eight new actual 2D/3D cases include 96 trusted continuation steps,
plus the prior 728. Independent source and root final evidence review accepted
151 CPU tests and seven scoped checks. Missing deferred trees still refuse;
`PhysicsUnsupported`, hostile live restore and full pipeline-phase admission stay
closed. The actual main221 composition passes 172 CPU cases in each of default
and fast8 modes plus all eleven scoped checks; final integration review is accepted
and PR223 merged after the required fast check.
No additional vendor changes or performance gate.
[Current composition](docs/spikes/physics-broad-main221.md).
[Scope and evidence](docs/spikes/physics-broad-owner.md).

## Inert narrow-phase owner — source and CPU evidence accepted

The checked physics boundary now preserves collider graph-index ownership, ordered
intersection payloads and independent emitted-event history. Eight new native
2D/3D controls pass, including 128 trusted continuation steps with retired-pair
reuse, sensor transfers, event-mask toggles and generation reuse. Independent source and final evidence review accepted 143 CPU tests and seven
scoped checks. The exact-source main217 qualification passes six current checks
and a 36-case copied-binary rerun with 728 trusted continuation steps; independent
current-main review is accepted and PR218 merged after the required fast check. `PhysicsUnsupported` and hostile live restore
admission remain closed; no vendor/world-format changes.
[Scope and limits](docs/spikes/physics-narrow-owner.md);
[current-main qualification](docs/spikes/physics-narrow-main217.md).

## Inert CCD and integration configuration — composed main220 checks passed

The accepted bounded configuration/CCD records and read-only vendor getter are
composed with the current narrow-owner boundary. Actual rebuilt deterministic
and fast8 suites each pass 164 tests; strict Clippy, wasm32 and the std-only normal
graph pass. Eleven current checks are zero-exit, with exact source/pin identities.
Independent current-main review is accepted and PR221 merged after the required fast check. Live save/restore admission remains
closed. [Scope and evidence](docs/spikes/physics-ccd-main220.md).

## Timestamp backend diagnostic — capability repair

Checkpoint22 is terminal and unpromoted. A separate direct timestamp
diagnostic failed on hosted Apple Paravirtual Metal after the production timing
test had correctly completed its unsupported-capability path. The test-only repair
keeps mandatory backend selection, runs supported timestamp modes, and executes
checked ordinary compute when queries are absent. Two CPU policy tests and focused
strict checks and independent final review pass; PR219 merged after its required
fast check, and hosted successor execution remains pending. [Failure and repair evidence](docs/spikes/timestamp-diagnostic-capabilities.md).

## Fixed-resolution temporal accumulation — composed checks passed

The [curated fixed-only integration](docs/spikes/fixed-temporal-main217.md) keeps
upscaling, MetalFX, bloom and SSAO outside its narrowed four-program native cooker.
Independent source review, default/cooked CPU checks, strict Clippy, native/browser
consumer compilation and actual Metal/SPIR-V recook validation pass. Two copied
current-composition binaries then completed eight bounded Metal frames, checking
fixed history, overlay depth and timestamp submission together. No new pixel review
is claimed. Producer acceptance remains opt-in fixed mode only, with softness and
MASK detail approximately 18% of reference. Final integration evidence review is
independently accepted and PR220 merged after its required fast check. Defaults
and phase/device gates remain unchanged.

## Output-space bloom — current composed evidence complete

The accepted physical-output bloom footprint is integrated independently of SSAO,
with unchanged shader math and resource bounds. Current default/cooked CPU,
consumer and browser checks pass, followed by eight bounded Metal frames joining
fixed history, bloom/refusal and Display timing. Original Claude0075 bloom-only
appearance acceptance remains source-specific; no new appearance claim is made.
Native cooked output refuses bloom explicitly. Independent final integration
review and exact main221 source relevance are independently accepted; PR222 merged after the required fast check. [Scope and evidence](docs/spikes/bloom-main219.md).

## Physics pipeline boundary — composed source and CPU evidence

The [pipeline capture slice](docs/spikes/physics-pipeline-boundary.md) preserves
exact gravity and the historical `None`/false/true staged CCD verdict with checked
integration/CCD witness association. Actual 2D/3D continuation shows reused versus
fresh pipelines can differ in CCD cache and substep counters after the last body
is removed, while serialized owners remain equal; no trajectory difference is
claimed. Deferred work, quarantine, nonpositive or unverified phases refuse.
Independent source review, both 176-case CPU modes, 20 parallel controls and all
ten producer qualification commands pass. The actual broad/narrow/CCD composition
passes 192 default and 192 fast8 CPU cases plus explicitly pinned two-worker
controls; fifteen commands pass. Final current composition review is accepted;
the actual main223 ancestry join preserves all tested executable inputs. PR224
merged after the required fast check.
[Current composition](docs/spikes/physics-pipeline-main222.md).
Whole-owner provenance, derived geometry/work
checks, live construction and exact physics save admission remain open.

## Static Wasm C builds — bounded local scheduling verified

The trusted bundle feature now enables pinned cc's Cargo-jobserver scheduler.
A local real-C comparison preserves exact object/archive bytes while respecting
`-j1`/`-j2`; fresh 221-module conformance passes 492 tests, Clippy and three development
processes. Compiler flags, locks and module coverage are unchanged. Feature
unification also affects other cc build consumers in the selected host graph.
Windows candidate execution and timing remain pending; no Windows speedup or
performance gate is claimed. [Scope and evidence](docs/spikes/wasm-c-parallel.md).

## Current overview — main231, 2026-10-13

Development main231 includes the reviewed opt-in cooked renderer families and
Vulkan cache lifecycle, dynamic resolution, numerical retained motion, CPU/GPU
profiling scopes and joined inert physics owner boundaries. Each lane retains its own
execution and acceptance limits below; the complete engine and game remain open.

Checkpoint24 promoted exact main226 through PR213 after all 23 required contexts
and eight exact-head workflows succeeded. Release `2a9d72ea` matches selected
main226 `079ee1aa` exactly. Independent review verified the tested tree, ancestry,
unchanged protection and all success receipts. The journal replay repair passed
on actual macOS/Linux/Windows; optional timestamp diagnostics preserve explicit
Unsupported results on Apple Paravirtual Metal and measured paths on Vulkan/Dx12.
Checkpoint25 is frozen on reviewed main230 (`56185dab`) in PR232, with all23
required contexts preserved. One Windows UI test timed out; the original failure
remains failed while remaining workflows run. Its scoped per-case deadline
repair passes local focused/full UI checks and is independently accepted;
publication follows its required fast check.
Later main changes do not alter either frozen checkpoint.
[Promotion evidence](docs/spikes/release-checkpoint-24.md).

Checkpoint23 is terminal and unpromoted: 22 required contexts succeeded and its
macOS command journal assertion failed. The hosted error was not printed, so the
inherited-lock cause remains inferred from a controlled local reproduction.
PR225's test-only isolation preserves production locking and authorization.
Earlier checkpoint21/22 portability and optional-query failures remain retained.
Main227–229 work stays outside release; no device, performance or phase gate follows
from promotion. [Previous checkpoint20](docs/spikes/release-checkpoint-20.md) remains
historical evidence for selected main205.

Failed checkpoints15–19 remain historical and unpromoted. In particular, checkpoint17
exposed the quota/probe issues repaired in PR188/191; checkpoint18 exposed the editor
test import repaired in PR193; checkpoint19 exposed the cache count and vendored
Parry compiler-identity issues repaired in PR201/203. Their original failures remain
retained. Promoted checkpoint24 preserves
ADR0014's exact selected tree and all 23 requirements.

Android static-Wasm integration is merged in PR190 after independent source-relevance
review and the fast check. Current CPU/public preflights and both-mode target compile/
admission checks pass; six actual Android callers remain producer-scoped.
Core authored bindings and the corrected locked-exit proof are merged in PR185
([current-main evidence](docs/spikes/core-main182.md)).

Dynamic resolution and cooked clusters have accepted producer Metal evidence; their
[current-main197 integration](docs/spikes/clusters-drs-main197.md) passes 14 CPU/compile/tooling checks with independent final packet acceptance; PR198 is merged after the required fast check.
Current composed GPU execution remains pending. GPU timestamps have scoped producer Metal evidence and passing current-main207 CPU checks, as described below. CPU profiling has accepted producer
package evidence and passing [current-main194 CPU/source-relevance checks](docs/spikes/native-cpu-profiler-main194.md);
independent integration review and the exact-head fast check passed, and PR195 is merged. These independent lanes do not satisfy
phone, performance, complete-renderer or phase gates. Older milestone narratives
below are historical integration snapshots, not the current queue.

Safe Core live-HUD changes are merged in PR192. The optional result-label and won-objective
correction is merged in PR194 after public native checks, independent integration
review and the exact-head fast check. Original source-free shipping remains producer-scoped; no new
appearance or device acceptance is inferred. See [Core result evidence](docs/spikes/core-result-hud-main192.md).
## Checked rigid collider properties — CPU source slice

The private [collider-state slice](docs/spikes/physics-collider-state.md) preserves
fixed native collider configuration, including mass modes, group-test modes and
historical enabled tri-state. The joined rigid owner derives force thresholds from
checked colliders and requires exact body/geometry witnesses. Independent source
and final evidence reviews accept the scoped result: 135 CPU tests (none ignored),
Clippy, wasm32, normal graph and repository checks pass. PR217 is merged after
current-main relevance review and the required fast check.
Physics world save admission stays closed; manager, derived arithmetic and
phase/constructor validation remain open. No live restore or device claim.

## Physics world continuation — trusted native proof, live saves still closed

The [joined rigid owner slice](docs/spikes/physics-world-continuation.md) now checks
solver buckets, color masks, pair hints, force-event caches and stamp-sensitive
sleep scans against the existing inert witnesses. Actual trusted native 2D/3D
reconstruction continues identically through contacts, sleep/wake, removal and
handle reuse; omitted scan state has a failing near-wrap control. Final default
121 cases and two overlapping 20-case parallel mode suites pass, along with
Clippy, runtime/wasm/compiler identity and tooling checks. The default-off pinned
Rapier adapter stays absent from ordinary runtime compiler features.

[The execution packet](docs/spikes/physics-world-continuation-results.md) has independent source and final evidence acceptance; current-main
qualification is independently accepted and PR212 merged after the required fast check. No hostile live constructor or engine save admission
is enabled; complete collider/manager/joint/phase composition remains pending.

## GPU timestamps — device review passed; concurrent interval corrected

Claude's bounded default-feature Metal review passes92 actual GPU/regression cases
and reports unchanged pixels in all timing-off/on controls. Three lit samples
exposed concurrent passes outside the former first/last encoding-order interval.
The CPU decoder now encloses all measured pairs, with explicit malformed/wrap
refusal; ten timing CPU tests and renderer Clippy pass, with independent source
review accepted. [Scope and retained evidence](docs/spikes/gpu-frame-envelope.md).
All ten [current-main207 integration checks](docs/spikes/gpu-timestamps-main207.md) pass: 103 default CPU cases and 130 overlapping cooked cases, with 94 GPU cases ignored in each configuration. Final independent integration review and the exact-head fast check passed; PR208 is merged. Retained motion is explicitly uninstrumented; the envelope spans measured extrema including intervening gaps and does not measure complete-frame GPU cost. Cooked/native-window/phone and complete
profiler gates remain open.

## Native-window CPU profiling — current-main checks pass

The desktop player accepts explicit summary/standalone Tracy profiling on its
window route. Simulation attempts, redraw-only CPU work and restarts have separate
fixed zones; the final native report uses the existing host callback. Ten producer
CPU/compile/tooling checks and independent producer review pass. All ten
[current-main204 checks](docs/spikes/native-window-profile-main204.md) also pass;
final integration review and the exact-head fast check passed, and PR205 is merged. Actual native-window execution and live Tracy capture remain pending.
[Scope and evidence](docs/spikes/native-window-profile.md).

## Retained opaque motion — current-main CPU integration verified

The opt-in retained motion/depth implementation preserves exact accepted live/staging
geometry and advances history only after submission. [Current-main203 checks](docs/spikes/retained-motion-main203.md)
pass 91 CPU cases plus 12 overlapping cooked-refusal cases, Clippy, wasm32, native-window
compile and dependency checks. The prior two numerical GPU processes remain scoped to
their recorded producer binary; no current composed GPU or appearance claim is made.
Final integration review and the exact-head fast check passed; PR204 is merged. Default rendering is unchanged; temporal color,
upscaling and cooked motion remain separate.

## Exact physics saves — inert island ownership verified locally

The inert path now checks reciprocal active/persistent body membership and
contact-link ownership, retaining native ordering, free stacks and deferred
split scheduling. Actual 2D/3D observations include sleep/wake, empty awake
containers, fixed-body active history, reorder, removal/reuse and delayed splits.
115 CPU tests, Clippy, wasm32 and dependency/contract checks pass; independent
producer review is accepted. The [current-main208 integration](docs/spikes/physics-islands-main208.md) passes eight checks and 115 CPU cases with 2,914 unchanged inputs; final integration review and the exact-head fast check passed; PR209 is merged.
Stamp-sensitive scratch, solver masks/buckets, complete collider/owner composition
and continued live execution remain open; physics saves stay refused.
[Island contract](docs/spikes/physics-inert-islands.md).

## Exact physics saves — inert regular body state verified locally

The inert path now retains every fixed regular rigid-body field, including
mass/force/velocity/CCD caches, activation history, exact u128 user data and
island scalar references, bound to checked body/attachment rows. 101 CPU tests
and all scoped checks pass; native both-dimension evidence covers force/mass
changes, sleep/wake, automatic CCD and removal/reuse. Independent producer review is accepted. The [current-main206 integration](docs/spikes/physics-body-main206.md) passes all eight checks and 101 CPU cases with 2,900 unchanged inputs; final integration review and the exact-head fast check passed, and PR207 is merged. Derived mass/island consistency, solver work admission, complete
collider/owner construction and exact continuation remain required; physics
saves stay refused. [Body contract](docs/spikes/physics-inert-body-state.md).

## Exact physics saves — inert pair persistence producer accepted

The std-only inert format checks pair event/color/recycle fields and closed
composite workspace maps against existing manifold/geometry ownership. Native
tuple keys, detector order/timestamps and manifold bijections are retained;
unknown workspace kinds refuse explicitly. Producer88 CPU cases, Clippy/wasm32
and dependency checks have independent acceptance. The [current-main205 integration](docs/spikes/physics-pair-main205.md) also passes all eight checks with 88 CPU cases and 2,895 unchanged inputs; final integration review and the exact-head fast check passed, and PR206 is merged. Previous-cluster scratch reconstruction, complete
body/island/solver ownership and live/save admission remain open.
[Pair-state contract](docs/spikes/physics-inert-pair-state.md).

## Exact physics saves — inert manifold ownership verified locally

The std-only inert format now joins contact edges to exact collider generations,
current body parents and checked child geometry, preserving fixed manifold
metadata and ordered plain/cluster point rows. Pending mutation journals return
an explicit typed refusal. Actual planar/spatial native tests observe nonzero
warm starts, removal/reuse and mixed-child cluster coordinate transforms.
79 producer CPU tests, Clippy, wasm32 and dependency guards pass; independent
source and final producer evidence reviews accepted. The [main201 integration](docs/spikes/physics-manifolds-main201.md)
passes all eight checks with 79 CPU cases; final integration review and the required fast check passed, and PR202 is merged. Solver/island reciprocal ownership, dispatcher/recycle payloads, full
world composition and live/save admission remain open.
[Manifold contract](docs/spikes/physics-inert-manifolds.md).

## Exact physics saves — inert contact leaf payloads verified locally

The next std-only inert component preserves complete tracked-point warm-start
data and ordered solver-contact rows, with exact float/feature/index bits and
bounded hostile decoding. Sixty-five CPU tests, Clippy, wasm32 and dependency
checks pass. Actual 2D/3D stepped contacts include nonzero warm starts/friction,
retained recycle references, unchanged sleeping rows and wake changes.
Independent source and final producer evidence review are accepted; [main199 integration](docs/spikes/physics-contact-main199.md) passes eight checks and 65 CPU cases. Final integration review is accepted; PR200 is merged after its exact-head fast check. Shape/owner/manifold
composition and full exact continuation remain required; live construction and
physics-save admission stay refused.
[Contact leaf contract](docs/spikes/physics-inert-contact-points.md).

## Exact physics saves — inert attachment ownership verified locally

The next std-only inert component preserves exact ordered body/collider
attachments and native local-pose bits against two checked arena layouts.
Fifty-two CPU tests, Clippy, wasm32 and dependency checks pass, including actual
2D/3D public attach/detach/reparent/remove/reuse observations and hostile decoding.
Independent producer source/evidence review is accepted; [current-main197 integration](docs/spikes/physics-attachments-main197.md) passes all eight checks with 52 CPU cases; independent final integration review and the renderer-only main198 qualification are accepted. PR199 is merged after the exact-head fast check; no snapshot or dependency change was introduced by that join. Enclosing-owner authentication,
complete body/collider/contact state and exact continuation remain required;
no live construction or physics-save admission is enabled.
[Attachment contract](docs/spikes/physics-inert-attachments.md).

## Exact physics saves — inert components merged; live restoration remains closed

PR197 merges the independently reviewed snapshot, arena and graph validators with 40 passing CPU cases. The normal dependency graph is std-only. No live constructor or runtime consumer is enabled, and `PhysicsUnsupported` is unchanged. See [current integration evidence](docs/spikes/physics-inert-main196.md).

The next inert format preserves node/edge order and validates both linked
adjacency sides, reciprocal links, complete coverage and dense payload mappings.
Forty CPU cases, Clippy, wasm32 and dependency checks pass, including actual
2D/3D native swap-removal order observations. Independent producer review is accepted; the [combined main196 checks](docs/spikes/physics-inert-main196.md) pass with independent final integration acceptance. PR197 is merged after its exact-head fast check. Contact/payload/solver ownership
and full native composition remain required; no live restore or physics-save
admission is enabled. [Graph contract](docs/spikes/physics-inert-graphs.md).

## Exact physics saves — inert arena validation verified

The next checked-format component preserves Rapier arena slots/free-chain order,
generations, dense payload-row mappings and stale/invalidated Coarena rows.
Current integration passes thirty CPU cases, Clippy, wasm32 and dependency checks, including actual
pinned 2D/3D one-way observations. Independent source and final evidence reviews
are accepted. There is still no live
construction or physics-save runtime route; contact/island/payload/native owner
composition remains required. [Arena contract](docs/spikes/physics-inert-arenas.md).
The [combined current integration](docs/spikes/physics-inert-main195.md) preserves
all accepted crate source and existing pinned package identities; independent
final integration review is accepted, and the Parry prerequisite is merged in PR196.

## Exact physics saves — inert record validation verified

A new dependency-free normal crate validates versioned inert BVH/VecMap and
closed rigid geometry records, with bounded allocation/traversal and exact field
preservation. Eighteen CPU tests, scoped Clippy, wasm32 compilation and dependency
checks pass, including trusted pinned Parry observations and malformed graph
controls. Independent source and final evidence reviews are accepted.
No live solver constructor or runtime consumer
exists; `PhysicsUnsupported` remains unchanged. Complete checked solver/native
owner composition and exact save continuation remain required.
[Contract and bounds](docs/spikes/physics-inert-snapshot.md).

## Parallel physics tree ownership — scoped correction verified

Vendored Parry 0.31.1 fixes overlapping mutable borrows in parallel BVH updates
and refitting, with duplicate rejection before mutation and explicit source
provenance. Both physics modes, browser compilation, script compilation and
Clippy pass; independent source review is accepted. Additional Miri experiments
remain unsuccessful due to Crossbeam diagnostics and are recorded separately.
This does not enable persisted physics saves, complete a dependency soundness
audit, or establish a performance gain.
[Producer scope](docs/spikes/parry-parallel-ownership.md). The current-main195 union passes 18 CPU cases in each physics mode, all-target Clippy, browser/script checks and four isolated patched dependency graphs. Actual source-change/restore controls verify runtime identity invalidation for both Parry and the optional profiler. [Integration evidence](docs/spikes/parry-main195.md) has independent final acceptance and is merged in PR196 after its exact-head fast check. No physics-save admission is enabled.

## Cooked renderer integration — CPU checks complete

The accepted sprite, UI, text and material producer changes are composed with
optional private Vulkan pipeline storage. Default and cooked configurations pass
87 and 113 CPU cases respectively (overlapping coverage), with 89 GPU cases
explicitly ignored in each local run. All-target Clippy, web compilation and
workflow/dependency checks pass. Claude 0062–0064 accepted the immutable producer
Metal results; these remain producer evidence. Release CI retains defaults and
adds cooked GPU checks with exact Metal/Vulkan/Dx12 adapter assertions, plus the
Linux fresh-process cache test. Current composed GPU execution, Vulkan/DX12
acceptance, cache benefit and reference-phone gates remain pending. Features
remain opt-in. [Integration evidence](docs/spikes/cooked-renderer-main186.md).

## Persistent Vulkan pipeline cache — CPU verification

An optional renderer cache now wraps the complete 31-pipeline startup catalog
with private, bounded, adapter/driver-bound storage and atomic publication.
Filesystem/CPU checks, browser compilation and dependency checks pass. A fresh-
process Vulkan corruption/reopen test is prepared for Linux release CI; actual
Linux driver execution and measured cache benefit remain pending. Default
rendering is unchanged. Metal/DX12 storage, per-level recording, asynchronous
fallback and reference-phone gates remain open.
[Contract and evidence](docs/spikes/persistent-pipeline-cache.md).

## Extended cluster counts/masks observer — Metal numerical parity passed

The corrected test now resolves real cooked model geometry before retained-frame
submission. Both frozen default/cooked-cluster GPU binaries pass on Apple M5 Pro
Metal: all 84 named raw count/mask records match exactly across cameras, viewport
sizes/origins, light-count boundaries, motion, clearing and resize. Three CPU
cases per configuration and both all-target Clippy checks also pass. The change
is test-only; GPU processes are reaped and the exclusive lease released. This
adds numerical producer evidence, not appearance, other-backend, phone or
performance acceptance. [Scope and evidence](docs/spikes/cluster-grid-readback.md).

## Fixed cluster assignment shaders — CPU/compiler checks passed

The opt-in compute cooker preserves the cluster WGSL/algorithm while emitting
fixed MSL, SPIR-V and HLSL. Explicit allocation lengths and static workgroup
locals cover Metal passthrough's missing reflection; a checked workgroup-only
SPIR-V decoration correction passes independent Vulkan validation. CPU checks,
clusters-only/combined/default/browser configurations, Clippy and the runtime
graph pass. Independent CPU/compiler source review is accepted. Native GPU, Claude appearance,
Vulkan/DX12 execution and phone performance gates remain open.
[Contract and evidence](docs/spikes/cooked-cluster-shaders.md).

## Fixed material shaders — CPU/compiler checks passed

The opt-in material path cooks the unchanged imported-material, light and shadow
sampling shaders to native backend artifacts. Its explicit Metal storage-length
binding covers metadata absent from passthrough; bounds and loop checks remain.
All seven CPU/static checks pass: 92 enabled cases, 63 default-library cases,
Clippy and browser portability. Both MSL stages compile and link. Independent source review passes.
Claude 0064 accepted the producer Metal GPU/appearance packet. Current composed
GPU execution, grid compute, full renderer cooking and reference-phone gates
remain open; optional Vulkan cache CPU integration is described above.
[Contract and evidence](docs/spikes/cooked-material-shaders.md).

## Cooked world-space text shaders — CPU and Metal compilation passed

The fixed text shader path passes independent source review, 90 enabled CPU
tests, Clippy, wasm32 compilation and 63 default library tests. Both generated
stages compile and link with the actual Apple Metal toolchain. Existing WGSL,
glyph placement, coverage and depth behavior are unchanged. Claude 0063 accepted the producer Metal GPU/appearance packet. Current composed
GPU and other-backend execution remain pending; this is not a rendering or device
gate. [Scope and evidence](docs/spikes/cooked-text-shaders.md).

## Cooked screen-space UI shaders — CPU and Metal compilation passed

The opt-in fixed UI shader family passes independent source review, 87 enabled
CPU tests, Clippy, wasm32 compilation and 63 default library tests. Existing WGSL,
layout, theme and blending are unchanged. Both stages compile and link with the
actual Apple Metal toolchain. Claude 0063 accepted the producer Metal GPU/appearance packet. Current composed
GPU and other-backend execution remain pending. No rendering or phase gate is claimed. [Scope and evidence](docs/spikes/cooked-ui-shaders.md).

## Cooked sprite shader family — CPU and compiler checks passed

The existing sprite shader now has an opt-in fixed backend artifact path with
checked stage/resource/host layouts and preserved sampling math. CPU tests,
Clippy, default/web compilation and actual Metal stage compilation/linking pass;
independent source review is accepted. Claude 0062 accepted the producer Metal GPU/appearance packet. Current composed
GPU execution, other shader families and device performance requirements remain pending.
[Scope and evidence](docs/spikes/cooked-sprite-shaders.md).

## Checkpoint17 restart quota assertion — test correction accepted

Linux and Windows baseline conformance failed the same restart quota test;
the original assertion omitted its returned error. The test-only correction merged in main188 and
isolates structural quota checks from the production CPU deadline and strengthens
maximum/overflow/rollback controls. Local 127 script cases and Clippy pass;
actual corrected hosted results remain pending. Candidate17 stays frozen and
release remains at accepted checkpoint14.
[Evidence](docs/spikes/restart-quota-main187.md).

## Authored Android static Wasm — producer execution passed

Ordinary Android export now admits the reviewed captured wasm2c module route
with complete bundle/producer/target/physics checks and exact NDK C compilation
audits. Both fresh shipping physics archives pass; six fresh source/cache/build/
map-deleted ARM API37 emulator processes match all 17,856 observed fields and
614,400 PCM bytes each, with twenty-six exact refusal controls. Public import,
atomic selection/mode Undo/Redo/journal reopen and four Wasmtime processes also
pass. Owned software AVDs were removed; the retained port-reuse failure was
resolved by rerunning only missing callers, without rebuilding archives.
Source/probe and final producer execution reviews are accepted. This historical
intake state is superseded by the accepted main189 composition and PR190 merge
(see [current composition](docs/spikes/authored-android-wasm-main189.md)).
API24 compilation is not minimum-OS execution proof.
No physical-phone, performance, GPU, live audio, signing or phase gate is claimed.
[Scope and retained evidence](docs/spikes/authored-android-wasm.md).

## Authored iOS static Wasm — producer execution passed

Ordinary iOS export now accepts one public captured Wasm scene module through
pinned wasm2c and the existing native bulk runtime. Complete captured/bundle,
explicit iOS destination, selected physics graph, five actual solver artifacts,
shipping Rust codegen and target C commands are checked. Both physics families
have simulator/device archives; twelve fresh C/Swift simulator processes match
17,856 full-field scalars and 614,400 PCM bytes each after private source/cache/
build/maps removal, and fifty-two exact refusals pass. Both device archives link
unsigned with C/Swift; no device execution is claimed. The first simulator
compiler output was reused after a reviewed observer-only correction, explicitly
qualified rather than called a new build. A separate simulator teardown failure
is retained; the complete retry passed without source/archive changes.
Source/probe and final execution reviews are accepted. This historical intake
state is superseded by the accepted main186 composition and PR187 merge
(see [current composition](docs/spikes/authored-ios-wasm-main185.md)).
Its original Android refusal is superseded by the
separate Android proof above; mixed per-entity TypeScript and static guest
checkpoints remain refused. No phone, minimum-OS, performance,
rendering, signing or phase gate is claimed.
[Scope and retained evidence](docs/spikes/authored-ios-wasm.md).

## Authored Core runtime bindings — merged in main185
## Opt-in native CPU profiler — locally accepted

Connected runtime/system/script zones and bounded player summaries pass actual
CPU checks. Explicit trusted headless exports bind summary/Tracy features; the
unchanged default path starts no client. Two fresh shipping packages preserve
full 3,600-tick reports and PCM with profiling off/on. An actual opted-in loopback
Tracy capture contains 20 zone types and four memory/allocation plots; all owned
processes are reaped. Rust requested bytes, QuickJS heap and RSS are distinct.
Independent source and final evidence review are accepted; the historical
intake state is superseded by the accepted main194 composition and PR195 merge.
Native-window/device capture, allocation stacks and the full profiler/performance
gates remain open. [Contract/evidence](docs/spikes/native-cpu-profiling.md).

## Dynamic-resolution producer — Metal review complete

Claude 0067 and root accepted the bounded producer scope: 89 existing GPU cases,
48 numeric attempts and 12 appearance frames, with original helper failures
retained and corrected within budget. Physical UI and viewport mapping pass;
nearest expansion remains visibly degraded and opt-in. Exact host feedback uses
recorded witness bounds. The producer-only composed-integration status is
superseded by the accepted main197 composition and PR198 merge. Full temporal
upscaling, MetalFX and reference-phone gates remain open.
[Evidence](docs/spikes/dynamic-resolution.md).

## Authored Core runtime bindings — CPU proof accepted

The 0060 mesa now binds its existing localized HUD and audio sources to the
unchanged current controller, with R/Start whole-session restart. Public atomic
authoring, Undo/Redo and reopen pass; all art/layout inputs are preserved. The
full 2,156-tick 48 kHz route, loss, locked exit, four locale readbacks, repeated
cues and four complete packaged reset routes pass. Fresh-session logs/entities/UI
and concatenated PCM match exactly after source/map/build deletion. Independent
final integration review and exact-head fast check passed; PR185 is merged.
No GPU/window/device gate is claimed;
historical offscreen provenance and Claude's visible restart hint remain open.
[Contract/evidence](docs/spikes/core-authored-runtime-bindings.md) and
[current-main integration proof](docs/spikes/core-main182.md).

## Core authored-level locked-exit proof — merged in main185

The retained 0060 locked run died before reaching the exit; its old assertion was
a false positive. A replacement public-input recording reaches the exit alive
with 3/4 pickups and observes actual positions/inventory for 30 consecutive ticks.
Three real CPU runs, seven focused oracle tests and independent root review pass;
original win/loss reports and PCM are unchanged. Controller, art and authoring
stay untouched. PR185 includes this correction and its
[current-main integration proof](docs/spikes/core-main182.md). No new visual,
device or performance gate is claimed. [Correction/evidence](docs/spikes/core-locked-exit.md).

## Bounded offline PCM streaming — CPU shipping proof accepted

The ordinary player now stages PCM with one tick buffer and an 8 KiB encoder,
then syncs and publishes without clobbering an existing output. The former
one-million-frame capture cap is removed; existing tick/sample-rate bounds,
diagnostic failed-observation output, fractional checkpoints and restart clocks
remain intact. 75 CPU tests and scoped Clippy/static-module/graph checks pass.
A fresh source/map/build-deleted Core package completes 2,156 ticks at 48 kHz
(1,724,800 frames), repeats exactly and matches the prior player for its first
1,000 ticks. Eight actual process controls cover the old refusal, diagnostic PCM,
and precise publication errors. Independent source and final execution reviews are accepted; PR181 is merged after the exact-head fast check. No whole-engine memory, native-window,
GPU or device/performance gate is claimed.
[Contract and evidence](docs/spikes/native-streaming-pcm.md).

## Streaming PCM — current main180 integration passed

The ordinary player stages bounded per-tick PCM and hashes incrementally. Current-main
validation passes 75 CPU tests and all eleven checks. A fresh source/map/build-deleted
Core export passes eight actual process controls over the complete 2,156-tick route,
with exact repeat PCM and preserved old prefix/no-clobber/diagnostic behavior.
Final integration review and the exact-head fast check passed; PR181 is merged. Native-window, device,
performance and appearance gates remain separate. [Evidence](docs/spikes/native-pcm-main180.md).

## Whole native session restart — CPU shipping proof accepted

Opted-in native TypeScript hosts now replace the complete world, VM, input/UI and
mixer from a trusted startup seed. Host-step/output chronology stays monotonic;
held controls and stale native text/pointer ownership cannot enter the new game.
Current-main integration passes 355 CPU tests and 24 fresh source/map/build-deleted
Core/Planar runs, including
exact fresh-state and concatenated static/streamed PCM comparisons. Independent
source and final execution reviews and the exact-head fast check passed; restart is merged in PR180.
Native-window/device/appearance acceptance, Wasm/headless restart ownership and
physics save continuation remain open. [Contract and evidence](docs/spikes/native-session-restart.md).

## Authored iOS fixed physics — current integration execution passed

Ordinary iOS export now selects the project's deterministic or fast kernel and
audits the selected normal graph, all five actual target solver artifacts and
shipping codegen. Four fresh archives pass; twelve source/cache/build/map-deleted
C/Swift simulator processes reproduce 864 checks, 432 complete log records and
exact nonzero PCM. Thirty-two precise refusals include checksum-valid opposite-mode
packages in both directions. Both-mode device archives and unsigned C/Swift links
pass without device execution. The accepted source/probe is unchanged, and the
complete producer execution packet has independent final acceptance. The composed
main181 runtime now passes nine checks, two fresh simulator archives, twelve actual
C/Swift positives and thirty-two precise refusals. Both current device target
compilations pass; original unsigned device links remain producer-scoped. Final
integration review and the exact-head fast check passed; PR183 is merged.
[Current integration evidence](docs/spikes/authored-ios-physics-main181.md).
No phone, performance, rendered-output, signing or phase gate is claimed.
[Scope and retained evidence](docs/spikes/authored-ios-physics-modes.md).

## Authored iOS project archives — merged in main178

The separate trusted-source iOS exporter cooks public-authored TypeScript imports,
props and scene bindings into fresh shipping archives plus captured data. The
current main177 union passes 205 affected Rust cases, Clippy and ten exporter
checks. Both fresh target archives pass exact shipping codegen and five actual
physics dependency audits. Six C/Swift simulator processes reproduce 432
observations, 216 log records and exact nonzero PCM after private source, cache,
build and maps are removed; fourteen precise refusal controls pass. Device
archives and unsigned caller links pass, without device execution. Independent
producer, integration-source and final execution reviews are accepted; the
exact-head fast check passed and PR178 is merged. This historical proof was deterministic-only; the separate producer above adds fast iOS projects. Desktop cross-target
export and mobile static Wasm remain denied in this slice. No signing,
mobile window/audio host, phone performance or phase gate is claimed.
[Current source and evidence](docs/spikes/authored-ios-main177.md).

## Native worker owner — merged in main179

The trusted developer owner builds the selected kernel, verifies worker identity,
and enforces request deadlines and complete process-tree cleanup. Actual sessions
preserve their original kernel while public authoring changes the next mode;
state matches ordinary native Play after private source removal. Both compiled modes pass all-target tests and Clippy; actual owned-process proofs
and full tools checks pass on main178. Independent final integration review and required fast check passed; PR179 is merged. Editor GPU embedding and Windows execution remain separate.
[Contract and evidence](docs/spikes/native-worker-owner.md).

## Retained native CPU worker — merged in main179

A native worker drops authoring state after startup and retains only its disposable
runtime composition. Both matching physics binaries pass 65 all-target headless tests each,
Clippy and real source-deleted process checks, including duplicate-step fencing,
strict protocol refusals and disconnect cleanup. The three mode-aware test fixtures
use public commands and preserve explicit foreign-mode refusal. Independent final review and fast check passed; PR179 is merged; [integrated evidence](docs/spikes/native-worker-main178.md). Automatic editor process selection and GPU viewport
embedding remain open. [Contract and evidence](docs/spikes/native-play-worker.md).

## ARM Android bytecode candidate — experiment accepted

The default-off trusted numeric probe has accepted source, 22 admission tests and
nine harness cases. An actual local Android 17/API 37 ARM emulator passes all 16
C ABI controls; complete numeric/game/PCM records match prior macOS/iOS callers.
Independent final execution review is accepted. That immutable candidate keeps default
and authored admission denied. The separate authored Android implementation below
now has independently accepted shipping/emulator evidence; no phone or performance
gate is claimed by either slice.
[Candidate contract and retained attempts](docs/spikes/android-bytecode-candidate.md).


## Physics process selection — local execution passed

The developer launcher builds the authored physics variant and runs a private
executable copy. Public setting edits, Undo/Redo and native execution pass in
deterministic, fast and deterministic again. Fresh ordinary shipping exports
of both modes run after source/cache/map/build removal with exact native
observations and PCM; five actual solver feature sets and shipping codegen are
audited. The full tools suite passes with four Python skips. This process/export path is now merged through main177; automatic editor worker
selection, physics saves,
reference-device performance and phase gates remain open.
[Scope and retained evidence](docs/spikes/physics-process-selection.md).

## Physics process/export selection — current main176 verified

Developer Play and ordinary desktop export now choose the fixed matching kernel
from the public project setting. Selected normal dependency audits replace the
workspace-unified metadata false refusal seen in frozen checkpoint15. The current
CPU integration passes 15 checks and two unchanged-package replays; all producer
compile/cook inputs remain exact. Independent final closure is accepted. Automatic
editor workers, physics saves and device/performance gates remain separate.
[Evidence](docs/spikes/physics-process-main176.md).

## Project physics selection — current main173 verified

The public command bus atomically selects deterministic or fast physics, with
legacy deterministic defaults preserved. Cook/Play explicitly refuse a mismatched
process before assets or source fallback. All 70 schemas and strict TypeScript
contracts match. Current integration reuses byte-identical three-mode Rust checks
and reruns schema/generation/guard checks; final independent review is accepted. Automatic editor
process switching remains separate; developer/export selection is recorded above.
[Integrated contract and evidence](docs/spikes/physics-project-main173.md).

## Pearl Cove presentation — current main173 verified

Accepted gameplay hooks, original Claude art and localized HUD are composed.
Current matching-headless verification passes seven original, seven presentation
and seven authored Cove scenarios, public Undo/Redo/reopen and four precise binding
refusals. Source, art and test bytes stay unchanged; Claude's offscreen appearance
approval remains scoped to its frozen packet. Original two shipping exports remain
separately accepted producer evidence. Final independent integration review is accepted; native
window, restart, audio and physical-device/performance/physics-save gates stay open.
[Integrated source and evidence](docs/spikes/planar-presentation-main173.md).

## Physics-mode foundation — current main172 verified

The full process graph forwards deterministic, fast and explicit eight-lane
physics choices without changing defaults. Cooked scenes retain their header
size and bind the exact compiled kernel before world allocation. Current main172
passes 353 CPU tests in each mode, Clippy and headless/player/mobile-library/editor
compilation with actual target feature receipts. Five additive Cargo seams preserve
current mobile and host capabilities; production Rust source matches the reviewed
foundation. Independent final integrated review is accepted. Project settings and automatic
process/export selection remain separate work; physics-save refusal stays closed.
[Integrated source and checks](docs/spikes/physics-mode-main172.md).

## Pinned mobile bytecode compatibility — merged main171

Fresh source cooks bind exact producer/destination and the pinned QuickJS ABI.
Actual C and Swift callers each pass the same complete numeric/input/failure
records on macOS and an iOS 27.0 ARM simulator (32 controls per platform). iOS
device archives compile and link only. Native script/cooker tests, union Clippy
and lean dependency checks pass; independent final source/execution review is accepted. Android
cross-target admission stayed denied in that immutable slice; the later Android
route below is separately reviewed. The authored iOS route above is a separate
shipping-profile proof. No
physical-phone or performance gate is claimed.
Current main169 integration passes all 16 checks with new matching archives and
64 actual C/Swift controls; final integrated audit and exact-head fast CI passed
before PR171 merged.
[Contract and evidence](docs/spikes/mobile-bytecode-compatibility.md);
[current integration](docs/spikes/mobile-bytecode-main169.md).

## Ordinary desktop exporter — library audit corrected

The player library and thin executable share a crate name. The shipping audit
now checks both target units independently, preserving required optimization,
source-compiler exclusion and final executable fat LTO. Six parser regressions
and 63 scoped tools tests pass; root source review is accepted. The original
failed export remains retained; two corrected fresh Planar packages now pass
the four-unit codegen audit. [Evidence](docs/spikes/native-export-player-units.md).

## Headless UI and reusable player — merged main169

The shared recorded UI route and optional mobile C ABI pass all 19 integrated
checks on the accepted audio ABI union. Real C/Swift callers of the new immutable
macOS archive pass 32 controls/39 commands, including native held/released movement,
exact failures and no-clobber output behavior. Producer source bytes and dependency
versions are unchanged; independent final integration review and exact-head
fast CI passed before PR169 merged. iOS/Android compatibility stays separate; no GPU/window/device gate.
[Integrated commands and actual callers](docs/spikes/headless-ui-mobile-integration.md).

## Mobile runtime callers — macOS C/Swift CPU proof accepted

Actual callers of the frozen native runtime archive pass 32 controls / 39
commands with typed cooked input-driven movement, precise failure reports,
repeat PCM and create-only outputs. Fixture Clippy and authoring-free graph
checks pass; source/execution review is accepted. The later trusted-source
compatibility route above adds iOS simulator execution and device archive links.
Android NDK/CI and physical-phone/render/device gates remain pending; the old
document smoke is not reused as game execution evidence.
[Source and measured scope](docs/spikes/mobile-runtime-callers.md).

## Shared cooked-player library

The ordinary packaged player now has a reusable Rust entry and optional bounded C
ABI. Local macOS CPU/CLI tests and native-window/static-module compatibility pass;
actual macOS C/Swift caller verification passes as recorded above. The first iOS
archive refusal remains retained; the separately verified trusted-source route is
described above. The separate authored iOS archive route is recorded above. No physical-device
or performance gate passes from the library extraction. [Contract and evidence](docs/spikes/mobile-runtime-library.md).
## Rust/Wasm audio transport — merged main168

The accepted ops59–61 pass 483 actual-host cases and all 12 integration checks.
A fresh ordinary exported player runs three source/map/build-deleted 96-tick cases
with 17,856 declaration scalars and 248 observations each, exact native-oracle PCM,
and thirteen precise no-output refusals. All accepted source inputs remain exact;
independent integration review and exact-head fast CI passed before PR168 merged. Desired state
snapshots do not expose mixer cursors; whole guest saves and device gates stay open.
[Current integration evidence](docs/spikes/native-wasm-audio-main166.md).

## Release checkpoint 14 — promoted main160 baseline

All 23 required contexts and eight exact-candidate push workflows passed.
Release `2444d81f` has the exact tested main160 tree. Actual hosted AppKit retirement
and numerical UI checks pass; the logs identify Windows DX12 software rendering,
Linux llvmpipe/Vulkan and macOS paravirtual Metal. Main161+ features remain outside
this release baseline. Failed checkpoints stay historical; Phase 1, reference-phone,
appearance and complete engine/game gates remain open.
[Promotion and retained evidence](docs/spikes/release-checkpoint-14.md).

## Wasm HUD controls — merged main166

The unchanged accepted ABI passes 469 actual-host cases, Clippy and target/tooling
checks. A fresh ordinary shipping player passes three source/map/build-deleted
12-tick runs with exact report/PCM equality and six precise no-output refusals.
Independent integration review and exact-head fast CI passed before PR166 merged. Module checkpoints,
GPU/window/appearance, device and full SDK gates remain open.
[Current integration evidence](docs/spikes/native-wasm-hud-integration.md).

## Planar gameplay — merged main165

All five reviewed sample/workflow inputs remain exact. Seven native gameplay
scenarios, strict types, 28 policy cases and actionlint pass with a matching engine
binary. Independent review accepts the source/receipts. Original shipping proof
remains explicitly based on main160; exact-head fast CI passed before PR165 merged. The
numerical room does not establish authored visuals, device or physics-save gates.
[Current integration evidence](docs/spikes/planar-sample-main163.md).

## Core presentation bindings — merged main164

The two reviewed sample/helper inputs are unchanged. Strict types, seven native
scenarios and a fresh ordinary source/map/build-deleted win/loss/repeat proof pass
on the integrated native HUD/audio engine. Actual text/locale/result parenting,
repeat PCM and terminal silence are verified. Independent integration audit and exact-head fast CI passed before PR164 merged.
Authored appearance/device gates remain separate.
[Current integration evidence](docs/spikes/core-presentation-integration.md).

## Physics build variants — current-main kernel checks passed

The 2D/3D physics crates build separately with deterministic math, hardware math,
or the opt-in eight-lane candidate; native parallel stepping stays enabled.
The same ten behavior tests and Clippy pass in each variant on main169, and actual compiler records
confirm the intended features. Both base variants compile for wasm32 without
parallel features. Existing callers retain deterministic defaults; per-project
selection, matching play/export processes, device measurements and exact-replay
gates remain open. [Evidence](docs/spikes/physics-runtime-variants.md).

## Core Sample presentation bindings — locally verified

Optional authored role maps connect gameplay to localized health, collection and
objective text, result labels, music and one-shot cues. The controller queues
native commands only for state changes and leaves layout and media to the level
builder. Seven original gameplay scenarios and strict typing pass. A fresh
ordinary export passes win, loss and repeat processes after its source/cache
inputs are removed; actual committed HUD text, French locale switching, result
parenting, repeat PCM and terminal music silence are verified. Independent
source and execution review is accepted. The initial helper's full-health win
expectation was corrected against the same frozen player; game code was unchanged.
This is a numerical fixture, not the finished authored level or device gate.
Headless recorded UI routing and physics save support remain separate open work.
[Contract and retained evidence](docs/spikes/core-presentation-bindings.md).

## Native physics saves — checked persistent boundary still open

A bounded Rapier 0.36 diagnostic preserves trusted typed 2D/3D solver state and
subsequent overlap events across eight cases / 960 continuation ticks. Rebuilding
only current poses, velocities and sleep state diverges at nonzero checkpoints,
including changed trigger events. Production `PhysicsUnsupported` remains intact;
no untrusted solver graph decoder or world-v12 section has been adopted. The
source audit identifies the private graph families a checked format must cover
and the storage/load limits of deterministic replay. Independent source/evidence
review accepted this bounded diagnostic; full physics saves remain open.
[Diagnostic, source barrier and next options](docs/spikes/native-physics-snapshot-boundary.md).
[Current-main kernel and diagnostic checks](docs/spikes/physics-boundaries-main169.md) pass; independent final integration review is accepted and fast CI remains required.

## Native audio transport — merged main163

541 affected CPU cases, Clippy and target/contracts checks pass. A fresh ordinary
package passes four source/map/build-deleted playback/continuation processes with
exact save and PCM equality plus ten precise refusal controls. Integration source
seams, final evidence and exact-head fast CI passed before PR163 merged. Wasm
transport, Core presentation and physical audio/device gates remain separate.
[Current-main evidence](docs/spikes/native-audio-transport-main162.md).

## Native HUD controls — merged main162

All 20 accepted inputs remain exact. Current-main CPU checks pass 444 affected
and 10 player cases plus Clippy/target/contracts/dependency checks. A fresh
ordinary package passes five source/map/build-deleted processes with canonical
shaped text/focus and exact save/PCM continuation, plus three precise refusals.
Independent integration review and exact-head fast CI passed before PR162 merged. Wasm parity,
window/IME/appearance and device gates remain separate.
[Current-main evidence](docs/spikes/native-hud-main161.md).

## Native HUD controls — CPU and public shipping proof verified

Typed native/TypeScript controls now update existing widget text/disabled state
and world Text content through the atomic gameplay transaction. Readonly
pre-command declarations, role/lifetime validation, aggregate quotas, localized
literal results and composed input/shaping reconciliation are tested. The 444
affected cases plus 10 player tests, all-target Clippy, strict typing, wasm32 and
normal-runtime dependency guards pass. Independent source and final execution
reviews are accepted.
Five fresh source/map/build-deleted player processes verify changed shaped text,
pre-command reads, exact save/PCM continuation; three exact refusals publish no
outputs. Final independent execution audit is accepted.
Wasm HUD parity, native/GPU appearance and complete Game UI remain open.
[Contract and limits](docs/spikes/native-hud-controls.md).

## Native audio transport — fresh shipping playback verified

Stop, restart and seek now pass the typed native transaction and retained mixer
checks. The six-package suite passes 530 tests and scoped Clippy. A fresh ordinary
export passes four separate player processes with byte-exact audio/save
continuation and ten specific refusals after private inputs are removed. Independent
execution review accepts the evidence; the historical intake state is superseded
by the accepted main162 composition and PR163 merge. Physical audio,
Wasm SDK parity, Core Sample integration and phone gates remain open.
[Evidence and limits](docs/spikes/native-audio-transport.md).

## Complete root motion — merged main161

The native/TypeScript path and additive Wasm operations 53–55 now pass current-main native
606-case and separate actual-host 453-case CPU suites, scoped checks and three
fresh ordinary 32-tick source-deleted shipping processes. Each run matches 22,464
oracle scalars and 256 selected vectors, exact logs/observations/PCM; 11 precise
refusals publish no output. All 119 producer inputs remain byte exact. Independent
current-main review and exact-head fast CI passed; merged main161; device, appearance,
CUBICSPLINE root import and whole-module checkpoint limits remain explicit.
[Current-main evidence](docs/spikes/native-root-motion-main160.md).

## Connected root motion — native/TypeScript CPU checks passed

Cooked17/18 root catalogs now drive independent exact graph/clip clocks,
interval deltas, bodyless/kinematic/character policies and staged root-strip then
IK sampling. World11 and QuickJS session continuation preserve active/paused
reverse fades; owned TypeScript cycles/ticks remain exact BigInt arrays. The
600-case CPU suite, scoped Clippy, wasm32, dependency and SDK checks pass.
Independent production source and retained evidence reviews accept the native
connection. The composed Wasm53–55 boundary now passes 453 actual-host cases,
Clippy, wasm32 and 240 Python checks; independent source and actual-host final
evidence reviews are accepted. Three fresh ordinary wasm2c package processes
now each match 22,464 native-oracle scalars plus 256 public vector checks after
private source/cache/build/map deletion. All eleven exact runtime refusals pass.
Independent final review accepts the
[public package packet](docs/spikes/native-root-motion-package.md); publisher
integration remains pending.
[Wasm contract and evidence](docs/spikes/native-root-motion-wasm.md). Physics saves,
general affine/CUBICSPLINE roots, rendered/device and full Phase1 gates remain
open. [Contract and measured limits](docs/spikes/native-root-motion-connected.md).

## Root-motion captured catalogs — admission foundation verified

A lean `incant_anim` crate now owns checked immutable root tracks and their bounded
binary payload, sharing the existing full-pose interpolation arithmetic. Typed
root selection uses normal command transactions, Undo/Redo and journal reopen;
the captured-model loader validates topology and deduplicates catalogs. All 232
affected tests, scoped Clippy, wasm32 and dependency checks pass. The catalog
round-trips and samples after deleting source and cache data. Independent source
and evidence review is accepted. The later connected native/TypeScript layer is
recorded above; this historical admission result alone does not establish complete
root motion. [Admission evidence](docs/spikes/native-root-motion-catalog.md).

## Cooked root-motion numeric sampler — locally verified

A bounded CPU sampler now reads rigid local translation/rotation deltas from
immutable cooked tracks. Exact signed cycle cursors support forward/reverse, wrap
and multiple-cycle motion, with ordered rotation/translation composition. Selected
scale or matrix-only tracks refuse explicitly. All 128 assets/runtime cases,
all-target Clippy, wasm32 visual and dependency checks pass; 4096 query iterations
allocate nothing after admission. Independent source and retained evidence review
accepts the implementation without findings; publisher integration remains pending.
Authored root selection, graph/clock integration, pose extraction, body movement,
save/SDK controls and rendered/device gates remain open; this is not complete
root motion or Phase 1. [Contract and evidence](docs/spikes/native-root-motion-core.md).



## Planar Sample gameplay — native and exported checks verified

The top-down 2D controller uses native planar sensors, solid tilemap collision
and bulk velocity updates. Public atlas import and command authoring pass
Undo/Redo and journal reopening. Seven native scenarios and seven fresh ordinary
shipping processes pass: keyboard/gamepad win, hazard loss, blocked wall, locked
exit, normalized diagonal motion and repeatability. Source/cache/build/map inputs
are removed from the exported proof; logs, observations and silent PCM match
native development exactly. Independent source and execution reviews are accepted.
The 2D presentation is now assigned to Claude Opus 5.5 at explicitly verified Max.
The numerical white atlas is not sample art; rendered/device/performance and
physics checkpoint gates remain open. Optional follow camera and simultaneous
hazard/exit branches are source-reviewed but not in these seven scenarios.
[Contract](samples/planar/README.md) and [evidence](docs/spikes/evidence/planar-sample-gameplay.json).

## Complete compound Wasm — merged main159

Additive operations 47–52 preserve 0–46 and all older definitions. The final
actual Wasmtime/wasm2c suite passes 436 tests plus all-target Clippy; the ordinary
public exported player passes three fresh 56-tick runs after deleting its
authoring/cache/build inputs and debug maps. All 64,512 scalar oracle comparisons,
report/PCM parity and 23 exact refusal controls pass. Producer source and execution are independently accepted. Current-main integration
passes the same 436 actual-host tests, scoped checks and three fresh player runs
through an explicitly reviewed interrupted-build recovery. The runtime inputs
remain unchanged across main157–158; final integration review is accepted and
the exact-head fast check passed and the slice merged as main159. Root-motion may append reserved 53–55 from the immutable source
ancestor. Whole-module checkpoints and visual/device gates remain open.
See [scope and evidence](docs/spikes/native-wasm-compound.md) and the
[current-main recovery record](docs/spikes/native-wasm-compound-main158.md).



## macOS retirement probe — callback repair verified; native run pending

Checkpoint13's general macOS job failed when the probe received a queued callback
following final window destruction. A probe-only terminal guard now preserves all
256 cycles and the first typed error while refusing later native work. Two red-to-green
regressions, native-feature build and Clippy pass. A bounded real-idle check did not
launch the executable because user input remained active. The comprehensive native
probe must still pass before release; checkpoint13 is terminal and unpromoted.
[Scope and preserved failure](docs/spikes/macos-retirement-late-events.md).



## Core Sample gameplay — main156 integration verified

A native TypeScript controller now covers movement, jumping, navmesh enemies,
pickups, damage cooldown and terminal game states. Seven actual development
scenarios and seven ordinary source-free shipping runs pass, with identical
observations, logs and PCM; an additional exit-proximity control confirms that
a missing pickup prevents winning. Public authoring, Undo/Redo and reopening
pass, and Play leaves the project unchanged. Existing desktop checkpoint jobs
now include the seven cheap headless scenarios; their hosted results are pending.
Current-main integration also passes seven native-play scenarios with 53 checks,
strict sample types, workflow lint and 25 CI policy tests. Independent integration
review and the exact fast check passed; merged main157.
The numerical room is a test fixture. Claude is assembling the finished level
and presentation; dynamic HUD bindings, music and reference-device performance
remain open. [Scope and evidence](docs/spikes/core-sample-gameplay.md).


## Compound animation transitions — current-main integration verified

Reviewed grouped-pose transitions now integrate with actual main155, retaining
independent clocks/interpolation, staged sampling/IK publication and exact
world-save continuation. The current union passes 766 CPU tests (82 GPU ignored)
and 420 real Wasmtime/wasm2c tests, all-target Clippy, wasm32, generated contracts
and 238 Python cases (4 existing skips); suite counts overlap and remain separate.
A fresh current-lock shipping package passes 3 complete runs and 10 save/resume
pairs after authoring/build/debug-map deletion, with 112 full pose rows and 224
vector checks per full run and exact save/PCM/log continuation. Two precise
negative controls refuse without checkpoint/PCM publication. Final independent integrated review and the exact fast check passed; merged main156. Complete compound Wasm ABI is merged as main159. Root-motion gameplay and device/appearance/phase gates remain open.
[Integration and preserved failures](docs/spikes/native-compound-main155.md).

## UI fixed-point blend reference — merged main155; checkpoint13 terminal

The independent reference now admits bounded 8-bit fixed-point UNORM blending
while retaining exact geometry, clip, opacity and untouched-pixel controls.
Shader/runtime behavior is unchanged. Current-main CPU tests, Clippy and wasm32
compile pass; independent producer/current-main reviews and seven caught mutations are retained.
Checkpoint13 passed the Linux and Windows numerical rendering stages. Its general
macOS probe failed and its Windows desktop job timed out later, so that batch remained
held until a repaired frozen candidate passes all 23 required checks.
The Windows conformance job also exceeded its 40-minute limit after successful
conformance stages; only that existing job now permits 90 minutes for the remaining
shipping checks. Workflow lint and all six release-policy tests pass.
Checkpoint12 is still failed/frozen. [Scope and evidence](docs/spikes/native-ui-unorm-main154.md).

## Native window error preservation — main153 integration verified

After an error or explicit exit, the window host stops delivering game callbacks.
Surface and IME cleanup still run, while the original typed failure takes precedence
rather than being replaced by a later generic failed-session or cleanup error.
The current-main union passes 57 player tests, native-window Clippy, dependency
and tooling checks with both reviewed source files unchanged. Independent current-main review and the exact fast check passed; this repair is merged in main #154. This does not diagnose the original instrumented-process
failure or change audio-overload behavior. [Scope and evidence](docs/spikes/native-window-terminal-errors.md).



## Native Rust/Wasm IK — main152 integration verified

Additive operations 44–46 expose complete pre-command v4 poses plus ordered IK
constraints and atomic target/pole/weight controls. Existing operations 0–43 and
layouts remain unchanged; all eight old pose readers refuse committed IK.
Current-main integration passes 418 actual-host tests, Clippy, three development
processes and scoped target/dependency/generated checks. Three fresh ordinary
source/cache/build/map-deleted players each match 252 public vector assertions,
4,140 full scalar observations and exact development PCM/logs. Fifteen precise
refusals pass. This IK-v1 API is merged in main #153 after independent
current-main review and its exact-head fast check.
Compound-pose SDK parity, static-module checkpoints, appearance and device gates
remain open. [Integrated evidence](docs/spikes/native-wasm-ik-main152.md).

## Authored two-bone IK — main151 integration verified

Bounded stable-ID constraints bind actual cooked model topology/fingerprints,
travel through v16 scene/v9 world saves, and expose owned TypeScript columns and
atomic controls. The canonical CPU consumer applies ordered IK after sampling;
unsupported chains explicitly refuse with retained-input rollback. The current
main151 union passes 823 CPU cases (82 GPU cases intentionally ignored), scoped
Clippy, target/dependency/schema/SDK checks and a fresh public shipping proof.
Four source/map/build-deleted processes preserve exact checkpoint, PCM and log
continuation; seven precise refusals pass. All eight legacy Wasm v1–v4 pose calls
refuse committed IK atomically. This connected route is merged in main #152
after final independent review and its exact-head fast check.

Renderer appearance/GPU/device budgets, edit-mode IK preview/authoring UI and
Rust/Wasm IK controls have the separately verified integration above. Allocation
bounds and the original producer
coverage remain in the [contract](docs/spikes/native-authored-ik.md); exact current
source/proof identities are in the [integration record](docs/spikes/native-authored-ik-main151.md).

## Native input corrections — main148 integration verified

Text-field pointer release now focuses without submitting its value. Buttons
still activate on matching release; explicit submit and IME ownership remain
unchanged. The original regression fails, and all 47 UI tests plus Clippy pass
after the correction. The combined current-main148 union passes 127 CPU tests,
all-target Clippy, wasm32 and dependency/tooling checks. The separately accepted Claude0058 native follow-up is limited to its frozen
older build; see [qualifications](docs/spikes/native-text-0058-acceptance.md).
[Integrated evidence](docs/spikes/native-input-main148.md).
[Evidence](docs/spikes/ui-pointer-focus.md).

## macOS native text retirement — local crash regression repaired

The managed input client now avoids an AppKit view-destructor callback into
already-dropped Rust state. The original invisible-window probe aborts; the
fixed source passes 768 actual contexts and 256 stale-owner checks, all 126
affected CPU tests, Clippy and independent source review. Current-main CPU
integration now passes; three ordinary visible processes also pass the scoped
Claude0058 follow-up. Live U.S. dead-key composition is covered, while CJK/Arabic
IME and the excluded heap-attempt diagnostic remain open. The same regression is added to
comprehensive macOS CI. [Repair and evidence](docs/spikes/macos-text-retirement.md).

## Authored trees and complete Wasm poses — current-main verified

The accepted authoring, cooked v15 tree evaluator, retained weighted-pose consumer
and v4 Wasm SDK are merged in main #148 after current-main147 verification. All 81 changed implementation
and build files match their independently reviewed producer bytes; current audio,
UI checkpoints, two-bone IK and pinned dependencies are preserved. The union
passes 793 affected CPU tests and 401 overlapping actual-host cases, all-target
Clippy, wasm32, exact schemas/SDK and tooling checks. All 82 GPU tests stayed ignored.

Two fresh shipping builds pass after deleting their private source/build/map
inputs: QuickJS runs three full games and six save/resume pairs with exact final
save and PCM equality, plus two precise refusals. The Rust v4 package runs three
fresh games matching 3420 full-pose scalars and 228 bounded checks against the
direct native oracle and captured-only development host, with eleven precise
refusals. All 1696 frozen inputs remain unchanged. These are local CPU results;
rendered/device/phase gates and full guest checkpoints remain open.
[Integrated checks and proofs](docs/spikes/native-animation-trees-main147.md).

## Cooked root-motion numeric sampler — main150 integration verified

A bounded CPU sampler now reads rigid local translation/rotation deltas from
immutable cooked tracks. Exact signed cycle cursors support forward/reverse, wrap
and multiple-cycle motion, with ordered rotation/translation composition. Selected
scale or matrix-only tracks refuse explicitly. All 128 assets/runtime cases,
all-target Clippy, wasm32 visual and dependency checks pass; 4096 query iterations
allocate nothing after admission. Independent source and retained evidence review
accepts the implementation without findings. The actual-main150 integration
passes all 128 affected CPU cases, Clippy, wasm32 and dependency/tooling checks;
[its source and logs are retained](docs/spikes/native-root-motion-main150.md).
Authored root selection, graph/clock integration, pose extraction, body movement,
save/SDK controls and rendered/device gates remain open; this is not complete
root motion or Phase 1. [Contract and evidence](docs/spikes/native-root-motion-core.md).

## Public Wasm complete animation poses — local host and package proofs passed

The Rust SDK now reads complete v4 pose frames with stable scene/entity identity,
exact native contributions and explicit interpolation kinds. Legacy SLERP pairs
retain their semantics; weighted tree poses use the native normalized quaternion
mean. Old observations refuse weighted poses. Published operations 0–41 remain
unchanged. Actual Wasmtime/wasm2c conformance passes 398 tests/doctests, Clippy and
three generic development smoke processes; the new guest matches a direct native
eight-pose oracle over twenty ticks. Source review accepts the frozen boundary.
The original producer source and package reviews pass; the current-main union
now also passes fresh package and combined checks, recorded below. The ordinary public Rust tree-game package proof now passes
three fresh source/cache/build/map-deleted wasm2c processes. Each matches 60
complete records (3420 scalars), 228 bounded checks, native clocks and PCM against
captured Wasmtime play and a direct native oracle. All eleven exact refusals pass,
including old snapshots and unsupported guest checkpoints. Independent package
source and final evidence review accepts the proof without findings; current-main
union validation also passes. [Package evidence](docs/spikes/native-wasm-animation-pose-package.md).
Synthetic sixteen-sample coverage establishes wire capacity only; tree-state
fades/composite operands, rendered/device gates and complete SDK remain open.
[Contract and evidence](docs/spikes/native-animation-pose-sdk.md).

## Authored nested animation trees — local and ordinary shipping checks passed

Stable-ID nested 1D and rectangular 2D motions now cook into a bounded v15 node
table and run on the native graph clock. Complete weighted samples travel through
native/render snapshots and owned TypeScript columns; legacy Wasm pose snapshots
explicitly refuse these unrepresentable states. Existing clip/flat-pair encodings
and SLERP sampling remain unchanged. All 665 affected CPU tests, Clippy, wasm32,
dependency guards and exact schemas/SDK checks pass; 82 renderer GPU tests stayed
ignored. Public command authoring, Undo/Redo and fresh recovery pass 228 selected
state checks plus complete 56-tick sample logs. An ordinary shipping player runs
after deleting authoring/build inputs and external source maps: three full runs,
six save/resume pairs and two exact refusals pass, with matching final saves,
logs and PCM. Current-main integration now passes the combined checks below;
rendered acceptance remains pending. Nonzero transitions involving blend states,
layering, authored IK, root motion and device gates remain in progress; this is
not animation or Phase 1 completion. [Contract and evidence](docs/spikes/native-animation-tree-playback.md).

## Ordinary offline audio session checkpoints — current-main verified

World v8/session v3 resume real static and streamed Kira voices alongside existing
script state/timers and hosted UI. Fractional/resampler history, birth order,
queued/retired voices, nested buses/listener state, tick-zero startup and master
suspension are preserved. Integration with main #145 passes 422 affected CPU
cases, 99 Kira cases, scoped Clippy and feature/platform/dependency checks.
A fresh public shipping export passes four source/build/map-deleted processes
with exact final saves and split/repeated PCM, plus nine precise refusals.
All 1672 frozen inputs remain unchanged after main ancestry integration.

Physics/module/device-native audio and native-window/GPU checkpoint routes remain
unsupported; arbitrary VM heap/effects and full save/device gates are not claimed.
[Contract and evidence](docs/spikes/native-audio-session-checkpoints.md) and
[current integration](docs/spikes/evidence/native-audio-main145.json).

## Two-bone IK retained CPU integration — current-main verified

The cooked-model animation player applies up to 32 ordered model-space constraints
to sampled or blended local poses and publishes the complete hierarchy/palettes
atomically. Both reach limits, weighted rotations, degenerate/refused chains,
source-deleted cooked skinning and allocation-free repeated solving are covered
by ten new cases; current-main #146 integration passes all 118 affected tests,
all-target Clippy, wasm32 and dependency checks. The six accepted source files
remain byte-identical. Existing cooked
formats, normal sampling and renderer paths stay unchanged. Authored/native/SDK
control integration, general affine chains and Claude rendered/animation-feel
acceptance remain open; this is not the complete Phase 1 animation gate.
[Contract and evidence](docs/spikes/native-two-bone-ik.md) and
[current integration](docs/spikes/evidence/native-two-bone-ik-main146.json).

## Public Wasm physics queries — current-main verified

The Rust SDK now batches native 3D/2D rays and character movement queries and
copies nondraining sensor enter/exit observations. Callback-scoped native solver
loans preserve committed-state visibility, exact world/epoch authority and atomic
rollback. Event frames retain scene identity and dimension. Published operations
0–38 and old layouts remain unchanged. Integration on main #144 passes 382 actual
Wasmtime/wasm2c cases, 202 native cases, all-target Clippy, wasm32/tooling and lean
dependency checks. All 1533 frozen inputs remain unchanged after main integration.
A fresh ordinary shipping package passes three source/cache/build/map-deleted
processes, 60 native-oracle assertions per run and ten exact runtime refusals.
The initial comparison host omitted its optional Wasmtime feature; its successful
shipping package was preserved byte-for-byte while the host was rebuilt and the
reviewed remaining proof resumed. The original failure stays recorded. Full SDK,
physics/guest checkpoints, device measurements and phase gates remain open.
[Package proof](docs/spikes/native-wasm-physics-package.md) and
[current integration](docs/spikes/evidence/native-wasm-physics-main144.json).
[Contract and evidence](docs/spikes/native-wasm-physics-queries.md).

## Nested animation tree numeric core — current-main verified

The native evaluator now supports bounded nested 1D and rectangular bilinear 2D
blend trees, normalized clip phases and sixteen-contribution fades. Full runtime
checks pass 200 cases; 4096 sampling/fade steps allocate nothing. Clippy, wasm32,
dependency checks and independent source review pass. This is numerical core
work; the separately reviewed authored/cooked/runtime/SDK connection and ordinary
checkpoint continuation are now verified above. Timed transitions involving
weighted tree states remain separate. [Contract and evidence](docs/spikes/native-animation-tree-core.md).

## Multi-pose animation sampling — current-main verified

The retained numeric animation player now mixes up to sixteen clip samples with
weighted local translation/scale and normalized quaternion averaging. Existing
unpublished pose storage supplies the scratch lanes, so retained memory and
per-sample allocations do not increase. All 108 affected tests, Clippy, wasm32,
dependency and formatting checks pass; independent source review accepts the
scope. Combined integration on main #143 passes 310 CPU tests and Clippy.
The authored nested/2D connection and complete-pose native/SDK wiring now have
the separate current-main proof above. Timed blended-state transitions remain open. [Contract and evidence](docs/spikes/runtime-weighted-pose.md).

## Checkpoint 11 — failed frozen batch, two probe contracts repaired

The main #141 snapshot finished 20/23 required checks successfully; release is
unchanged. Windows model receipt validation now requires all twelve exact
geometry counters, and the macOS shipping helper expects the current explicit
manual-input argument refusal. CPU regression checks and independent review
pass; production behavior and rendering thresholds are unchanged. Linux UI
numeric differences now have a reviewed, locally CPU-verified candidate correction;
actual Linux/adapter verification awaits the next full checkpoint after merge. See [retained failures and checks](docs/spikes/checkpoint11-probe-repairs.md).

## Hosted native UI session checkpoints — current-main CPU proof passed

The bounded widget codec now composes with native world v7 and session v2:
committed UiNode declarations, text buffers, grapheme selection, logical focus,
script persistent values and timers restore into one fresh world and UI owner.
Non-UI encodings remain unchanged. Exact world/generation/tick authority and
complete typed sections are checked before consumer/guest initialization;
ordinary loads neutralize physical input and discard transient pointer capture.
The current-main suite passes 450 CPU tests plus nine player tests, scoped Clippy,
feature-disabled/static/development-module builds, wasm32 and lean runtime checks.
A fresh ordinary public export passes four map/source/build-deleted player runs
and seven typed refusals; uninterrupted and resumed final saves match exactly.
Active preedit, physics state, static/development modules and window/GPU
checkpoint routes remain unsupported. Offline audio continuation is covered by
the reviewed local slice above. This is not a full-session, native IME,
Game UI or device gate. The fresh current-lock public proof passes again; source and evidence review
remain scoped to this CPU ordinary-load route. [Contract and evidence](docs/spikes/native-ui-session-checkpoints.md).

## Native game UI reserved-frame connection — current-main verified

Managed native input and Claude's accepted screen-space consumer compose in the
native player. Reserved-frame sizing now fences old pointer coordinates before
relayout and preserves text ownership; unchanged frames reuse UI resources.
All 138 affected CPU tests, Clippy, dependency checks and independent source
review pass again with both production files unchanged on actual main139.
Actual native-window/IME and pixel review are assigned to fresh Claude0057, so no
native appearance or complete Game UI claim is made. See
[the sizing contract](docs/spikes/native-window-ui-reserved.md).

## Public Wasm navigation paths — current-main integration verified

The Rust SDK now queries the committed native navmesh and prepared-grid solvers
through bounded copied request/results. Existing opcodes 0–35 remain unchanged;
new calls preserve stable IDs, exact generations/costs and off-mesh metadata.
Navigation and steering share the same callback quota, with a separate bounded
requested-search budget; authorized failures reject the complete staged native
transaction. Actual Wasmtime/wasm2c conformance passes 369 tests/doctests, Clippy
and three development processes. A separate 190 native runtime tests, supported
wasm32/tooling checks and the authoring-free dependency check pass. Independent
source and final evidence review accepted the implementation. Current-main
integration repeats all 369 actual-host and 190 native cases with unchanged
source; PR #138 passed the fast gate and merged. An ordinary source-deleted navigation guest shipping proof, full SDK,
guest checkpoints, devices and phase gates remain open.
[Contract and evidence](docs/spikes/native-wasm-navigation-paths.md).


## Public Wasm animation graphs — current-main integration verified

The Rust SDK and native Wasmtime/wasm2c hosts now expose copied graph state,
parameters and atomic controls, with an additive v3 snapshot for actual timed
and parameter-driven pose blends. Published opcodes 0–30 and old layouts remain
unchanged; old snapshots explicitly refuse blends they cannot represent. The
corrected implementation passes 355 actual-host tests/doctests, Clippy and three
development processes, plus supported wasm32/tooling checks. Independent source
review closed a forged-rest-destination parser case. The actual main132 union
repeats all 355 cases and three development processes with the 31 reviewed source
inputs unchanged; PR #134 passed its fast gate and merged. Complete animation/SDK, guest checkpoints, device and
phase requirements remain open. [Contract/evidence](docs/spikes/native-animation-graph-sdk.md).

## Public authored Wasm route — current-main integration verified

One typed scene module now connects public bounded `.wasm` import/reimport,
atomic selection/clear with provenance and Undo, durable editor-default journal
reopen, captured-only Wasmtime development Play and the ordinary native exporter.
The exporter translates the exact validated/metered capture with pinned wasm2c;
ordinary `--package` startup automatically selects the linked guest. A real Rust
SDK guest and three fresh source/cache/build-deleted packaged processes agree on
20 ticks of clock, controls, movement and committed logs. Malformed/stale/mixed
inputs and forged cache identities refuse. Current integration passes 278
feature-enabled tests, five separate feature-disabled startup cases, all-target
Clippy, supported wasm32 and tooling/contracts. A fresh current-lock shipping
export repeats the three source/cache/build-deleted runs and refusal controls.
The shipping graph excludes Wasmtime, cooker and authoring dependencies. Full
guest checkpoints remain explicitly Unsupported, including active/paused
crossfade sessions. Full SDK, routed UI ABI, other shipping platforms, device
performance and phase gates remain open; this scope merged through its fast main gate in PR #128.
[Contract and measured evidence](docs/spikes/authored-wasm-export.md).
Independent review also corrected interrupted cache publication: private staged
files now publish without clobber, with partial-write/concurrent/corrupt/symlink
controls; the original source-free shipping proof remains separately recorded.

## Native screen-space UI drawing — scoped integration

The canonical UI candidate now reaches the retained GPU consumer, drawing labels,
buttons, text fields, focus, selection and preedit over the world. Candidate
refusal preserves accepted owner/resources; unchanged frames reuse their UI
resources. Claude0056 supplied and reviewed the visuals on its frozen offscreen
Metal build. Current-main CPU/compile checks preserve all renderer/shader source.
The capability also admits the existing desktop UI startup route; reserved-frame
correction now has current-main CPU verification; actual window/IME acceptance
remains with Claude0057. PR #139 passed the fast gate and merged. No current-main GPU, phone or complete Game UI gate is claimed.
[Contract and evidence](docs/spikes/native-ui-draw.md).

## Native game UI window delivery — CPU adapter

The ordinary desktop host queues bounded opaque pointer/text receipts into its
composed UI tick, with logical-DPI relayout, exclusive pointer ownership and
transactional cancellation. The new opt-in Tao 0.37.1 managed bridge binds native
Cocoa/GTK/IMM contexts to exact text-owner leases, supplies preedit/commit/cancel,
and discards retired callbacks. The actual main136 integration passes 119 CPU
tests, 8 exact protocol tests, scoped Clippy, editor/Windows compile and runtime
graph checks. Prior Linux compile evidence retains identical Tao sources and its
entire dependency closure. Actual OS IME interaction, canonical caret geometry
and combined window/draw acceptance remain open. This integration retains its unsupported draw guard;
no native/GPU/device gate is claimed. See [managed IME evidence](docs/spikes/native-ui-ime.md).

## Durable budget lock release — validated in checkpoint8

Checkpoint7 exposed Busy when reopening a completed ledger. A private lock guard
now explicitly releases ownership before descriptor close; two deterministic
shared-description regressions fail old source and pass fixed. All 64 agent
tests and Clippy pass, retaining live exclusion and conservative reservations.
The exact earlier hosted scheduling cause remains unconfirmed. Checkpoint8 now
passes all 23 required contexts, including the actual macOS tests, and is promoted.
[Evidence](docs/spikes/ledger-explicit-release.md).

## Release checkpoint6 correction

Failed checkpoint6 selected main110 and was never promoted. Three native physics jobs
exposed a stale isolated probe lock edge; the reviewed one-edge correction and
required-dependency guard pass local locked Cargo/Clippy and tooling checks.
The fixes merged in PR #112; checkpoint7 checked frozen repaired
main113 through the same 23 required contexts in PR #111. Checkpoint7 finished
with 22 successful contexts and one macOS ledger failure; it is not promoted.
PR #115 repairs that failure; checkpoint8 passes and promotes frozen main117.
The independent fresh observation-routing correction restores the precise
pre-startup refusal while preserving absolute checkpoint-resume bounds; its
unchanged process regression, 36 scoped tests and Clippy pass.
[Lock evidence](docs/spikes/checkpoint6-probe-lock.md),
[routing evidence](docs/spikes/checkpoint6-player-routing.md).
The separate Windows budget-test correction closes the journal owner before
independent reads, then verifies recovered state. Seven local tests and Clippy
pass; PR #113 merged the fixture correction. Checkpoint7 Windows desktop and
Wasm conformance both pass.
[Journal test evidence](docs/spikes/checkpoint6-budget-journal-tests.md).

## Checkpoint10 replay fixture correction

All eight frozen workflows finished: 20 required checks passed and three desktop
checks failed the same outdated unknown-version fixture. Recording version 2 is
intentionally supported. The test now refuses version 3 and compares accepted
versions 1/2, preserving all prior refusal and no-output assertions with specific
diagnostics. Three actual headless cases and scoped Clippy pass; independent
source review accepted. Checkpoint10 remains failed and unpromoted; hosted repair
confirmation awaits a new frozen candidate. [Evidence](docs/spikes/checkpoint10-input-fixture.md).

## Checkpoint9 agent fixture correction

The frozen main132 release run reaches the same pre-provider test assertion in
Linux/macOS/Windows credential checks and Linux desktop tests: the old fixed
allowance is smaller than the current generated tool input. Only that fixture
now derives its allowance from the current catalog, retaining exact redaction,
conservative full charging and no-mutation assertions. The unchanged baseline
fails locally; all 108 agent tests and Clippy pass after the correction, with
independent source/evidence review. Checkpoint9 remains failed and unpromoted;
actual cross-platform repair validation needs a later frozen batch. The reviewed
fixture correction merged through its fast gate in PR #135.
[Evidence](docs/spikes/checkpoint9-agent-fixture.md).

## Latest comprehensive release baseline

Checkpoint8 promoted frozen main117 into release at
`b614679a1d48aba6e4ff5387974351602de5c026` after all 23 required checks and eight
exact-candidate push workflows succeeded. Its tree is exactly the selected main117
tree; parents and Built-by trailer are verified. Checkpoints6–7 remain historical
failed attempts. Later main118+ work continues through reviewed local checks and
fast PR gates. Checkpoint9 finished on frozen main132 with 17 required successes
and six jobs failing at the same pre-provider fixture; no failed batch was
promoted. The independently reviewed correction merged in PR #135. Checkpoint10
froze main137 at `af3f84fdd60dda011d77295cbc21bcf55414b3f8` in PR #133,
with the same 23 required contexts. Navigation and later feature work are excluded.
Release does not track moving main.
[Promotion evidence](docs/spikes/evidence/release-checkpoint-8.json).

## Mixed-direction text wrapping — current-main CPU correction

The native text review's dropped-space/overflow defect is corrected in pinned
cosmic-text, with separate regressions for direction changes and narrow glyph
wrapping. The 77 integrated CPU cases, Clippy, wasm32 and dependency checks pass;
independent source review accepted. Claude0055 also accepted the corrected
180/300-pixel native cases on its frozen macOS/Metal ci-profile build, retaining
all 250 non-whitespace glyphs with no new defect. Shipping/device/full Unicode
and complete Game UI/IME gates remain open.
[Evidence](docs/spikes/text-bidi-wrap.md).

## Whole-session restart — integration proof completed

The resumed private integration passes355CPU cases and two fresh ordinary shipping
exports with24 separate player processes. Core/Planar restart matches fresh state
and exact concatenated PCM; both matching headless fixtures explicitly refuse
unsupported restart. Independent final integration review and exact-head fast check passed;
restart is merged in PR180. [Current evidence](docs/spikes/native-restart-main179.md) retains the
original missing-bundle compile attempt and unchanged-source correction.

## Authored Core runtime bindings — current integration passed

The full accepted 0060 presentation, corrected live locked-exit route and native
HUD/audio/restart bindings pass six current main182 integration checks. Sixteen
focused tests, the original seven numerical scenarios and fourteen fresh native
positive processes pass, with one precise headless-restart refusal. All sample
and art bytes remain exact; the matching headless is reused by exact source
identity. The original ten source-free shipping processes and four exact reset
PCM comparisons remain producer-scoped. Final integration review is accepted; the required fast check precedes publication.
Historical offscreen helper provenance remains qualified; live appearance/device
acceptance is separate. [Evidence](docs/spikes/core-main182.md).

## Checkpoint17 terminal and shipping probe repair

Checkpoint17 finished with 20 required successes and three failures and was not promoted. Linux/Windows quota controls have the reviewed main188 repair. A separate macOS player library/executable audit correction now passes the exact retained compile-log replay and six regression controls; independent final review is accepted and the fast/full frozen checks remain required. See [probe evidence](docs/spikes/shipping-player-probe-kinds.md).

## Core live HUD integration reviewed

The sanitized Claude0065 hint/wrapping/font changes are composed with main190 and independently accepted for scoped integration; the fast publication check remains pending. Seven CPU checks, 14 native positive processes, four-locale font coverage and exact baseline gameplay/PCM pass. Result-title/narrow contrast and separate prompt/objective follow-ups remain open; no new rendered or device acceptance. See [integration evidence](docs/spikes/core-live-hud-main190.md).

## Core optional result-label binding — producer CPU proof accepted

The sample accepts an explicitly authored optional `hud.prompt` label and clears
the objective on winning. Result parenting and whole-session restart restore the
expected text/parents while the existing 12-widget HUD stays supported. Twenty
focused Python tests, strict TypeScript, 28 successful native processes and two
fresh source/map/build-deleted ordinary packages pass. Twenty shipping processes
include eight complete R/Start/held-input reset routes with exact fresh-state and
concatenated PCM comparisons. Nine exact native refusals pass. Independent source
and final execution review are accepted; current-main integration is pending.
The production prompt node/localization/style is still Claude-owned and absent;
this is not appearance or device/performance acceptance. Prior 0065 evidence stays
immutable. [Contract/evidence](docs/spikes/core-result-hud.md).

## Historical integration snapshots

The paragraphs in this section preserve the state at earlier integrations. Their
main numbers, queued work and active checkpoint labels are historical; use the
current overview above and linked immutable receipts for present status.

Main #184 includes compound native animation and Wasm snapshots, authored two-bone
IK, native/TypeScript/Wasm root motion, audio transport and HUD, offline audio/UI
checkpoints, public physics queries, and the Core/Planar gameplay foundations.
Pearl Cove's authored art/HUD and runtime hooks are merged with their scoped
producer visual and shipping evidence. Headless UI, the reusable player library,
macOS C/Swift callers and trusted-source iOS bytecode compatibility are merged.
Reviewed physics kernel forwarding, cooked/save identity, project selection and
mode-aware developer/desktop export tools are merged; automatic editor worker
selection and actual physics saves remain separate. Authored iOS archives are
merged with actual simulator proof and qualified device-link evidence. The retained
CPU worker and managed owner are merged through main179;
whole-session restart is merged in main180 and bounded PCM in main181. Both authored
Android variants pass preserved pre-PCM archive/emulator proofs; the final composed
deterministic archive and both-mode mobile/target checks pass, with independent
integration review and the exact-head fast check passed; Android is merged in main182.
The composed iOS fixed-mode simulator proof, final integration review and required
fast check passed; fixed-mode iOS export is merged in main183. Core authored HUD/audio/restart bindings are independently accepted locally
and queued; their new live presentation acceptance remains separate. Phase 1 and the finished game
remain incomplete.

Checkpoint14 promoted selected main160 `d1ef35ee` after all 23 exact-candidate
required checks and eight push workflows succeeded. Release `2444d81f` has tested
tree `c0137be6`; later main161+ features are excluded. The repaired AppKit probe
and hosted Linux/Windows numerical UI checks passed. Failed checkpoint13 remains
historical: its general macOS probe failed and Windows desktop exceeded its old
one-hour allowance. The main158 probe repair and main160 Windows job allowance
were verified by this successful batch, without removing checks or reusing private
shipping builds. [Promotion and evidence](docs/spikes/release-checkpoint-14.md).

Checkpoint15 finished unpromoted with20 required successes and three exporter-graph
failures; Windows conformance passed in87minutes. Checkpoint16 now freezes selected
main179 `02203670`, candidate `6a336ac5`, with the exact same tree `f9a6007d`.
The general macOS fresh-session ledger failed by confusing the same-named player
library/executable commands. The reviewed crate-kind correction passes24 focused
regressions and330 tooling cases (four existing platform skips), with exact hosted
command replay. Windows conformance remains active at this snapshot. The candidate
stays frozen; all23 contexts remain required. Release remains checkpoint14 until a
reviewed repaired candidate fully passes. [Failure and repair](docs/spikes/shipping-session-player-kinds.md).

Native animation graphs now connect ordinary typed authoring, captured models,
v14 cooking, deterministic native clip states and flat adjacent-pair 1D blends.
TypeScript controls and v6 checkpoints retain parameters, phase, fades and a
separate master pause, allowing transitions after natural completion. The actual
main130 integration passes 538 CPU cases and 342 overlapping actual Wasmtime/wasm2c
cases, Clippy, wasm32 and 68 exact schemas. A fresh current-lock shipping export
removes source/cache/build inputs and external debug maps before three 90-tick
replays, eight checkpoint split/continuation processes and two precise refusals.
Active, paused, completed-paused and blended v6 continuations preserve exact final
save bytes, PCM and logs. Legacy Wasm v1/v2
snapshots explicitly refuse unrepresentable graph blends. The additive guest
graph SDK now carries bounded snapshots and atomic controls through both native
Wasm hosts, with its current-main checks recorded above. The newer nested/2D
tree route is verified separately above. The later compound transition and authored IK routes are separately verified
above; complete layering, rendered acceptance and device gates remain open. Connected
root motion merged as main161.
[Scope and evidence](docs/spikes/native-animation-graphs.md).

Connected native UI now composes typed UiNode authoring, v13 cooked scenes,
Taffy layout, focus/edit/activation routing and canonical CPU gameplay. The
reviewed main129 integration passes 510 CPU cases plus 209 overlapping final
script/Play/game cases, Clippy, compiler-disabled/wasm checks and 67 exact fresh
schemas. A fresh current-lock shipping export deletes authoring/build storage
and developer source maps before three 12-tick runs and five precise refusals.
Hosted CPU UI checkpoints are locally verified above; static/development Wasm UI remains explicitly unsupported.
Claude0056's reviewed offscreen drawing implementation now has current-main
CPU/compile validation: 206 cases, Clippy, downstream and wasm32 checks pass;
89 GPU tests remain ignored in this integration. Its original frozen Metal
numerical and appearance acceptance stays separately recorded. Managed IME has
CPU/platform compilation evidence. Reserved-frame geometry has scoped CPU verification. Actual window interaction,
HiDPI glyphs and complete Game UI/device gates remain open. [CPU scope and evidence](docs/spikes/native-ui-layout.md).

Native two-input crossfade clocks and controls now retain independent outgoing
playback, pause both inputs, finish at explicit endpoints and persist active
transitions in world-save v5/session checkpoints. Older v4 animation saves keep
their encoding. The local core passed 319 runtime/script tests and scoped Clippy;
independent review accepted the exact sources. Canonical render bindings now carry
both samples through rollback and preallocated CPU blending; 350 distinct affected
CPU tests, Clippy and the wasm32 adapter check pass locally. TypeScript/Wasm
crossfade APIs retain copied full transition snapshots, explicit v2 Wasm framing,
atomic controls and actual guest continuation. The frozen SDK suite passed 313
CPU tests/doctests, Clippy and wasm32 compilation with independent source review.
The current main122 integration passes 367 CPU tests, 313 actual dual-host tests
and Clippy. A fresh current-lock QuickJS export removes authoring/build inputs
and external debug maps before nine copied-player processes: three full replays,
active/paused v5 continuations and two refusals all pass. The original export
retaining an external map remains qualified historical evidence. Crossfade GPU/Claude acceptance, layering,
authored IK and root motion remain open; nested/2D trees now have the separate proof above; bounded clip-state graphs are recorded above. See
[the consumer evidence](docs/spikes/native-animation-crossfade-consumer.md),
[the core evidence](docs/spikes/native-animation-crossfades.md) and
[the SDK evidence](docs/spikes/native-animation-crossfade-sdk.md) and
[the public export evidence](docs/spikes/native-crossfade-export.md).
[Current combined receipt](docs/spikes/evidence/native-crossfade-main122.json).

The project is still building the Phase 1 core engine. Native simulation,
authored scripting, imports, physics, audio, input and source-free exports have
working, tested paths, but the complete Core Sample and its device gates remain
ahead. Editor and agent work overlaps later phases without completing their gates.

Native Wasm logging/localization, desktop gamepad input, session and
exported-player checkpoints, packaged-font text shaping, and the browser renderer
compile repair are merged into main, with skeletal asset foundations and font
import/cooking added in PRs #107–108. The actual native editor Play/chat/budget
review completed one real account-backed turn, Undo and reopen with retained
login and usage. Both follow-up UI fixes passed focused native rechecking; PR #109
merged this editor/agent/budget integration into main.

Skeletal animation merged in PR #116: authored clip metadata, native
clocks, atomic TypeScript controls, retained deformation and save continuation
are implemented locally. The combined runtime/cook/render suite passes 397 CPU
tests; a separate frozen renderer passes all 70 numerical Metal tests. Independent
review found two save-version admission defects, now corrected with actual world
and script-session continuation regressions. The integrated editor/runtime/script
suite passes 357 CPU tests, Clippy and a fresh native build with the current
production UI. The final main110 union passes 519 ordinary tests, 57 cooker/cooked-geometry CPU
tests (overlapping scope), Clippy and a fresh source-free export proof with exact
replay/checkpoint continuation and refusal controls. Claude0052 passed native playback/transport/resize/authoring checks, then found
the nullable clip field was unsupported in the Inspector. A bounded bridge
correction and focused Claude0053 native recheck now pass. The main114 fresh
export repeats exact source-free continuation after the atlas lockfile change;
main115 adds only the reviewed authoring ledger repair with 64 agent tests/Clippy.
Two adjacent Runtime focus/transient-layout observations remain scoped followups. [Integration evidence](docs/spikes/native-animation-integration.md).

Opt-in cooked output/diagnostic/shadow shaders merged in PR #110 after scoped
Metal numerical and Claude appearance acceptance; both features stay off.
Runtime glyph atlases proceed independently. Advanced
rendering, full blend trees/state machines/IK/root motion, complete game UI/text
input, remaining public Wasm capabilities, and the Core Sample/2D sample with specified
device performance checks still prevent a Phase 1 completion claim. Main now includes
native animation (#116), Console logs (#117), Wasm animation (#118), typed text
packaging (#119), Wasm formatting (#120), pose blending (#121) and the composed CPU
text consumer (#122) and crossfades (#123). Canonical world-space text drawing
now has scoped Claude appearance/Metal numerical acceptance and current-main CPU
integration checks (#124). Bounded text-editing state merged in #125; steering now
merged in #126 after its corrected fast check passed. The reported mixed-bidi
wrapping defect merged in #127 with reviewed current-main CPU checks;
Claude0055 separately accepted the scoped frozen-build native rendered recheck.
Connected Game UI and remaining Wasm capability parity continue separately;
the public authored Wasm development/export route now passes its current-main
integration and fresh ordinary shipping proof.
No complete Game UI or phase gate is claimed.

Progress reports are scheduled approximately every 30 minutes while the goal is
active. Report merged work separately from local checks and unmet exit gates.
The project is still early relative to the complete Engine 1.0 and Driftwake
scope; no defensible percentage or completion date follows from the PR count.

## Native Wasm local avoidance — current-main integration checked

The public Rust SDK now has copied bounded steering requests/results backed by
native avoidance, with generated Wasmtime/wasm2c bindings and atomic rollback.
Both real hosts pass the original 331-test suite and the integrated 335-test
conformance suite, with all-target Clippy. Independent
review caught an f64-to-f32 admission mismatch; original-value domain checks and
causal rollback regressions now pass. The strict generated ABI, wasm32 guest
compile and 228 Python cases (four platform skips) pass. The reviewed main128 union also passes wasm32, tooling and shipping dependency
checks after correcting the audit feature selection. Public shipping steering,
remaining navigation bindings and reference-device gates are not claimed. [Scope](docs/spikes/native-wasm-steering.md).


## Native local avoidance — current-main integration checked

The native simulation and TypeScript SDK now expose bounded bulk steering with
fixed native IDs and owned, sorted velocity proposals. Existing authoring schemas
and numerical avoidance behavior are preserved. Shared work limits, hostile
getters, expired callbacks and caught failures retain the whole-update rollback
contract. The 364 integrated scoped tests include strict authored TypeScript that
continues moving after its source directory is deleted; Clippy, wasm32 compilation
and authoring-free dependency checks pass. Wasm steering bindings now have the reviewed dual-host path above; automatic
obstacle gathering, debug draw, the Core Sample and real-device performance
evidence remain open. [Scope and evidence](docs/spikes/native-steering.md).

## Native text editing — merged in PR #125

The runtime text buffer now supports bounded committed text, directional
grapheme selection/deletion and transient IME composition. Focus loss cancels
preedit; rejected input preserves text and selection. A real packaged-font test
shapes the composed Latin/CJK string and preserves its retained layout through
commit. All 38 integrated text tests, Clippy, wasm32 and lean dependency checks pass, with
independent review of the exact source. Widget layout/focus ownership, OS event
delivery and IME activation, visual caret/hit testing and mobile keyboard remain
open. This is not a completed text widget or live platform gate.
[Evidence](docs/spikes/native-text-editing.md).

## Native text scene, packaging and composed CPU input — integrated CPU verification

Typed Text declarations now cook into bounded v12 binary scene data with explicit
ordered font chains and immutable recipe bindings. Structural edits and
save/restore retain/check the same font catalog and invalidate retained consumers
by world identity/generation. Font loading and export data packaging preflight a
64 MiB aggregate before payload reads. The actual public command/import/cook,
export producer and data packager feed a fresh source-deleted numerical
Latin/Arabic/CJK shaping/atlas test. The existing composed host now retains that
canonical text input beside spatial/script/audio state; motion preserves immutable
layout/atlas allocations and failed candidates preserve accepted CPU input.
Three actual fresh exported `--package` runs after source/build deletion pass
mixed model, native script motion, multilingual shape/atlas and audible PCM
controls; missing/tampered packaged fonts refuse before output. The affected
120 CPU tests and Clippy pass; 74 GPU tests were intentionally not executed.
The original CPU-only milestone kept the canonical GPU text refusal; the reviewed
0054 consumer now replaces that refusal with checked glyph drawing in the shared
HDR graph and uses the ordinary packaged font-cache/window route. Current-main
integration passes 127 CPU tests, Clippy, wasm32 and cooked-shader compilation;
82 GPU tests are ignored in this integration run. Claude's original Metal tests
and rendered review remain bound to their frozen build. Widget layout/IME/focus,
reference-phone gates and the separate bidi-wrapping repair remain open. Evidence: [text bindings](docs/spikes/native-text-bindings.md),
[composed text input](docs/spikes/native-text-input.md) and
[draw handoff](docs/spikes/native-text-draw-handoff.md).
The current main121 union passes 120 CPU tests, all-target Clippy and three fresh
source/build-deleted exported runs with matching atlas and audible PCM. Missing or
tampered fonts refuse; 75 GPU tests remain ignored.
[Integration receipt](docs/spikes/evidence/native-text-input-main121.json).
The original package retained an external debug source map; an exact-player
replay after removing only that map passes all three runs and two refusals with
all remaining package bytes unchanged. Future probes remove these sidecars.
[Replay evidence](docs/spikes/evidence/native-text-input-map-free-replay.json).

## Runtime font atlas — integrated CPU foundation

A retained CPU alpha atlas now rasterizes actual packaged TrueType glyphs with
content-bound keys, stable placements, explicit reset/invalidation and bounded
outline/mask admission. Latin, Arabic, Hebrew and a real CJK subset pass numerical
coverage tests, including a cache-deleted cooked-font reader route. Scoped 31
tests, Clippy, wasm32 compilation and lean runtime graph checks pass on the
main113 integration. Main122 adds the retained composed CPU text consumer and
actual packaged-player observations above. Text upload/draw is in Claude's returned
implementation and independent review; it is not yet merged here. Widgets/input/IME,
color/CFF support and device evidence remain; this does not complete Game UI.
[Scope and evidence](docs/spikes/native-font-atlas.md).


## Opt-in cooked output, diagnostic and shadow shaders — scoped Metal acceptance

The build-only cooker emits actual native MSL/SPIR-V/HLSL for output, diagnostic
geometry and all eight shadow variants. `cooked-output` and `cooked-geometry`
remain disabled by default. Private fixed-family consumption validates the full
vertex/resource contract before native shader calls. CPU cooker/renderer tests,
Clippy, default/feature Web compilation and build-only dependency guards pass.
The frozen v2 default/cooked pair passes all 138 existing/new numerical renderer
tests on Apple M5 Pro/Metal: all 16 output and 12 actual-diagnostic readbacks are
byte-identical. The actual main109 integration repeats all 138 tests successfully,
with identical paired and prior raw results. Claude0051 inspected 24 bounded PNGs
from eight successful/reaped processes and reports scoped Metal appearance passes;
all 12 pairs are identical. Shared thin-wall contact leakage, low-sun aliasing,
far thin-caster breakup and subtle receiver modulation remain renderer follow-ups;
the shadow fixture does not establish main-pass caster visibility. Vulkan/DX12
drivers, phones/performance, remaining shader families, caches and whole-renderer
translation removal remain open. Both features stay disabled by default.
[Contract and evidence](docs/spikes/cooked-geometry-shaders.md).

## Skeletal pose blending — reviewed main integration

The retained animation player can blend two independently sampled clips in local
TRS space before composing hierarchy and skinning. Exact endpoints, missing-channel
rest values, antipodal quaternions, atomic failure recovery, source-deleted fresh
loading and 1,024 allocation-free sampling/deformation calls pass. The 165 affected
asset/renderer CPU tests and Clippy pass; 70 GPU tests stayed ignored. Independent
review accepted the source. Authored blend trees, state machines, native frame/SDK
blend controls, IK and root motion remain; no Phase 1 gate is claimed.
[Evidence](docs/spikes/runtime-pose-blending.md).

## Native editor agent integration — merged in PR #109

The native session/bridge now connect streamed turns, approval/cancel, bounded
account-scoped conversation and real edits to the editor's shared bus. Account
receipt fences cover reconnect/switch and delayed approvals; repeated browser
opening runs off the admission path. One dedicated worker now retains the native
Play host and conversation across same-account/project turns without moving its
QuickJS state between threads. Actual cross-turn query, failed candidate and
replacement controls pass alongside cancellation/shutdown/panic checks. The
provider invalidation listener is integrated with its intervening-read race fixed.
The current editor/script/native-agent union passes 180 scoped Rust tests and
all-target Clippy after integrating main104 exported-player checkpoints. Claude’s chat UI is integrated with retained
conversation resizing, accurate denial/failure messages and cause-neutral uncertain
usage. The current combined UI suite passes 904 tests; strict app/test types and the
editor build pass. The corrected native cap-change and gameplay-focus fences pass 39 editor tests and Clippy. Four real native-handler turns used
the saved ChatGPT login for approved edits, native Play and a later retained-state
query; Undo and exact journal/account recovery passed after reopening. Claude's
Play, chat and project-budget controls are integrated. Independent review found
an approval-button hold missing during Undo/Redo: the native admission remains
safe, and both the explicit readiness metadata and UI consumer are now corrected.
Twelve deferred-IPC regressions and the combined 902-test UI suite pass; combined
native visual/WebView acceptance passed on the separately identified main100
build. Actual unsaved/saved Play, input focus, splitters, allowance edits and a
real approved agent rename were checked, including Undo and reopening with the
same account and 22,044 charged tokens. Two new source fixes address picker
focus loss and a clipped model field; both pass the rebuilt main104 native check
([follow-up](handoffs/0050-native-editor-focus-fit/result.md)). Account and usage
were retained. That run recorded an HTTPS exchange before user input; the latest
backend now restores account presence locally and leaves renewal to agent
requests. Main108 integration passes scoped Rust tests and Clippy, with no UI
source change or new provider turn.
No completed Phase 3 gate is claimed.
[Binding evidence](docs/spikes/native-editor-agent.md),
[persistent owner evidence](docs/spikes/persistent-native-agent-host.md),
[chat integration review](docs/spikes/agent-chat-integration-review.json).

## Per-project agent budget — in progress

Typed user-authored caps, the durable usage ledger and Session accounting are
now connected to native admission and reads. Usage survives new service instances,
cap changes and account changes without being part of Undo. Independent reviews
closed cancellation and cap-installation races. The verified bridge cap command and
dynamic shell capabilities are integrated, including offline edits and unchanged
usage across cap/history recovery. The integrated controls and shared bridge pass
902 UI tests, including the independently reviewed approval-readiness correction
and its 12 deferred-IPC regressions.
A fresh live native check completed four saved-account turns and retained all
73,259 measured tokens through Undo and a separate app process, with the same
login and exact journal restored. The headless command now shares the durable
ledger and recovered authored cap; invocation limits cannot refund usage or raise
the project cap. Its 16 tests, scoped Clippy and independent integration review
pass ([headless evidence](docs/spikes/headless-agent-budgets.md)). Claude-owned
controls and the approval-readiness correction are integrated; combined native
WebView/appearance acceptance passed on its main100 build, with 22,044 tokens
retained after Undo, reopen and a lowered cap. Rechecking the two UI fixes on
the rebuilt main104 union remains.
[Control integration review](docs/spikes/evidence/native-agent-allowance-integration.json).
[Budget accounting evidence](docs/spikes/agent-project-budget.md).

## Agent conversation context — locally verified

The shared agent loop now retains bounded, project-bound completed exchanges
across turns. Interrupted batches cannot leave unmatched calls in the next
request; partial commits remain explicitly acknowledged. All 53 agent tests,
Clippy and independent review pass. Native session, account and chat integration
remain outstanding; no live multi-turn provider gate is claimed.
[Evidence](docs/spikes/agent-conversation-context.md).

## Reusable native agent host — locally verified

The existing headless native Play controller/session and lazy authored viewport
provider now live in authoring-only `incant_agent_native`. Trusted host fallback
selection preserves the embedded headless behavior; authored source failures
never fall back, and checked candidate publication keeps its original atomic
boundary. Independent review, 102 scoped tests and matching all-target Clippy
pass; 62 schemas are unchanged and shipped dependency guards stay clean.
[Evidence](docs/spikes/native-agent-host.md). Editor wiring and native Play-frame
capture are separate work; no new GPU or live-provider gate is claimed.

## Native agent worker controller — locally verified

Account-backed turns now carry a durable non-secret selection receipt and check
it after responses/approvals and before tools, retaining actual response usage
when a stale account is refused. The 73-test agent suite and agent/headless
Clippy pass. [Account fence evidence](docs/spikes/agent-account-fence.md).
Native start/account/shutdown admission remains part of editor integration;
these checks do not atomically combine account changes with project commits.

The provider-independent session controller now supports one physical native
worker, immediate cancellation of stream/approval publication, stale-decision
rejection and bounded versioned snapshots. Session token accounting survives
errors and cancellation; uncertain accounting exhausts the allowance explicitly.
All 62 agent tests and scoped Clippy pass, including 15 new controller cases and
real agent/command-bus integration with a deterministic provider.
[Evidence](docs/spikes/native-agent-session.md). Actual editor/account/native-host
binding, conversation-context integration, persistent budget settings and visual
chat/approval/history remain separate work. This is not a live-provider gate.

## Packaged-font game text shaping — locally verified

`incant_text` now shapes real packaged fonts with explicit fallback order,
Unicode bidi and wrapping, absolute UTF-8 clusters and reusable immutable glyph
geometry. Fifteen numerical tests, all-target Clippy, independent font-ingress
review and standalone wasm compilation pass. The review's malformed-face panic,
metadata priority and name-allocation amplification findings are fixed. Runtime
fonts never come from host discovery. Project font import/cooking now retains
stable asset IDs through reimport/Undo and supplies bounded checked cooked bytes
to the shaper after original source deletion. The optional runtime reader keeps
the default audio-only dependency graph unchanged; 124 affected CPU tests and
scoped Clippy/compile checks pass. [Font asset evidence](docs/spikes/packaged-font-assets.md).
Font atlases, actual drawing,
widgets/IME, broader script coverage and device gates remain; this does not
complete game UI. [Evidence](docs/spikes/native-game-text.md).

## Opt-in native session checkpoints — locally verified

Bytecode behaviors can explicitly opt into bounded typed persistent data and
ordinary-load checkpoints containing committed world state, live authored
attachments and source-scoped timers. Trusted initial scene/program bindings are
checked before VM execution; invalid replacement preserves the live session.
Integrated main101 passes 324 affected tests/doctests, scoped Clippy/contracts and four successful fresh
source/build-deleted processes plus a corruption refusal pass. Whole checkpoint
bytes match uninterrupted execution. Module globals/closures remain explicitly
ephemeral and physical input restarts neutral. Physics/audio/static continuation
and platform durability remain unsupported; this does not complete Phase 1 saves. [Contract and evidence](docs/spikes/native-session-checkpoints.md).
Animation-bearing world-save v4 now passes the same ordinary-load boundary,
including nondefault locale restoration and exact script-state/timer continuation;
legacy and wrong-binding refusals stay atomic.
[Correction evidence](docs/spikes/evidence/native-animation-checkpoint-compatibility.json).

## Native game and exported-player checkpoints — locally verified

The existing GameSession families can create ordinary checkpoints and construct a
validated candidate on its exact restored WorldIdentity before VM/audio startup.
The public exported offline player accepts create-only `--save-checkpoint` and
`--load-checkpoint`, retains rational sample phase, uses new neutral relative input
and absolute observations, and refuses unsupported window/GPU/static routes.
311 affected tests, scoped Clippy and a fresh shipping export pass. After source
and private build deletion, five successful processes and nine refusal controls
pass; uninterrupted and twice-resumed final checkpoint bytes match. This remains
opt-in typed gameplay persistence, not arbitrary VM, physics/audio or complete
Phase 1 saves. PCM/checkpoint are not a multifile atomic transaction, and full
power-loss durability is not claimed. [Contract/evidence](docs/spikes/native-game-checkpoints.md).

## Shared agent project access — locally verified

The agent loop can share the editor command bus without holding its mutex during
provider requests, streaming, approvals, asset cooking or perception. The common
mutation path retains revision checks, provenance and undo. Native Play validates
the captured revision after immutable startup and before candidate ticks, so a
concurrent script write cannot publish a mixed scene/source revision. All 47 agent
tests, five native agent Play cases and scoped Clippy pass; independent review
accepted the corrected source-race boundary. [Evidence](docs/spikes/shared-agent-project-access.md).
The editor loading container now implements the same shared-access contract:
loading/failed states refuse, and ready state uses its existing bus and journal.
The adapter passes all 48 agent tests, three editor loading/recovery cases and
scoped Clippy; independent review accepted it. No duplicate bus is created.
This is backend work; chat-panel wiring, visual approval/history and budget UI
remain outstanding. It does not complete Phase 3 or a live-provider gate.

## CI cadence change — 2026-10-11

The director authorized local tests and a fast hosted development gate for main,
with comprehensive cross-platform CI on one frozen release checkpoint approximately
every three hours. [ADR 0014](docs/adr/0014-ci.md) defines integration, cache and
failure handling. The integrated policy passes 138 Python, 12 Node and 536 UI tests, strict types/build,
formatting, generated checks and actionlint; [evidence](docs/spikes/release-checkpoints.md).
PR83 merged after its exact-head fast check passed: 2s queue and119s execution. Main requires that fast check; release was bootstrapped at fully checked main82 and now requires all 23 full contexts, including local-service protocols. Both enforce administrators and prohibit force pushes/deletion. The three-hour heartbeat is active; checkpoints 1–4 remained unpromoted after diagnosed failures. Checkpoint 5 promoted exact main97 into release at `c603ca07bbe1e6e1ad46b2fd3aa2f70aaec07f50` after all 23 required contexts passed. [Promotion and timing evidence](docs/spikes/evidence/release-checkpoint-5.json). The historical missing-Wasm-target failure remains recorded separately.
No phase, live-provider or physical-device performance gate is waived.

The initial frozen checkpoint PR85 found a Windows-only journal test lock violation. A reviewed test-only lifetime correction passes all six scoped cases and Clippy locally; checkpoint 2 confirmed these Windows cases; checkpoint 5 subsequently completed release promotion. [Failure and repair evidence](docs/spikes/script-journal-test-portability.md).

The second frozen checkpoint includes the Windows journal repair and all 23 release
contexts. Its local-service job exposed a Linux private S3 config UID mismatch;
the isolated repair passes the real local protocol/restart proof while preserving
private permissions and the non-root server. The checkpoint also found macOS
Python process-observation and Windows native TypeScript startup failures. All eight workflows are now terminal: 20 required contexts passed and these three failed.
The combined reviewed repairs pass 199 Python tests (four Windows-only skips),
17 Node tests and scoped native compiler checks. All failures remain recorded;
no failed checkpoint is promoted. PR89 merged after its 126-second fast check;
checkpoint 3 was frozen at repaired main89 with all 23 contexts retained.
[Combined evidence](docs/spikes/checkpoint-2-portability.md).
[Service correction evidence](docs/spikes/local-service-foundation.md#linux-private-bootstrap-correction).

Checkpoint 3 passed the Linux service repair and macOS Python tooling step, then
Windows exposed a separate restricted TypeScript-host path lookup mismatch.
The reviewed correction normalizes only compiler callback keys, preserves strict
source/library confinement and passes actual TypeScript drive/UNC regressions,
19 Node tests, six actual native compiler/agent tests and the source RPC proof.
Checkpoint 3 is terminal with 22 of 23 required checks passing; macOS general
validation took 49m39s plus 4m37s queue. Its failed result remains historical,
and checkpoint 4 confirmed that exact authored TypeScript native-agent regression on Windows.
[Evidence](docs/spikes/windows-native-compiler-entry.md#checkpoint-3-restricted-typescript-host-lookup).

Historical checkpoint 4 was frozen at reviewed main94. All three native physics jobs exposed
a stale isolated probe lock missing navigation/localization dependencies. The
reviewed repair preserves existing pins and the engine's vendored source patch;
the fast structural regression adds no Rust compilation or new check context.
Main95/96 carried the reviewed repairs; checkpoint 5 passed all three native
physics jobs and strict comparison.
[Failure and repair scope](docs/spikes/checkpoint-4-probe-lock.md).

A later checkpoint 4 Windows export audit failed after the fresh shipping build:
PE bytecode symbols were incorrectly read with llvm-nm. The reviewed correction
uses a fresh final-link map and preserves parser/codegen checks and matched PDB
packaging. Local PE/PDB fixture validation, 206 Python tests (four skipped), and
a fresh macOS export with three source-deleted runs pass; checkpoint 5 completed the actual Windows export
with three source/build-deleted runs and independently verified PE/PDB/map identities.
[Exact failure and repair evidence](docs/spikes/checkpoint-4-export-symbols.md).

## Authored source and native package — locally integrated

Native localization now carries authored string tables through checked cooked
scenes into bounded TypeScript queries and the source-free player. Runtime locale
commands share atomic rollback, and world save v3 retains settings/catalog with
origin binding. Local validation passed 372 tests (two existing GPU tests ignored),
Clippy/contracts and three fresh shipping replays after authoring/build removal;
both independent source reviews passed. Actual main93 integration additionally
passes 420 affected tests, Clippy, 64 exact contracts and a repeated fresh three-run
source/build-deleted shipping proof with every accepted input unchanged.
Font rendering, RTL/CJK visual checks,
static-Wasm localization, composite session persistence and physical-device gates
remain separate. [Implementation and evidence](docs/spikes/native-localization.md).

A public host-equals-target exporter now cooks authored Script attachments into a
fresh shipping player and copies only referenced native assets. Three fresh
source-deleted runs reproduce all expected motion; startup tampering is refused.
Source transactions and the shared compiler/exporter now integrate actual main87.
The public acceptance probe creates/writes/attaches, plays, saves, reopens and
exports through the real interfaces, then reproduces motion after deleting its
private authoring fixture. 786 workspace, 75 GPU, 15 optional window/project,
178 Python (two Windows skips), 17 Node and 536 UI tests pass, with Clippy,
build/generation and shipping audits. Independent review removed a resurrected
old workflow and duplicate tests. [Evidence](docs/spikes/native-project-package.md).
Existing full checkpoint jobs include the export probe. Checkpoint 4 passed hosted
Linux and macOS export execution; checkpoint 5 also passed the repaired Windows
export. Window package execution and complete UI/export/device gates remain open.

The shared-agent backend's actual main92 integration preserves all nine reviewed
source identities and passes95 Agent/headless tests, scoped Clippy,62 exact
contracts and the public32-request/two-process source/Play proof. The stale
source-cook regression rejects a mixed candidate and retains the previous run.
Editor chat wiring remains separate. [Evidence](docs/spikes/shared-agent-project-access.md).

## Native editor Play transport — local, UI integration pending

The real editor worker now owns disposable native Play, Pause, Step, Stop and
selected-entity inspection. Cooking stays off the window thread and no authoring
state enters ticks. 21 scoped Rust tests, 545 UI tests, strict types and Clippy pass.
Actual macOS native-window numerical acceptance confirms paused GPU redraws,
one-tick stepping, minimize/resume, discarded Stop state, fresh restart and
unchanged project/script/journal bytes plus revision/history. Visible controls,
desktop input routing and inspector presentation remain Claude handoff 0044 work; this is
not a completed Play UI, pixel review, phone/performance or phase gate.
[Evidence](docs/spikes/native-editor-play.md).

## Current integration and parallel work

The closed retained-state / native SDK / shared Play-startup union passes 837
ordinary and 75 serialized GPU tests, full Clippy, 536 UI tests and the fresh
four-player shipping proof. Audio and navigation preparation now precede all
publication, with a combined atomic-refusal regression. World-save v2 remains
world-only; solver/audio/VM continuation and editor controls remain open.
The repaired main89 integration also passes two actual compiler cases, 51
Play/headless tests, scoped Clippy, the 32-request public source proof and 199
Python tests (four Windows-only skips).
[Integrated evidence](docs/spikes/native-state-play-union.md).

- Checkpoint 2 Windows native TypeScript startup has a reviewed local path adapter
  correction: fixed engine-relative entry and canonical-equivalent ordinary launch
  cwd; the scrubbed environment remains unchanged. Nine local tests and scoped
  Clippy pass; checkpoint 5 passed the actual Windows helper/workspace and export
  execution after the additional restricted-host correction.
  [Diagnosis and evidence](docs/spikes/windows-native-compiler-entry.md).

- Local native TypeScript navigation queries are implemented and independently reviewed:
  committed mesh/grid paths with owned off-mesh metadata, revocable cache loans,
  shared bounded search work and atomic script rollback. 299 distinct scoped tests,
  Clippy and generated-contract checks pass. Repaired main91 integration preserves
  all reviewed source hashes and passes 351 scoped tests, Clippy, 62 exact contracts
  and the real 32-request/two-process source/Play proof.
  [Evidence](docs/spikes/native-script-navigation.md).
  Query-only: movement, grid/world mapping, steering, automatic traversal, runtime
  rebuild controls, static-Wasm parity and physical-device navigation gates remain open.

| Workstream | Current result | Next work / boundary |
| --- | --- | --- |
| Editor and sprite authoring | [PR #61](https://github.com/snowdamiz/incant/pull/61) combines the final plain UI, typed atlas/audio imports, focus fixes and the corrected sprite/atlas baseline. 524 UI tests, strict TypeScript, production and native builds pass. | Merged after exact-head hosted checks and isolated native Claude acceptance. [Evidence](docs/spikes/editor-release-union.md). |
| Local service foundation | Digest-pinned Postgres/S3-compatible storage/Zitadel/SMTP capture with private generated state and loopback-only access passes 28 actual local protocol checks plus four restart-persistence checks after integration with actual main84. Private state-home refusal and complete SMTP body verification are covered by 16 scoped tests; all 154 Python and 17 Node tooling tests pass. | Independent scoped review accepted the foundation; checkpoint 2 exposed the Linux private-config ownership issue. Its reviewed correction passes the actual local protocol and restart controls. The candidate-only `local-service-protocols` context is now required among 23 checks; checkpoint 3 has passed all 28 protocol and four restart/persistence controls on hosted Linux AMD64. Accounts/player services, PITR, other local substitutes and real provider/device gates remain. [Evidence](docs/spikes/local-service-foundation.md). |
| Script-source transactions | Shared `write_script` combines one bounded `.ts` content edit with registration and attachment, with exact-preimage Undo/Redo, actor provenance and recoverable Prepare/Commit/Abort history. Registered-ID read/write tools and trusted editor/headless adapters use the same bus. | Locally: 145 scoped tests, Clippy, 59 generated contracts and 32 real RPC requests across two processes pass, including native motion and crash recovery. Source/compiler/package integration now passes the actual recovery guard and source-deleted shipping proof; existing three-OS checkpoint CI will collect platform execution evidence. Multi-file authoring, reconciliation UI, a script editor and complete agent-created gameplay remain open. [Evidence](docs/spikes/script-source-transactions.md). |
| Import completion and Undo | Native imports return an exact revision/transaction receipt; unchanged imports preserve history and redo without offering stale Undo. Claude 0038 accepted the browser completion copy on frozen source. | The original PLAN revision 8/import union is included in merged PR76 and PR80 after their full applicable checks passed; 534 UI tests and the scoped native receipt/Undo checks pass. [Evidence](docs/spikes/editor-import-receipts.md). |
| Native TypeScript cooking | Project-attached sources now cook once into grouped native behaviors with frozen per-entity props, scoped timers and atomic whole-tick rollback. 730 workspace tests, 98 Python/17 Node/536 UI tests and a 16-process source-deleted embedding proof pass locally. | Reviewed local implementation; editor controls, source editing, complete SDK/save/export integration and hosted/device gates remain. [Attached scripts](docs/spikes/native-project-scripts.md), [embedding proof](docs/spikes/native-project-script-embedding.md). |
| Native Play RPC and agent | Cooked-native retained RPC and bounded agent play/query passed twelve source-deleted developer processes and a real saved-OAuth Astra turn. The new local attached-script route passes actual fixed-Node dispatcher tests, public CLI motion checks and a saved-OAuth provider run with native play/query. | Base capability is merged through PR82 after all21 required checks passed. Attached-script changes stay local for combined integration; editor controls, gameplay capture and complete session saves remain. [RPC](docs/spikes/native-play-rpc.md), [agent evidence](docs/spikes/native-agent-play.md). |
| Native game and rendering | Input, embedded QuickJS, native physics, scene audio and retained GPU submission compose through one accepted-tick lifecycle. Binary v5 sprites share immutable geometry. | [PR #62](https://github.com/snowdamiz/incant/pull/62) is merged after all 20 checks. Native editor/legacy Play migration, complete SDK, shader pipeline work and device budgets remain. |
| Native window player | Optional cooked native game window has bounded recovery, keyboard/mouse input, suspension and device audio. Source-free lifecycle checks and Claude0037 corrected native acceptance pass. [Evidence](docs/spikes/native-window-player.md). | PR63 is included in merged [PR80](https://github.com/snowdamiz/incant/pull/80), whose 21 applicable checks passed. Paused refresh preserves gameplay/audio; fullscreen-route limitations remain explicit. Current-window scale now converts pointer/pixel-wheel coordinates to logical pixels while preserving line units/signs/order; 19 integrated numerical/routing tests and Clippy pass. [Input evidence](docs/spikes/native-player-input-units.md). No phone or frame-budget claim. |
| Native tilemaps | Typed v6 data and exact solid-cell collision preparation preserve the reviewed [PR68](https://github.com/snowdamiz/incant/pull/68) ancestry in the consolidated union. Retained GPU tile rendering now shares checked atlas versions, painter ordering and combined sprite/tile resource caps; 45 ordinary and 62 GPU tests pass. | Claude0039 accepted scoped tile appearance; PR68/73 are included in merged PR80 with passing hosted checks. Native solid-map collision is included through the planar physics boundary below; legacy Play remains unsupported. [Data](docs/spikes/native-tilemaps.md), [rendering](docs/spikes/native-tilemap-rendering.md). |
| Native planar physics | Retained Rapier2D, cooked v7, typed native commands and solid-map collision pass 585 workspace tests, 66 GPU tests, 18 native/browser parity streams and seven fresh shipping processes. | PR78 is included in merged PR80; hosted cross-desktop 2D parity and all applicable checks passed. Root/unit-scale planar bodies only; physics saves, 2D character/joints, full script SDK and phone/performance gates remain open. [Evidence](docs/spikes/native-physics-2d.md). |
| Native flipbooks | Local v8 playback and its additive QuickJS/Wasmtime/wasm2c bulk SDK are independently reviewed. The frozen SDK passes 687 workspace and 158 combined conformance tests plus a fresh 64-success/43-refusal shipping proof. | The corrected FBX/prewarming/flipbook baseline is merged through PR81 after all21 hosted checks. The SDK and shared shipping union are published in PR83 with integrated local verification; the policy transition and corrected setup merged through PR83 after the fast main gate passed. Full release-checkpoint validation remains pending. The generated public Rust SDK is locally integrated below; complete gameplay SDK, save and device/performance gates remain. [Playback](docs/spikes/native-flipbooks.md), [SDK evidence](docs/spikes/native-flipbook-sdk.md). |
| Static FBX import | Shared asset transactions accept a bounded static subset through pinned MIT ufbx; existing cooked model/texture formats and stable reimport IDs are preserved. 71 scoped Rust and 531 UI tests, Clippy and 11 CLI process controls pass. | Claude0040 accepts static conversion/colour/reimport and the separate generated-tangent correction after 101 ordinary/61 numerical GPU tests and a bounded 12-capture delta. The corrected FBX/prewarming/flipbook union passes 685 ordinary/73 GPU tests and a fresh source-free player proof; all21 combined hosted checks passed in merged PR81. The separate OPAQUE/emissive/environment mip-filter correction is locally integrated below; combined checks remain pending. Browser source-model cooking explicitly refuses; actual browser dispatch and native fixture checks pass. Animation, skinning and full FBX compatibility remain unsupported. [Evidence](docs/spikes/static-fbx-import.md). |
| Color mip roles | Local INCMOD03 and Color manifest3 preserve alpha-independent OPAQUE/emissive/environment RGB while retaining sprite/MASK/BLEND coverage filtering. Integrated PR81 source passes 703 workspace ordinary tests, Clippy and 68 numerical renderer GPU tests; checked base sharing retains logical resource caps. Claude0043 accepted 13 bounded captures without a blocker. | The PR83 SDK/color union passes 739 ordinary and 75 serialized GPU tests plus the complete local four-player fresh proof; twenty old hosted checks passed, and the general setup failure is corrected locally. The exact new fast main gate passed; complete release checks remain a separate pending gate. Old immutable versions require source-backed reimport to adopt the fix. [Evidence](docs/spikes/color-mip-roles.md). |
| Native WebAssembly | Shared bounded capability ABI serves Wasmtime and wasm2c. PR #64 passed actual Windows /Od + /O2, Linux and macOS static conformance and fresh packaging. PR #67 composes static modules with the native game session. The versioned input ABI now drives real guest motion/spawns in both hosts and a fresh local packaged game proof. | The input/composed-player hosted checks passed in the merged runtime union. Complete SDK, static-module renderer/session saves and device gates remain. [Input ABI evidence](docs/spikes/native-wasm-input-abi.md). |
| CI speed | Registry-only caches have actual three-host consumers. The strict documentation selector and measured opt1 functional profile are merged through PR76/80; all applicable hosted checks passed with full shipping coverage. | Main79 saved all three native CI-profile dependency caches. These cold migration jobs do not establish a warm Windows speedup; next natural consumers provide that evidence. [Profile scope](docs/spikes/ci-test-profile.md). A single older matched local macOS sample saved 16.8%; the final SDK/color union now passes all four fresh proofs and selects the reviewed shared session in the macOS workflow. Its hosted correctness/latency remains pending; no Windows/device speed claim. Shipping audits and device gates are unchanged. [Proof](docs/spikes/shipping-fresh-session.md). |
| Generated Rust module SDK | Local public no_std Rust SDK and generated v1 host/guest/C registry bindings preserve existing capabilities. 167 actual-host conformance tests, 692 workspace tests, Clippy and a fresh 8-success/26-refusal shipping proof pass. | Independent review and combined 167-test actual-host conformance, real guest execution and fresh GPU/SDK player proof pass; hosted platform checks remain pending. No complete gameplay SDK, module build sandbox, save, device or performance gate. [API guide](sdk/rust/README.md), [evidence](docs/spikes/native-rust-sdk.md). |

The local npm cache follow-up preserves existing keys and download paths while
restricting saves to successful exact default-branch or frozen release-candidate
jobs. All 131 tooling tests and
workflow lint pass; hosted restore/version compatibility and retained-size
measurements remain pending. [Evidence](docs/spikes/npm-cache-producers.md).

Credential CI has a report-only locked dependency classifier and conservative
local-crate trigger coverage. Actual tests remain enabled; any selection policy
requires separate review. [Scope and limits](docs/spikes/credential-impact-reporting.md).

Pinned WABT provisioning now retries only bounded transient transport failures;
all URL, checksum and archive checks remain enforced. Five local fault controls
and all 95 tools tests pass; a real pinned macOS download succeeds. Hosted
verification remains separate. [Evidence](docs/spikes/wabt-download-retry.md).

Three Astra extra-high subagents own implementation/integration work in isolated
checkouts. Root reviews and integrates completed slices rather than waiting for
all workstreams to finish together. Claude sessions own editor/native appearance
review; concurrent native reviews coordinate foreground ownership.

## Merge coordination

One integration coordinator writes main. At most two independent, reviewed PRs
are published for checks at a time; parallel implementation stays local until
there is capacity. Each candidate integrates actual main before publication.
Completed candidates merge after their applicable checks pass and the prospective
main tree is verified against the tested source. Included source PRs close only
after their exact authored commits are verified in main. This avoids dependent
PR chains and repeated CI restarts for metadata-only updates.

PR76, PR79, PR80 and PR81 are merged. PR80 includes the exact PR63/68/73/78 source
history; PR81 preserves the corrected PR77 history, pipeline preparation and
flipbooks after all 21 checks passed. PR82 has since merged native TypeScript
cooking, RPC/agent Play and planar-save refusal after all21 checks. PR83 now
combines the reviewed SDK/color/shared shipping work with the director-authorized
CI cadence transition. Authored native Play is merged through PR84. PR86 adds local services and PR87 fixes the checkpoint-discovered Windows journal test. The source-transaction/package successor integrates actual main87 with passing local checks; navigation and retained world saves remain the next local consolidation slice.
Further work remains local; this queue does not waive any phase or device gate. [Verified ancestry and queue evidence](docs/spikes/foundation-integration-queue.md).

## Implemented boundaries and evidence

The standalone native-window player now connects desktop gamepad events through
a shared bounded input adapter. Stable session IDs, analog/stick/D-pad mapping,
focus/suspend hotplug recovery and queue refusal have local synthetic tests;
default/CPU/offscreen dependency boundaries remain intact. Physical-controller
and Windows/Linux execution, haptics and touch remain unverified or unsupported.
[Evidence](docs/spikes/native-window-player.md#native-desktop-gamepads).

The native v1 save reader now rejects unsupported 2D solver state consistently
with its writer and in-place restore. A rehashed-payload regression fails before
the fix and passes afterward; full native session saves remain open.
[Evidence](docs/spikes/native-save-planar-refusal.md).

The reviewed native-assets/rendering union now preserves corrected FBX, pipeline
prewarming and flipbook source on actual main80. All 685 ordinary/73 GPU tests,
536 UI tests, 96 final tools tests, both workspace Clippy profiles, optional-window
checks, actual browser smoke and fresh source-free shipping controls pass. Final
combined hosted checks passed and PR81 is merged. [Evidence](docs/spikes/native-assets-render-union.md).

- **Pipeline preparation (merged through PR81).** The current finite renderer
  catalog is prepared before the first frame. All 601 ordinary and 67 actual GPU
  tests pass; material reimports across all current variants and four output
  formats allocate no additional pipelines or shader modules during drawing.
  A fresh source-free shipping proof passes eight GPU cases and 33 refusals.
  Claude 0041 accepted the scoped appearance. Combined flipbook source passes
  666 ordinary tests, two helper tests, 90 tools tests, release Clippy, all
  72 GPU cases and the fresh integrated shipping proof. All 21 combined PR81
  hosted checks passed. Offline backend
  shaders, persistent caches, recorded per-level permutations and phone hitch
  gates are still open. [Evidence](docs/spikes/prewarmed-render-pipelines.md).
  Claude also identified existing opaque-texture mip darkening; asset-cook
  diagnosis is in progress alongside the generated-tangent correction.

- **Authoring and accounts.** Typed canonical JSON and stable ULIDs, atomic shared
  commands, provenance, undo/redo and durable recovery; CRDT round-trip and scale
  investigations; editor/CLI OpenAI tools and a 19/20 live evaluation. macOS OAuth,
  refresh and persistence across rebuilt applications are verified. The director's
  private-file credential-store choice avoids repeated Keychain prompts. The
  editor remains usable offline. [Foundation](docs/spikes/phase-0.md),
  [login evidence](docs/spikes/auth-login-repair.md).
- **Assets.** glTF/GLB, PNG/JPEG/EXR and WAV/OGG import/cook with retained checked
  caches, reimport and source-independent loading. Stable cooked atlas frame IDs,
  packing, nine-slice and authoring validation are implemented. Native readers
  are below the authoring boundary. [Asset pipeline](docs/spikes/asset-pipeline.md),
  [atlas work](docs/spikes/sprite-atlases.md),
  [reader boundary](docs/spikes/cooked-visual-assets-boundary.md).
- **Native world and scripts.** Disposable cooked binary ECS scenes, typed bulk
  Transform/Velocity access, bounded structural command buffers, native input and
  physics, explicit observation, and versioned native world saves. Build-produced
  QuickJS bytecode excludes the source compiler from the normal shipped graph.
  Legacy document-backed Play still exists and is not the new runtime path.
  [Native script columns](docs/spikes/native-script-columns.md),
  [structural commands](docs/spikes/native-script-structural.md),
  [bytecode](docs/spikes/cooked-native-scripts.md),
  [world saves](docs/spikes/native-world-saves.md),
  [input](docs/spikes/native-input-session.md),
  [physics](docs/spikes/native-physics-runtime.md). Native scripts now have bounded
  committed-state raycasts, character sweeps and trigger events with revocable
  capabilities and whole-update rejection. 131 scoped tests and a 45-process
  source-free proof pass. [Queries](docs/spikes/native-script-queries.md),
  [character/trigger evidence](docs/spikes/native-script-character.md). Native fixed-tick
  timers now stage schedules alongside numeric/structural changes, with 63 script
  tests and a 48-process source-free proof. [Timer evidence](docs/spikes/native-script-timers.md). Native
  headless observation plans now check selected typed state and input actions at
  accepted ticks, return structured false-assertion reports/nonzero status, and
  preserve terminal runtime refusal. Seven focused and 577 ordinary workspace
  tests plus 54 fresh source-free process controls pass; existing project-oriented
  legacy Play and full Core Sample/agent integration remain open.
  [Observation evidence](docs/spikes/native-headless-observations.md).
- **Native rendering.** Canonical typed renderer input and a separate authoring
  adapter; exact retained asset versions; binary v4 cameras/models/lights and
  binary v5 sprites; direct committed pose updates and mandatory GPU host sync.
  Numeric moving-frame regressions cover source/cache removal, reflection,
  replacement and rejection/retry. Sprite CPU staging is allocation-free in the
  measured test; complete encoding still allocates. Shared immutable sprite
  streams reduce the calculated geometry cap from 576 MiB to 232 MiB and moving
  uploads from 64 to 12 bytes/vertex, excluding textures/metadata/driver storage.
  [Renderer boundary](docs/spikes/native-renderer-boundary.md),
  [retained frames](docs/spikes/native-retained-render-frames.md),
  [native sprites](docs/spikes/native-sprites.md),
  [memory evidence](docs/spikes/retained-sprite-memory.md).
- **Native game and audio.** Retained cooked clips, buses, spatial playback and
  streaming compose with simulation, input, scripts and GPU publication. Invalid
  preflight input is retryable; failure after acceptance destroys the composite
  state. A fresh 7,404,688-byte macOS shipping executable passes 40 process
  invocations after source/project/build removal, with exact repeated PCM/pose
  and numerical GPU hashes. The fixtures exercise v4 models and v5 atlas sprites,
  including mirrored nine-slice geometry and missing-cache/unknown-frame refusal. [Composition](docs/spikes/native-game-session.md),
  [audio host](docs/spikes/native-scene-audio-host.md),
  [GPU proof](docs/spikes/native-game-gpu-player.md),
  [sprite executable proof](docs/spikes/native-sprite-player.md).
- **WebAssembly.** Optional Wasmtime 49.0.2 development host and a statically
  compiled WABT 1.0.42/wasm2c path share copied numeric/structural capabilities.
  C traps remain in C, fuel/storage/imports are bounded, and accepted failures
  terminate the module. A fresh 2,780,816-byte macOS executable passes three
  exact-motion processes and 18 refusals after temporary sources/build removal.
  Actual iOS C/Rust compilation is compile-only. Initial static memory/table
  growth remains unsupported. A native x64 MSVC /Od baseline and /O2 candidate
  passed actual hosted conformance and a fresh optimized PE/PDB proof on PR #64.
  Static modules also compose with native input acceptance, physics and offline
  scene audio. The new `incant_input_v1` bounded snapshot preserves raw controls,
  gestures and named actions; 102 combined tests/doctests and 34 fresh local
  process checks pass. The 3,933,488-byte packaged macOS player responds to delayed
  accepted input with delayed native spawning and retains the exact audio clock.
  Hosted input results passed in the merged runtime union; complete SDK and
  session/renderer/device evidence remain open. [Input ABI](docs/spikes/native-wasm-input-abi.md).
  [Static game session](docs/spikes/native-wasm-game-session.md).
  [Development host](docs/spikes/development-wasm-host.md),
  [static host](docs/spikes/shipping-wasm2c.md),
  [MSVC follow-up](docs/spikes/wasm2c-msvc.md).

Local integration checks: 570 ordinary Rust release tests/doctests; 138 scoped
static/development-Wasm/runtime/script tests; workspace and optional-host release
Clippy; 529 UI tests, strict TypeScript and production build; 43 Python tool tests.
The renderer's unchanged frozen implementation separately passed all 66 actual
GPU tests. Exact sources, scopes and logs are retained in the linked reports;
these local results do not replace pending hosted checks or device gates.
The first hosted runtime run exposed stderr/JSON mixing in the Wasm audit and a
stale isolated physics lockfile. Reviewed fixes pass real combined Wasm
conformance, browser-target physics compilation and six native parity streams.
[Correction evidence](docs/spikes/runtime-ci-corrections.md). A subsequent Linux
C11 trap-link failure now has a reproduced POSIX declaration fix; the Windows
Camera accessibility test is split into its six independent scans with unchanged
coverage and bounded asynchronous cleanup. Both corrections passed local and hosted checks before PR #62 merged.

Native script logs now publish only after accepted numeric/structural updates,
with primitive Unicode validation, fixed per-update limits, bounded retained
history and dropped-message counts. Native/game observers hide logs after
terminal failure; the source-free player reports them once at completion.
585 ordinary workspace tests and 47 fresh CPU-mode process controls pass.
[QuickJS evidence](docs/spikes/native-script-logs.md). Additive native Wasm logs
now share its data/history contract and reach the actual static NativeSession
observer. 57 combined actual-host/runtime/SDK tests, eight QuickJS logging cases,
scoped Clippy and 16 generator/preparer cases pass locally.
[Wasm evidence](docs/spikes/native-wasm-logs.md). Full native SDK, session saves,
platform/device conformance and remaining editor/headless integration stay open.

Native Wasm localization now exposes copied locale/message queries and ordinary
atomic runtime locale settings, using the existing native catalog/ICU rules.
Actual Wasmtime, linked wasm2c and no_std Rust guest checks cover rich arguments,
fallback/pseudo/missing metadata, next-callback visibility, quota refusal, mixed
rollback, reload and static session log observation. Shared ingress failures on
active correct-epoch locale calls reject the entire batch. Standalone formatting,
reports, catalog editing, static checkpoints and platform/device gates remain
open. [Contract and evidence](docs/spikes/native-wasm-localization.md).


The reviewed native script/input stack is now tested together with the static
module lifecycle and CI/import union: 593 ordinary tests, 97 actual composed-module
tests, all-target Clippy, 86 tooling tests and two fresh shipping proofs pass.
PR76 merged after all 21 applicable checks passed. [Union evidence](docs/spikes/native-runtime-integration.md).

Native project playback now cooks authoring once and runs a disposable native
world with a trusted build-embedded behavior. The developer coordinator serializes
build/copy, removes temporary compiler inputs, and binds each private executable
to its source digest. Replay, selected observations, committed logs and optional
PCM preserve authoring and publish no partial output on runtime failure. The
integrated source passes 601 ordinary tests, 88 tooling tests, six custom-behavior
process controls and a fresh 47-process shipping-player regression proof.
[Evidence](docs/spikes/native-project-play.md). Editor Play, agent play tools,
project script routing, GPU capture and complete session saves remain open.

## Consolidated native-window and 2D integration

The ancestry-preserving union of PR63/68/73/78 on PR79/76 passes 650 ordinary
workspace tests, all 69 ignored GPU tests, both workspace Clippy profiles,
18 optional window policy/routing tests, 534 UI tests and 90 tools tests.
Generated schemas remain unchanged. Static-module composition passes 101 tests
and a fresh 34-process shipping proof; all 18 native/browser physics streams
match, and seven fresh planar shipping processes pass. Six custom native-project
runs and the fresh source-free GPU player also pass. PR80 merged after all 21
applicable checks passed; its actual-main tree equals the tested head. [Integrated evidence](docs/spikes/native-window-2d-union.md).

## Remaining engineering requirements

- Complete migration of the editor, headless runner and full gameplay SDK from
  legacy document-backed Play into the native runtime. Persistent script state,
  remaining timer/coroutine semantics, native physics/navigation/audio/UI access and full composite
  session restoration need production coverage. Do not extend the legacy tick
  path or add project cloning/JSON/authoring validation to native ticks.
- Finish render features and their native path: offline backend shader cooking,
  persistent pipeline caches, asynchronous fallback/prewarming, temporal upscaling,
  dynamic resolution, backend interop and recorded device budgets. Runtime WGSL
  compilation is still present; source-free script proofs do not certify shader
  performance. Native pixel placement, flipbooks and broader tilemap/2D-physics
  feature and device coverage remain.
- Complete animation, navigation, game UI/text/IME, localization and remaining
  input/haptics/native SDK deliverables. Existing scoped/legacy implementations
  are not evidence of full native feature parity.
- Complete Wasm gameplay SDK and module checkpoint coverage beyond the locally
  verified single-scene public import/Play/export route; establish platform/device
  conformance and measure the
  settled TypeScript AOT/physics candidate adoption rules. Do not reopen Section
  12 decisions or substitute desktop timings for reference-phone results.
- Build and exercise Core Sample and the 2D sample on the required device classes,
  including hosted real iPhone/Pixel devices through AWS Device Farm, with actual
  device model/OS, shipping profile and matched baselines. Phone script p95,
  shader-stall, frame-time and memory gates remain open.
- Deliver the remaining editor/AI/content/export/network/services/game production
  and release scope in PLAN.md. Phase 1 progress is not Engine 1.0 or Driftwake
  release completion.

## WebGPU renderer compile portability

The default renderer library compiles for wasm32 with explicit browser
NonSend graph ownership and copied GPU error diagnostics. Desktop Send/Sync
resources and typed errors remain intact. Local 36 CPU tests, scoped Clippy,
formatting, actionlint and 12 workflow-policy cases pass; 18 GPU tests stayed ignored.
The existing web checkpoint job now checks the renderer library. Browser
rendering acceptance and device/performance gates remain open.
[Evidence](docs/spikes/web-renderer-portability.md).

## Deferred gates and human-owned actions

The [Phase 0 ledger](docs/gates/phase0-review.md) remains unapproved: live
revocation verification with a suitable disposable connection, scheduled nightly
artifact history, staffing/reviewer/contracts and director approval remain open.
Do not revoke the director's working account for a test. Manual Windows/Linux
account checks and the Windows viewport review were replaced by CI under the
director's explicit decision; no extra desktop machine or live API key is requested.

Hosted device/service accounts first gate real staging or device work, not
local feature development. Payments and human staffing/legal commitments remain
human-owned. Signing credentials and store/developer accounts first gate
Phase 5 store uploads. No phone purchases, name clearance or package reservations
are required. Unsigned packages and simulator/compile-only results do not satisfy
store distribution or real-device gates.

Earlier implementation notes, superseded counts and publication chronology are
preserved in the [historical ledger](docs/spikes/implementation-history-2026-10-10.md).
Technical evidence remains under [docs/spikes](docs/spikes); `python3 tools/check_gate.py`
reports the recorded gate state without granting approval.

The reviewed general-check selector allows only nine audited plan/convention/ADR
files onto a Linux documentation path, preserving full checks for every other
change and all classification failures. Local82tool tests/actionlint pass; hosted
latency remains pending. [Policy and evidence](docs/spikes/ci-document-selection.md).

The private authored-script union now includes the reviewed PR83 policy without
changing its tested runtime/compiler/ABI inputs. Integrated verification includes
761 ordinary tests, 75 GPU tests, 178 actual-host conformance tests, full Clippy,
536 UI tests and 17 Node tests. The fresh bound-script embedding proof passed all16 copied-player processes after removing source/build storage. [Integrated evidence](docs/spikes/native-authored-sdk-integration.md).
Actual main83 is now integrated without a tree change; the authored candidate is ready for direct-main publication.

## Native skeletal model consumer — numerical GPU verified, appearance pending

The corrected glTF skeletal foundation is preserved in actual main107. Native
clock samples now bind to exact sealed clip-duration tables; retained CPU skinning
feeds the canonical material/shadow vertex path with independent live/staging
buffers. Source/cache-deleted clocks/extraction, rollback, normal/tangent basis,
uniform reflection and mixed-sign refusal have CPU controls. The affected suite
passes 155 tests, all-target Clippy and runtime dependency checks. All 70 affected
GPU tests pass on Apple M5 Pro/Metal, including independent pose, single-sided
reflection and shadow oracles. Claude appearance acceptance and the complete
authoring/export/native-Play union remain pending.
Blend/state/IK/root motion, GPU skinning and device budgets remain.
[Scope/evidence](docs/spikes/native-animation-rendering.md).

GameSession/exported-player checkpoint integration: reviewed source `0d841fcd` is preserved unchanged in main103 union `4661e254`; 34 integrated CPU tests and optional-GPU all-target Clippy pass. Original311-test/five-success/nine-refusal shipping evidence remains applicable; full-save limits are unchanged. See [integration evidence](docs/spikes/evidence/native-game-checkpoints-main103.json).

## Native Wasm localization integration

Actual main104 integration is clean: 74 development/static/SDK, 106 session library
and five GameSession checkpoint cases pass, with 20 tooling tests, Clippy,
compiler-disabled/static and dependency checks. Static checkpoints remain refused.
[Integration evidence](docs/spikes/evidence/native-wasm-localization-main104.json).

Packaged-font text integration against actual main105 passes all 15 actual-font
tests, Clippy, standalone wasm32 and runtime dependency checks, preserving all
935 prior lock records. [Integrated evidence](docs/spikes/evidence/native-game-text-main105.json).
## Cooked skeletal-animation foundation

The bounded glTF skin/TRS import, INCMOD04 cooked reader and retained CPU sampler
are implemented with reviewed clip-only root validation; see
[scope and evidence](docs/spikes/gltf-skeletal-animation.md). Static03 compatibility
is preserved. At that foundation merge, static consumers explicitly refused
animated models; the local native consumer integration is described above.
Navigation still refuses deformed geometry. Blend/state/IK/root-motion and
rendered acceptance remain outstanding; this does not complete Phase 1 animation.

Skeletal foundation main106 union passes 90 integrated CPU tests, Clippy, audio-only/wasm32 compilation and dependency checks, with reviewed source unchanged. [Integration evidence](docs/spikes/evidence/gltf-skeletal-main106.json).

Packaged-font backend/CLI import main107 integration passes affected CPU tests, Clippy, font/default-reader, wasm32 and consumer checks while preserving skeletal registration. Editor font import form and rendering remain unfinished. [Evidence](docs/spikes/evidence/packaged-font-assets-main107.json).
Native TypeScript animation snapshots and atomic playback controls are implemented
against the retained native clock. Actual QuickJS/NativeSession, multi-instance
TypeScript cook, rollback and memory-bound tests pass; see
[scope and evidence](docs/spikes/native-animation-script.md). Wasm control parity,
rendered acceptance and the remaining animation systems are still outstanding.

The public authored animation/source-free CPU export proof passes 23 numerical
checks over 160 ticks, three identical fresh shipping replays and byte-identical
41+119 checkpoint continuation including state/timers/locale/logs/PCM. Private
source/build storage is removed; refusal controls withhold save and PCM. The
separate retained fixture uses the editor-default journal and recovered Undo/Redo.
No window/GPU/device or appearance acceptance is claimed.
[Scope and evidence](docs/spikes/native-animation-export.md).

Native Play committed script logs now reach the existing Console through bounded
incremental copied tails, with generation/project fences, stable row IDs and an
explicit truncation warning across restarts. Local CPU host, bridge race/bounds,
strict types and Clippy checks pass; native window/rendered Console acceptance
remains separate. [Scope and evidence](docs/spikes/native-editor-play.md#committed-script-logs-in-console).

## Native Wasm skeletal playback — locally verified

`incant_animation_v1` and the public no_std Rust SDK now copy native animation
snapshots and queue select/seek/play/loop/speed controls in the ordinary atomic
runtime transaction. Actual Wasmtime and compiled wasm2c guests agree on playback
and reject mixed numeric/entity/animation/log prefixes on failure. The combined
281-test suite, scoped Clippy, default wasm32 compile, generated contracts and
20 tooling tests pass; independent boundary review accepted the frozen code.
[Evidence and limits](docs/spikes/native-wasm-animation.md). Publication/integration
against actual main117 now repeats all 281 actual-host tests and affected checks. This does not add
blending/state machines/IK/root motion or satisfy device/complete SDK/Phase 1 gates.

## Native text data/package foundation — historical verification

The earlier v12/font packaging foundation is merged with 320 scoped tests,
Clippy, wasm32, package/font controls and schema/SDK checks. Its then-explicit
consumer refusals remain historical evidence in
[native text bindings](docs/spikes/native-text-bindings.md). The composed CPU
consumer and current shipping-player observations are described above; GPU text
drawing, widgets and input/IME acceptance remain separate unfinished work.

## Native Wasm standalone formatting

Wasm modules now expose standalone ICU number/date formatting through additive
opcode 25 and the public no_std Rust SDK. The actual committed locale, date
lengths, Gregorian validation and number domain match TypeScript; queued locale
changes remain invisible until atomic acceptance and pseudo settings do not wrap
primitive text. Correct-epoch ingress and ignored SDK errors reject mixed work.
Frozen local validation passes 294 tests/doctests, actual Wasmtime/wasm2c guests,
all-target Clippy and default wasm32 compilation. See
[formatting evidence](docs/spikes/evidence/native-wasm-formatting.json).
Catalog editing/report paging, complete SDK and device/performance gates remain
open; existing bulk fresh-process smoke is not a formatting shipping export.

Wasm standalone formatting is integrated on actual main119 with all 23 reviewed
implementation hashes preserved. Current-source actual Wasmtime/wasm2c validation
passes 295 tests, all-target Clippy, default wasm32, 19 tooling tests and SDK/graph
checks; the three fresh bulk processes remain a separate development-only smoke.
[Receipt](docs/spikes/evidence/native-wasm-formatting-main119.json).

Retained CPU pose blending integration against actual main120 preserves all five
reviewed source hashes and passes 166 CPU tests plus Clippy/graph checks; 71 GPU
tests remain ignored. [Receipt](docs/spikes/evidence/runtime-pose-blending-main120.json).

- Native Rust/Wasm HUD: additive ops56–58 preserve ops0–55/input-v1. 365 ordinary tests, three actual Wasmtime/wasm2c tests, all-target Clippy and scoped generation/wasm/runtime guards pass. Public source/cache/build/map-deleted package passes three fresh shaped-HUD/movement/PCM runs and six exact refusals; source and final evidence reviews accepted, publisher intake pending. Module checkpoints, headless native-play UI routing and native/GPU/device gates remain open. See `docs/spikes/native-wasm-hud.md`.

- Native headless UI routing: explicit bounded viewport now reaches the retained host; CLI and native Play RPC replay the existing UI/resize format through the ordinary player's shared resolver. All 100 affected tests, scoped Clippy, input wasm32 and runtime/generation checks pass, including actual CLI/RPC parity and failure preservation. Independent source and final execution reviews accepted; publisher intake pending. No native-window/GPU/IME/device claim. See `docs/spikes/native-headless-ui.md`.
- **Native Rust/Wasm audio transport (private execution checkpoint).** Additive
  ops59–61 preserve all previous imports and expose copied desired declarations,
  exact u64 transport epochs and atomic ordered controls. Independent source
  review, ten focused tests, 483 actual-host tests in 87 groups, all-target
  Clippy and three fresh development processes pass. Full 96-tick declarations
  and real mixed PCM match the typed native oracle. Independent final host review
  accepts the retained evidence. The public import/select/Undo/Redo/reopen and
  captured-only development route passes; three fresh ordinary wasm2c players
  match every declaration field and exact PCM after private source/cache/build/map
  deletion, with 13 exact refusals. Independent final package review accepts the
  retained evidence; the historical intake state is superseded by the accepted
  main166 composition and PR168 merge. No actual-cursor,
  arbitrary guest checkpoint, physical-device or phase-gate claim. See
  [contract and evidence](docs/spikes/native-wasm-audio.md).

## Compiled physics selection — locally verified

Native deterministic, fast and optional eight-lane builds now forward their
kernel choice through 32 affected packages. Cooked scenes and embedded save
scenes refuse a foreign exact variant before world creation; legacy deterministic
bytes are preserved. All three builds passed the complete 249-test runtime suite,
105 consumer tests, five focused admission/save tests, actual headless/player/editor
all-target checks and applicable Clippy. Independent production source review is
complete; current-main integration remains separate. Per-project settings,
automatic matching Play processes and export selection are the next work, and
physics-world saves, performance/device measurements and eight-lane adoption
remain open. [Scope and retained evidence](docs/spikes/physics-mode-admission.md).

## Authored physics mode — locally verified

The document now has a strict fast/deterministic setting with preserved legacy
canonical bytes and a public, undoable `set_physics_mode` command. Cook and Play
startup refuse a mismatched process before assets or scripts; export producer
receipts identify the selected family and exact kernel. All three builds passed
134 document/command/cook/Play tests and Clippy with frozen inputs. Actual matching
launcher/export execution and editor process switching remain in progress.
[Contract and evidence](docs/spikes/physics-project-setting.md).

## Physics setting generated contract — locally verified

The public schema registry, agent-tool command schema and generated TypeScript
API now include the undoable physics mode. All 70 schemas match a fresh matching
Rust binary; strict consumer checks and the editor build pass. The integrated
cook/Play suite passes 65 tests and Clippy in each of three compiled variants.
Mode-aware launch/export and editor process switching remain separate follow-on
work. [Evidence](docs/spikes/physics-project-setting.md).

- Authored Android CPU archive route: reviewed fresh-source admission and separate physics-aware exporter pass focused Rust/Python/Clippy checks. Both deterministic and fast ordinary shipping archives pass three source/cache/build/map-deleted ARM Android17/API37 processes each, exact per-field/native/PCM parity and fourteen boundary refusals; owned emulators cleanly removed. Independent final execution review is accepted; the historical intake state is superseded by the accepted main181 composition and PR182 merge. No minimum-OS/phone/performance/platform-host claim. See `docs/spikes/authored-android-project.md`.

## Authored Android — composed main181 execution passed

Seventeen current checks pass, including both-mode mobile tests and fast Android
target compilation. A fresh deterministic archive passes three actual source-free
ARM Android17/API37 processes with exact native logs, observations and PCM plus
seven precise refusals. Owned emulator resources are removed. The earlier complete
fast shipping proof stays explicitly pre-PCM; no later fast archive rebuild is
claimed. Final independent review is accepted; publication awaits its fast check. No phone/rendering/performance gate
is approved. [Scope and evidence](docs/spikes/authored-android-main181.md).

## Authored iOS static Wasm — current-main proof complete

Main185 composition retains Android TypeScript and explicitly refuses Android
static modules. The frozen current source passes 62 cooker tests and scoped
checks, two fresh simulator shipping archives, twelve actual C/Swift positives
and fifty-two exact refusals with full native-oracle/log/PCM parity. Final
independent integration review is accepted; publication awaits its fast check. Original device links remain
producer-scoped; no phone, rendering or phase gate is claimed.
[Current evidence](docs/spikes/authored-ios-wasm-main185.md).

## Windows checkpoint conformance split

Checkpoint16 remains failed: baseline and optimized Windows conformance each
passed 484 tests, then the shared job exhausted its 90-minute limit before fresh
shipping. The reviewed repair separates four unchanged proof commands behind the
existing success-only required context; all 23 release requirements remain.
Local policy/tooling checks pass; hosted execution is pending the next frozen
checkpoint. [Evidence](docs/spikes/windows-conformance-split.md).

### Opt-in retained dynamic resolution (CPU slice)

- Bounded frame-duration control now selects retained scene attachment dimensions
  without changing output/UI size or authored pixel-camera grids. Default behavior
  is unchanged; nearest-texel presentation reuses existing WGSL.
- Native composed hosts can opt into one-slot, nonblocking queue-completion latency
  feedback. This is not isolated GPU timing or whole-game frame time; the automatic
  native clock explicitly refuses browser targets, whose controller can still use
  explicit host measurements.
- Independent source/evidence review accepts this CPU slice; publisher integration
  and Claude0067 actual callback/render acceptance remain pending.
- 73 CPU tests, Clippy, renderer wasm32 and runtime graph checks pass. All 89 GPU
  cases remain ignored. Actual callback/render execution, Claude visual acceptance,
  temporal reconstruction/MetalFX and reference-device budgets remain open. See
  [contract and evidence](docs/spikes/dynamic-resolution.md).

### Inert integration/CCD configuration (producer, final audit pending)

The std-only snapshot crate now retains pinned rigid integration settings and the
completed-step CCD fixed-target cache, including exact historical prediction bits,
ordered handles/AABBs and all three cache states. A separate default-off vendor
getter only borrows the previously serde-skipped cache. Independent source review
accepted the scoped implementation; full source003 suites pass156 each in both
numerical modes. Final source004 changes only two test constructor expressions;
focused21 each, strictClippy/wasm/lean/fmt/conventions pass. Historical failures are
retained. No live constructor/cache installer or save admission is added. See
`docs/spikes/physics-integration-config.md`. This historical final-audit/intake
status is superseded by the accepted main220 composition and PR221 merge.
Cache/shape coherence and full live-owner admission remain open.


## Private physics constructors — merged main231

Exact private storage/body/collider construction at `0a96dc0f` has independent
source/final CPU acceptance: 216 snapshot cases and 22 native controls per mode,
with 144 continuation steps per dimension. Other owners remain trusted clones.
The separately reviewed observer alias from `d6b53f8b` is included. This private
main229 composition now has independent source acceptance and21 successful
current checks:216 snapshot plus22 private native controls per parallel mode,
with144 continuation steps per dimension. Final integration review is accepted;
the disjoint main230 renderer join is independently accepted. PR231 merged
after the required exact-head fast check.
[composed evidence](docs/spikes/physics-construction-main229.md) is retained. Exact manager construction,
whole-owner admission and live physics saves remain closed.
[Producer evidence](docs/spikes/physics-native-construction.md).
