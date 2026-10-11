# 0041 — Appearance preservation after pipeline preparation

Use Claude Opus 5.5 with ACP explicitly confirming Max thinking. Astra owns the
implementation and numerical GPU checks; you own rendered appearance review.
This is a bounded regression review, not a new design pass or a device budget claim.

## Frozen candidates

Astra will copy two public `incant_headless` binaries and `binary.json` into this
worktree's ignored `artifacts/tools/`. Read and verify the manifest before every
execution. Baseline source is `74238785fe92c50daea1d8dd056492359def61f2` (PR79);
candidate production source is `21a2aa2fb9afcd5903453ca4c1cca32ebf72c6fb`.
Only pipeline creation timing changed. All six WGSL shader families are unchanged.
Initialization prepares the current bounded material/shadow/output variants;
drawing selects prepared pipelines. There is no new offline shader compiler,
persistent driver cache, async fallback, or performance-gate completion here.

Read the production diff between those commits and the existing material,
shadow, sprite and retained-render tests to choose a compact useful fixture set.
Do not rebuild any binary, run Cargo, change dependencies, or change production.

## Scope and acceptance

Use real public `init`, `import`, `rpc` and `screenshot` commands. Create all
project/scene/component changes through `command.execute`, and save through the
public command path. Author original simple glTF/image fixtures as necessary;
never edit project documents or journals directly. Compare exactly the same
saved projects/caches with the baseline and candidate in separate processes.

Cover opaque/masked/blended materials, both culling choices, mirrored placement,
directional shadows with opaque and masked casters, point/spot lighting, and
ordinary/atlas sprites including nine-slice and reflection. Reuse trustworthy
prior fixture helpers where appropriate and identify their origin. Check normal
map orientation, silhouette, depth/blending, shadow shape and output tone/color.
The Rust GPU regression separately exercises all four presentation formats;
do not claim a CLI screenshot validates every native display format.

Capture at least one accepted changed/reimported material and verify its updated
appearance against baseline. Remove source assets after cooking and compare a
fresh source-free capture. Preserve deliberate unsupported-feature refusals.
Report whether each difference is new, an existing shared limitation, or an
unsupported fixture. Never visually approve on hashes alone; inspect the actual
engine pixels yourself. Record numeric comparisons separately from your verdict.

## Ownership and limits

Only write `handoffs/0041-prewarmed-pipelines/` and ignored
`artifacts/0041-prewarmed-pipelines/`. You may create helpers and original fixture
art with provenance in those paths. Do not modify shaders, renderer, scripts,
schemas, UI, lockfiles, other handoffs or other agents' artifacts. No native UI,
audio-device use, system-wide screenshots, browser manipulation or external
accounts are needed.

Before any renderer process require `artifacts/tools/gpu-review-ready` containing
Astra's explicit release. Until it exists, do CPU-only fixture planning and wait.
At most 40 total engine captures, at most 960×540 each, with a 60-second timeout
per subprocess; reap every child. Preserve a command/hash/exit/adapter/dimensions
ledger. State explicitly when every GPU process has exited so the next agent can
use the device while you write the report. Do not launch another capture after
declaring the slot released without coordinating through Astra.

## Deliverable

Write `handoffs/0041-prewarmed-pipelines/result.md` with verified model/effort,
source/binary identities, actual OS/GPU/backend, command and image hash evidence,
appearance verdict, defects with minimal repro, and limitations. Retain a compact
selection of unedited engine captures plus reproducible helpers. Keep full raw
output, private fixture projects/cache and copied binaries ignored. Commit only
the packet with `Built-by: claude`; do not publish or merge. Astra reviews your
diff/evidence and integrates the result after numerical checks.
