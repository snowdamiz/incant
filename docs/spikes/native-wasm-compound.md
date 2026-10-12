# Complete compound animation through native Wasm

Operations 47–52 add pose-v5, IK-v2 and graph-v2 length/read pairs. Operations
0–46 and all prior wire records remain unchanged. Poses preserve independently
evaluated clip, spherical-pair and quaternion-mean groups, followed by an outer
TRS crossfade; they are never flattened. Each compound side has at most eight
samples. A general standalone weighted pose retains the existing sixteen-sample
typed contract; authored graph states remain limited to eight.

IK-v2 includes the complete pose and ordered model-space constraints. Graph-v2
includes captured outgoing clocks and current shared parameters. Controls use
the existing accepted-tick transaction. Sparse owner/descriptor membership,
all numerical values, inactive endpoints and both nested frames validate before
borrowed SDK views. Legacy readers refuse incomplete representations, including
after a complete cache is populated; caught active errors poison the whole tick.

Source review accepted all 40 frozen inputs. The mechanical renderer correction
restores accepted 21791d21 bytes after a duplicated automatic merge block; no new
renderer behavior belongs to this protocol. See the [source review](evidence/native-wasm-compound/source-review.json)
and [source inventory](evidence/native-wasm-compound/source-freeze.json).

At this source commit, focused real Wasmtime/wasm2c conformance and public
authoring/native-oracle development checks have passed. The final broad suite
and fresh ordinary public export are still running, and execution acceptance
remains pending. This is no GPU, appearance, device-performance or phase gate
claim. Whole-module checkpoint persistence remains explicitly unsupported.
