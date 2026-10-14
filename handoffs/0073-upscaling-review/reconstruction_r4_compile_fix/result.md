# 0073 R4 compile fix: reserved WGSL identifier

**Status:** source fix complete and verified with scalar checks only. Not built, naga-validated, cooked, run on a GPU or rendered by me. This grants no GPU or appearance acceptance.

**Model:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max. Per the packet, root verified both through ACP. Auto-memory is off.

**Cause:** Astra's naga parse reported "name `coherent` is a reserved keyword".

**Fix:** in the R4 copies at `dc7701b2`, the helper `coherent` is renamed to `motion_support_coherent`. Each shader changes in exactly two places: the definition and its single call (`resolve.wgsl` lines 82 and 109; `resolve_owner.wgsl` lines 105 and 252).

**Hashes:** the only other change is the `Upscale` and `UpscaleOwner` combined trusted hashes in `incant_shader_cook/src/temporal/mod.rs`. Each hash covers `common.wgsl + "\n" + program`.

**Not changed:** no algorithm, parameter, layout, memory, threshold, comment or other shader byte changed. The prior R4 files, the root `result.md` and the tests are untouched.

| Item | R4 (`dc7701b2`) | Fixed |
|---|---|---|
| `resolve.wgsl` file sha256 | `240069814b80af16133998892f2865cc5227f2cb9fcf534977feace3443b0ef5` | `a0853f1c934a3e68fa5a69904c4308e53176ae75d32af9bd1065236fdef43607` |
| `resolve_owner.wgsl` file sha256 | `bdad50d3a8869563921ccaa2515c9eaa1b619d5f8cdf414bd59f972ed57fc14d` | `dc43ac2aea2c057cead64cb97bef1fb2429b8eadde52c20a5605e4d7487a51fe` |
| `Upscale` pin (combined) | `9c515525e946a314a846d70f6143f1b72213826189beb4abb59c06ba14bd2836` | `de4df804c0fd93fd13c849575cbf4c4fc6184baa9988adb21c535273cfd1d3a3` |
| `UpscaleOwner` pin (combined) | `c48f648d65b40a7bcb5dc44c388985ff79555a74ad22f67b4084c712eee82003` | `0897bfd2ed5e8f8b8f9fc0b99d3e2c528fb29e677635e188da835405ffedeb81` |
| `temporal/mod.rs` file sha256 | `1992e76021dd155169adede92130f91018b0476eb3fa54ed6cdd9bb2c618e5e6` | `2cbaeb743ce9fefcc666bbcc2ff81bae4e87896bbcafd9a00e377e64251d3f77` |

The pins are computed against the unchanged 76be `common.wgsl`.

**Patch:**
- `compile_fix.patch` (sha256 `44c1d5544e13efec368e71e65bcff0e8b5182c4a865518c2236c0044c0f4c639`) covers 3 files and applies against the R4 copies.
- It passes `git apply --check` in `/tmp/incant-0073-r4-compile-fix-*`, which was deleted afterwards.
- Applying it reproduces the corrected copies under `src/` byte for byte.

## Checks (`compile_fix.py` → `result.json`, all pass)

1. **Rename scope:** each shader had exactly two whole-word `coherent` occurrences, and both were renamed. `motion_support_coherent` did not appear before the rename.
2. **Identifier-only equivalence:** reversing the substitution (`motion_support_coherent` → `coherent`) recovers both R4 shader files byte for byte. The line-level diff shows only the two renamed lines per shader.
3. **Pin rows:**
   - Each old pin row in `mod.rs` equals the R4 combined hash, and the new row equals the fixed combined hash.
   - The `mod.rs` diff is exactly eight byte-array lines (two pins × two lines, removed and added).
   - Comments and every other pin are unchanged.
4. **Reserved-word scan (read-only):**
   - Names declared in each shader were compared with the 410 identifiers in naga 29.0.4's local `src/keywords/wgsl.rs`.
   - The R4 bytes match exactly `[coherent]`; this negative control is asserted.
   - The fixed shaders match none.
   - This is a lexical scan, not a parse. Only the next real naga run can confirm that no other parse or validation errors remain.

**Run:**

```sh
python3 -I -B handoffs/0073-upscaling-review/reconstruction_r4_compile_fix/compile_fix.py \
    --base /Users/andreyyurlov/.codex/worktrees/owner-quality-repair/incant \
    --naga-keywords ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/naga-29.0.4/src/keywords/wgsl.rs
```

**Not run:** Cargo, naga, the cooker, GPU work, images and screenshots, all excluded by the packet. Astra owns the separate test and oracle repairs.
