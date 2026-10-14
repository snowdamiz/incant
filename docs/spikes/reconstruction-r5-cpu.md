# R5 reconstruction candidate: scoped CPU and cooker qualification

The private candidate changes clamp-displacement reaction in both ordinary resolve paths, an empirical `r/16` output-pixel motion tolerance, and a matching `-1.5` lower LOD bias limit for internal material/MASK draws. These are a limited quality experiment. Ordinary-UP thin geometry over clear support and low-scale fresh-frame reconstruction remain incomplete. The R4 appearance rejection and earlier failures stand; no R5 GPU or appearance result exists.

The exact seven-file Claude packet is `51deb3848801f46c78ea85ca65fc330c5f60b758`, patch SHA-256 `8e226ad36180561868b899b54188fabfa2a4bcf692bc0ec43df5e7b1e6bc8253`. It was applied to preserved R4 `825ff047284e84252d00f786a7b14cb6ecb44293` as `268f3b57228ab7bef47511103d0c97c2af057d56`, with Claude's author trailer. Mechanical formatting commit `8c39fd6adbf95d470388867de4d305756b05913c` changes only the test-oracle closure's expression block/line wrapping. The other 2,838 build inputs, including all shaders, are exact to the Claude import.

The first qualification stopped at rustfmt exit 1. Its original log, source freeze and target-release receipt remain in the archive. The second frozen attempt passed all eight commands on pinned Rust 1.99, offline/locked profile `ci`, one serial builder with two jobs and two Rayon workers:

| Check | Actual result |
|---|---|
| Default renderer library | 167 passed, 43 GPU tests ignored |
| Complete shader cooker suite | 34 passed, 0 failed; includes native shader round-trip checks |
| Cooked-temporal + native-MetalFX renderer library | 182 passed, 44 GPU tests ignored |
| Strict all-target renderer Clippy, default | Passed |
| Strict all-target renderer Clippy, cooked/native | Passed |
| wasm32 renderer library compilation | Passed; five pre-existing timing dead-code diagnostics retained |
| Workspace rustfmt | Passed |
| Diff whitespace check | Passed |

The renderer configurations substantially overlap; their counts are not independent coverage totals. No ignored GPU test ran. Ten executables were copied immediately after their corresponding commands, with original compiler feature/profile identities and executable permissions. The default renderer copy is `f4123f827f5696d496bbc6b1fe785c697d02f2af775cee6941ad4d0a8d65c9bf`; cooked/native is `f74086ba2577a7a930110c30c168df662fd88cc0719627323301dbc2e614440e`. All command process groups were reaped and the private CPU target was released.

[The compact archive and verifier](evidence/reconstruction-r5-cpu/manifest.json) retain 58 records in 42 deduplicated payloads, both source generations, the failed fmt check, all passing logs and executable identities. The final source freeze is `d615ab15dfc3ae10ec0bd060b9df2d473854cdae093c98514b28ebee93e37407` over 2,839 build inputs. The collector uses immutable source Git objects and can verify the original records and local binary copies:

```sh
python3 -I -B docs/spikes/evidence/reconstruction-r5-cpu/collect.py --verify-only --originals --local-binaries
```

The corrected scalar model uses the pre-add history weight `W = min(C, cap) * k`; its illustrative fractions are 1/16, 1/8 and 1/37. It is a constant-reaction linearization, not measured aliasing or per-pixel confidence. The `-1.5` and `r/16` policies are empirical candidates, not a sampling theorem, positional-quantization bound or same-surface proof. The corrected section in Claude's report supersedes the retained older claims. The reference follows the explicitly changed policy; the strict numeric fixture tolerances, half-interval derivation, original scenes, schedules and historical appearance thresholds are unchanged.

No uniform, binding, target, pass, allocation ceiling or history transaction changed. Nineteen of the 21 WGSL files are byte-identical to the R4 baseline. The two changed trusted shader hashes match their exact common-plus-program source. Unchanged source is not a measurement of GPU memory, cost or quality. Next work requires a separately reviewed numerical/render plan and GPU lease, followed by Claude Opus 5.5 at explicitly verified Max for actual appearance review.
