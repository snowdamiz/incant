# Cooked renderer and Vulkan cache integration

This union preserves the reviewed opt-in sprite, screen-space UI, world-space
text and material shader paths, plus bounded private Vulkan pipeline storage.
No default renderer feature or WGSL arithmetic changes in this integration.
The production renderer/compiler files match accepted cache commit `6fe64538`,
which extends the accepted shader producer source. New renderer deltas are test
wrappers that require the selected backend when release CI supplies an expected
backend, plus a CPU regression for the validator.

The frozen main186-based source passed ten commands: default CPU tests (87),
combined cooked/cache CPU tests (113), all-target Clippy, wasm32 compile,
normal dependency guard, release policy, actionlint, formatting, conventions and
diff checks. The counts overlap; each CPU run explicitly ignored 89 GPU tests.
The packet records 1,959 frozen inputs and 145 renderer/compiler source rows.
No GPU work ran in this integration lane.

Claude 0062, 0063 and 0064 accepted their immutable producer Metal GPU and
appearance results. The retained receipts identify those builds, PNG pairs and
source-free rerenders. They do not claim that this union was rendered or that
Vulkan, DX12 or physical devices passed.

Existing checkpoint jobs retain default GPU tests and add the combined cooked
features. Every fixture requires actual `Metal`, `Vulkan` or `Dx12` adapter
identity; fallback adapters cannot silently satisfy another backend's gate.
Linux also runs the exact ignored cache test in four fresh processes, checking
missing, supplied, corrupted/rejected and replaced cache storage. This is a
future actual-platform gate, not a locally passed driver or cache-hit claim.
All 23 required release context names are unchanged. The current checkpoint17
remains frozen and does not include this unpublished feature union.

[Machine-readable manifest](evidence/cooked-renderer-main186.json) retains all
raw check logs losslessly, source hashes, the exact backend patch, original
reviews and an assertion-only collector. The actual main187 ancestry join preserves all renderer/compiler, lock and
backend-test workflow bytes. Its thirteen source changes are exact previously
accepted iOS export/cooker files, outside the tested renderer closure; seven were
in the broad original inventory. The qualification wrapper checks these Git
identities before replaying the original assertions without changing receipts.
Final integration review/publication is pending. No phase, phone performance, full offline renderer or measured
cache speedup approval is implied.
