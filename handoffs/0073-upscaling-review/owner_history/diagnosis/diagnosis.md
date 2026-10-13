# Owner-history case 3: CPU diagnosis

**Status: diagnosis and proposal only.**
- No GPU work, renderer binary, build, image or retry was used, and no renderer
  source was changed.
- The failed result `13540b9c`, the GPU001 raw evidence, the frozen shader
  `df600750`, `common.wgsl` `7a104170`, the test `07b168aa` and the binaries are
  untouched.

**Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max.
Root's `0073-owner-history-diagnosis-001/acp-start.json` reads `"verified": true,
"gpu_authorized": false`.

**Bottom line.** The GPU cause is **not uniquely established**. Two mechanism
classes reproduce every printed byte; CPU models cannot tell which code path Metal
executed. Separately, the shader's changed-owner rule is semantically too weak, as
a deterministic control shows. The patch below fixes that rule, and the
discrimination probes let root's next bounded GPU packet separate the two
mechanisms.

## Observation (raw `wgsl/stdout.txt` `bf7e6397…`, `stderr.txt` `21c22e6b…`)

**What printed:**
- Case 1 and case 2 marker lines.
- Case 3 then panicked at the CENTER probe [8,8]: expected colour 0 ± 0.001, got
  `[0.40478516, 0.40478516, 0.40478516, 1.0]`.

**What did not print:** the depth and record are asserted after the colour, so
neither was checked or printed at the failing probe. Cases 4–12 and the cooked
binary never ran.

**This expectation stays a failed counterexample.**

## What the oracle is

`case3_oracle.py` is an f32-exact scalar transcription of `df600750`, run on all 12
frozen fixtures.
- **Pinned inputs:** the frozen shader, the frozen test, and the raw stdout and
  stderr.
- **Arithmetic:** each basic operation is computed in binary64 and rounded once to
  f32, which is innocuous because 53 ≥ 2·24+2. FMA is rounded exactly from
  rationals.
- **Variables:**
  - how `u=(p-o)/w` divides, and how `o+u·w` rounds;
  - changed-table and support-index mutations;
  - f16 colour storage, either round-to-nearest-even or toward zero;
  - `mix` form: Metal separate, Metal fused, or WGSL.

## Established on CPU

1. **The oracle matches the test.** Under exact IEEE arithmetic, with or without
   FMA, the unchanged shader passes all 12 unchanged test expectations. Case 3
   CENTER is 0.0, the [9,8] tap has weight exactly 0, and every inside texel has
   an exactly zero fraction.
2. **The GPU did not follow the exact model.** "Exact arithmetic + correct table +
   transcribed logic" gives 0.0 at CENTER, not the observed value.
3. **Two classes reproduce everything that printed:** the case 1 and case 2 marker
   lines byte-for-byte, and the case 3 colour.

   | Class | What happens | Combinations |
   |---|---|---|
   | **A, arithmetic** | `u=(p-1)/14` rounds at least 1 ulp high **and** `1+u·14` is fused, or u is at least 2 ulp high. CENTER then gets q = 8.5+2⁻²⁰, fract 2⁻²⁰, and tap [9,8] weight 9.5367e-7. Owner 7 enters the positive-weight support. The d90 rule then admits the `{5,7}` history, and the 3×3 ring (7's colour 0.5) lets 0.45 through. | `recip+fma`, `quot+1+fma`, `quot+2`, `quot+2+fma` |
   | **T, table** | `changed_id(7)` reads false, so the persistence branch carries 0.45. | Zeroed table, bit-reversed bit, reversed vec4 component, next-vec4 offset |

   - The same combination of both classes also reproduces.
   - Reciprocal division with separate rounding does **not** reproduce, nor does
     IEEE division, nor a quotient 1 ulp low.
   - Aliasing the header does not reproduce, because the case-3 class mask
     equals its table word (128).
4. **Index mutations alone cannot reproduce it.** These are a dropped support
   store, slot-0 overwrite and reversed slots. Under exact arithmetic every one
   leaves CENTER at 0.0; membership does not depend on slot order.
5. **f16 storage, observation only.** All 42 reproducing combinations, out of 120,
   need toward-zero f16 colour storage. Round-to-nearest reproduces none: case 2
   would print 0.7001953 rather than 0.69970703, and case 3 0.4050293. This
   suggests toward-zero conversion at the f16 target on this GPU. It is **not**
   established Metal behaviour. It cannot affect pass/fail at tolerance 0.001.
6. **Classes A and T give the same CENTER output:** colour, depth 50.05 and record
   `[P|5,7]`. So nothing printed for case 3 can separate them. Other texels can:

   | Texel | A predicts | T predicts |
   |---|---|---|
   | [3,3] | 0.0, record `[P|5,0]` | 0.40478516 |
   | [9,8] | 0.49975586 | 0.45483398 |
   | Case 10 CENTER | passes | 0.51953125 |

Root's hypothesis is class A. It is plausible and reproduced, but not unique. The
exact condition is division ≥1 ulp high together with FMA, or a quotient ≥2 ulp
high. Reciprocal division alone is not enough.

## The semantic question

**Is arbitrarily small positive current support enough to justify a large stale
contribution from a changed history contributor? No.**

**Why the d90 rule fails.** The rule (changed recorded owner admissible while
present in the 2×2 support) makes presence binary.
- Under class A, owner 7's support weight is 9.5e-7. Yet the tap's whole history
  (0.45, laid down when 7 was elsewhere) enters at blend weight 0.9: a ratio of
  about 10⁶.
- The 3×3 ring is unweighted and contains 7's current colour, so it cannot bound
  the stale value.

**It is not only about tiny weights.** History taps are fetched along the
**anchor's** motion vector, never along a different owner's. The deterministic
large-support control gives owner 7 weight 0.5 at CENTER through a half-texel
source offset. Even there, the current rule carries the stale value (0.42993164,
against 0.25 current-only).

So no epsilon or weight threshold is principled. The principled boundary is the
motion source.

## Proposed correction (`resolve_owner-anchor-change.patch`, `97029540…`)

**The rule.**
- **Ordinary reprojected tap:** it already requires `r.a == anchor`. Only that
  owner may have changed, because its motion produced the reprojection. Any other
  changed exact owner rejects the tap at every weight: `record_admissible` returns
  `r.b==0u || !changed_id(r.b)`.
- **Unchanged:**
  - overflow (any changed class rejects);
  - persistence (needs every owner unchanged);
  - recording of positive support and all 3×3 clamp contributors;
  - canonical and absent handling;
  - clear, premultiplied, finite and disocclusion contracts;
  - coordinates, thresholds, bindings and memory.

There is no new mode, target or epsilon.

**How it applies.**
- `git apply --check` applies it cleanly to `df600750`; the patched source would
  be `0618bd77…`.
- The oracle applies it in memory and verifies that hash and its rule text.

**Effects (oracle):**
- **No change under exact arithmetic:** all 12 fixtures are identical, including
  cases 7 and 8 (no changed owners, so baseline equivalence holds) and case 10.
- **Class A, case 3:** the current-only CENTER is about 4.17e-7 after f16
  storage.
- **Changed anchor (moving object):** still reprojects, identical to today.
- **Unchanged second owner:** identical to today.
- **Large-support control:** 0.25, not 0.43.
- **Class T:** **not** repaired, which is deliberate, so it stays detectable.

**Cost.**
- Edges between two depth-compatible *changed* owners, such as touching moving
  instances, become current-only where d90 kept history.
- d90's decide row "changed recorded owner still present reprojects" becomes
  current-only. The committed d90 models stay unchanged as history.

## Discrimination controls (`owner_device_tests-discrimination.patch`, `dd7b8cad…`)

`patch --dry-run` applies it to the frozen `07b168aa`. It makes two changes:
1. **Case 3 gains a probe at [3,3]** (0.0, depth z, `[P|5,0]`) before CENTER. T
   fails there first; A and exact arithmetic pass.
2. **CENTER record follows provenance.** It must be `[P|5,7]` exactly when the
   read-back CENTER colour is non-zero; otherwise it stays `[P|5,0]`.
   - CENTER's colour can only become non-zero through the [9,8] tap.
   - Colour, depth and their tolerances are unchanged.
   - This is the positive-support invariant, not a widening: the original
     `[P|5,0]` silently assumes the [9,8] weight is exactly 0.

Neither change alters submissions, maps or the marker format; only case 3 prints
`probes=2`. The runner accepts any probe count.

**Predicted outcomes** for root's next packet, which is root's decision, not
requested here:

| Shader + test | Exact arithmetic | Class A | Class T |
|---|---|---|---|
| Current + original | pass | case 3 CENTER colour 0.40478516 (observed) | the same |
| Patched + original | pass | case 3 CENTER **record** `[P|5,7]` (the original counterexample, still failing) | case 3 CENTER colour |
| Patched + discrimination | pass, CENTER 0.0 | pass, CENTER about 4.2e-7 printed in the case-3 marker | **fails at [3,3]** with 0.40478516; case 10 would also fail |

Root may instead keep the strict original record and treat a `[P|5,7]` failure
as A-evidence.

## Reproduce

```
python3 -I -B handoffs/0073-upscaling-review/owner_history/diagnosis/case3_oracle.py \
    --out handoffs/0073-upscaling-review/owner_history/diagnosis/results.json
```

- **Hashes:** `case3_oracle.py` `61cfa488…`; `results.json` `d6433fcf…`.
- **Result:** exit 0, all checks true.
- **Reproducibility:** two runs are byte-identical, and `/usr/bin/python3` 3.9.6
  gives identical output once the version string is normalised.
- **Running time:** about 2 seconds.

## Limits

- **Scope of the oracle.** It is a scalar transcription, not the naga/Metal
  pipeline. It models stylised uniform division errors (±k ulp on every texel)
  and three `mix` forms, not Apple's actual instruction selection.
- **f16 storage.** The storage conclusion is a hypothesis.
- **What the GPU printed.** The depth and record at the failing probe are
  unknown.
- **Next step.** Only a rebuilt binary run under a new root grant can settle A
  versus T (or another unmodelled path).
- **Still open:**
  - prior quality failures (edge/subpixel, C7/C9/P1);
  - the cooked backend;
  - retained history, appearance, performance and device gates.
