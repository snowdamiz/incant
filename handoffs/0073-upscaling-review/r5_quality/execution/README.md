# R5 strict numerical checks followed by the unchanged visual protocol

Preparation only. This packet contains no GPU authority. Root must review the
final identities and issue a fresh grant before execution. Preserve all R4 and
earlier failed numerical/visual packets.

The new runner is the reviewed R4 quality runner extended with a four-child
numerical prefix. Its numerical stdout acceptance is copied from the reviewed
R4 numeric002 runner. The original lifecycle helper, quality fixture, PNG
metadata checker, artifact/event checks, scenes, thresholds and capture schedule
are unchanged. No decoded pixels are inspected by Astra.

The source is R5 `8c39fd6a`, with docs wrapper `8ae3a3ed`. The fresh quality
executable was built from all 2,839 frozen inputs plus unchanged fixture
`650316a2`, using Rust 1.99.0, explicit `aarch64-apple-darwin`, locked/offline CI
profile, two Cargo jobs and two Rayon threads. The build, scoped strict Clippy
and five-name ignored listing passed. The binary was copied immediately after
the build, before Clippy. Its build receipt and release are retained under
`~/.codex/incant-evidence/r5-quality-build-001`.

Run order is closed:

1. Ordinary default: modes 0–18, both attachments; 19 submissions/maps.
2. Owner default: all 12 cases, including case 3 current-only whole-plane
   equality and cases 7/8 baseline equality; 12 submissions/maps.
3. Ordinary cooked-native: the same 19 submissions/maps.
4. Owner cooked-native: the same 12 submissions/maps.
5. Stationary quality: 118 submissions, 50 maps, 6 PNGs.
6. Motion quality: 88 submissions, 44 maps, 4 PNGs.
7. Transitions quality: 212 submissions, 106 maps, 2 PNGs.
8. C7 disocclusion observation: 164 submissions, 82 maps, 2 PNGs.
9. P1 crossing observation: 196 submissions, 98 maps, 1 PNG.

All four numerical children must pass before any quality child launches. Every
attempt is debited before launch; a failed process or contract stops all later
children. The total ceiling is nine test children, 840 queue submissions, 442
maps and 15 PNGs. Numerical targets are at most 16×16; quality outputs remain
960×540. Each child has a 90-second timeout (810 seconds cumulative child
ceiling). No retry, enumeration, build, extra probe, capture or repair is allowed
during execution. Numeric submissions include their packed readback within the
same encoder; they do not add separate readback submissions.

The runner uses one global GPU lock for the complete sequence, regular retained
stdout/stderr files, exact copied binaries and an environment allowlist. It
checks the actual Metal adapter across both kinds of child, verifies each
binary again immediately before launch, preserves BaseException cleanup, and
releases only after owned processes and groups are empty. Failed-attempt PNGs
remain available even if the artifact contract rejects them.

`runner.py check /absolute/path/plan-ready.json` is CPU-only. `run` requires the
root grant at the exact evidence directory in the plan. The grant must bind this
runner, plan, all three binary identities, lifecycle helper, exact budget and
the actual predecessor release. The template is intentionally not authority.

After a future grant, use the existing 0073 ACP route with Claude Opus 5.5 and
explicit Max verified before the prompt, auto-memory disabled. Claude must
inspect every produced PNG and judge R5 honestly against the retained R4 result
and existing checklist. Numeric or artifact success does not imply appearance
acceptance. The old C7 linear/sRGB distinction, C9 finite-reference limitation
and limited P1 fixture coverage remain explicit; observations cannot close
differently defined historical gates.

R5 motion slack and LOD floor remain empirical candidates. They are not proofs
of same-surface identity, quantization error or independent samples. Thin lines
over clear coverage and low-scale first-entry/postcut quality remain open until
the actual rendered result is assessed. Do not reinterpret CPU controls as
measured pixel rejection or a full quality repair.
