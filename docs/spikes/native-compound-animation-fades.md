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
memory or committing a command prefix. A complete compound Wasm protocol and
ordinary source-free export proof remain required; rejection is a compatibility
boundary, not completed public SDK parity.

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
The full affected-package/Clippy/wasm/dependency chain is running against the
immutable inputs in grouped-fade-core-checks-001/freeze.json. Independent review,
full package continuation and current-main integration remain pending.
