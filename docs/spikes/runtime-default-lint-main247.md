# Native owner default feature qualification

Release checkpoint 28 found `clippy::derivable_impls` in both native owner
wrappers when `native-checkpoints` is disabled. Ordinary worlds can derive
`Default`; checked owners require the explicit `for_simulation` constructor.
The wrappers now derive only in the ordinary configuration and implement the
explicit constructor only in the checked configuration. This preserves both
initialization paths and removes the feature-dependent lint.

Based on main247 `e301af8110c47aa2cf4e193579eeeefeb14a249f`. Both physics owner
crates pass strict all-target Clippy with native checkpoints off/on in both
deterministic and fast-simd8 modes. Formatting and diff checks pass. The
[portable receipts](evidence/runtime-default-lint-main247/manifest.json) retain
all four actual commands and logs with exact edited-source hashes. These are
local macOS checks; the failed hosted results remain failed until a new frozen
release candidate completes the required checks. No runtime behavior or public
save capability changes.
