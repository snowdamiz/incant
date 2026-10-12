# Compound animation graph transitions

Private implementation, not a completed animation gate. Nonzero authored edges
between clips, one-dimensional blends and nested weighted trees retain both
state clocks and their own interpolation rules. A compound fade stores at most
eight samples per side; the outer transition uses shortest-arc TRS interpolation.
A spherical pair is never flattened into a weighted quaternion mean. Parameters
remain shared while the outgoing speed is captured at entry. Master pause stops
both clocks and the fade; natural incoming completion does not stop the fade.
An active fade owns the interval, and selecting a state explicitly cancels it.

Cooked scene revision 18 admits these edges, after the reserved revision 17 root
motion section. This private implementation writes only an empty revision 17
section and refuses nonempty root data until the separately implemented decoder
is composed. Older cooked revisions still reject compound edges. World save
revision 10 retains the outgoing state and its independent clock in a bounded,
validated extension. The root-motion work owns world revision 11.

The TypeScript bulk snapshot exposes ordered groups, interpolation kinds, outer
weight and outgoing state clocks as owned columns. Existing Wasm pose v1-v4 and
graph v1 readers explicitly refuse active compound fades without writing guest
memory or committing a command prefix. A complete compound Wasm protocol remains
required; rejection is a compatibility boundary, not completed public SDK parity.
The ordinary QuickJS export proof now passes as described below.

The CPU animation player now offers one staged sample, optional root-translation/
rotation removal, ordered IK solve and final pose publication. Scale is preserved;
root identity/topology remains the caller's cold admission responsibility. A late
IK numerical refusal cannot publish newly sampled locals, globals or palettes.
The retained renderer uses this path with no root removal until root bindings
are integrated. No visual or GPU acceptance is implied by numerical CPU tests.

Focused checks pass: 17 runtime graph tests; 23 TypeScript and 26 Wasm animation
tests; 43 skeletal asset tests including zero-allocation staged publication;
14 renderer animation CPU tests with two GPU tests ignored. Initial compile and
fixture failures are retained under ~/.codex/incant-evidence/grouped-fade-*.
The full affected-package suite now passes 749 CPU tests in 113 groups, with 82
GPU tests explicitly ignored. All-target Clippy, wasm32 renderer compilation,
lean runtime dependency checks, strict SDK TypeScript and formatting/conventions
pass on the frozen inputs. Independent source review accepted the 389481c7 core
with no correctness blocker; the later authoring-test correction changes no
production code. [Raw logs and identities](evidence/native-compound-fades.json)
retain the earlier failures and their corrections. Current-main integration remains pending.

## Ordinary exported game and save continuation

`tools/probes/native-compound-animation-export.py` authors the project through
public commands, including the persistent TypeScript behavior, then checks Undo,
Redo, journal reopen and Play isolation. A closed-form integer-quarter-second
timeline independently checks every incoming/outgoing sample, inner interpolation,
outer weight, captured speed, shared parameter and clock. It covers eight-to-eight
tree fades, weighted-to-pair, pair-to-clip and clip-to-weighted transitions; zero
outer weight, master pause, opposite incoming/outgoing speeds, natural completion,
automatic edge deferral and explicit state-selection cancellation.

The ordinary exporter produces a fresh shipping-profile player. Private build
storage, authored project and caches, and external source maps are removed before
three complete 56-tick runs and ten checkpoint split/continuation pairs. Each full
run checks 112 complete log records and 224 public vector assertions. Every split
matches exact final save bytes, concatenated PCM, complete logs and final CPU
render input; per-instance persistent script counters survive reopening. Active
compound saves use world10, while the noncompound manual-cancel split retains
world6. All 1,126 engine inputs are unchanged from the accepted full core checks.

Two negative controls bind exact diagnostics and absent PCM/save files. A corrupt
checkpoint refuses before any stdout. An observation assertion failure emits its
complete failed diagnostic report before withholding output files, as designed.
An initial review follow-up incorrectly required empty stdout for that second
case. The final helper instead checks all four false assertions, tick and complete
logs; both controls passed again on the unchanged player. No engine change or
recook was used to satisfy the corrected evidence requirement.

[Public package identities, raw logs and continuation saves](evidence/native-compound-export.json)
retain the original complete run and the precise output-contract follow-up.
This proves local CPU behavior through the ordinary QuickJS export path; combined
authored IK/root-motion and Wasm continuation, GPU appearance, device performance
and phase gates remain separate work.
