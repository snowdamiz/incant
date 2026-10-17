# Reconstruction complete-route cost: negative result

E040 rejects the current private reconstruction layout as a performance candidate.
On the matched diagnostic workload, guarded reconstruction took **94–97 ms** of
complete operational wall time versus **about 3 ms** for original shading. Its
required CPU admission alone took **76–79 ms**. This result does not adopt renderer
code, approve appearance, or satisfy a shipping-profile/reference-device gate.

The experiment ran on 2026-10-17 on **Mac17,9, Apple M5 Pro, arm64, Metal,
macOS 27.0.1 (26A434)** with exclusive quiet GPU access and native builds held.
Rust 1.99.0 used the workspace `ci` profile (`opt-level=1`), with test-only package
overrides `incant_render.opt-level=2` and `sha2.opt-level=3`. It was an offscreen
960×540, four-sample MSAA fixture with 720×405 coefficient shading. The three fixed
poses were frames 32, 36 and 40 from the retained reconstruction experiment. This
was a **current-only diagnostic**, not a continuous motion or shipping frame test.

## Matched result

O is unchanged original shading. L is the qualified reconstruction route with
late output masking. G uses the same route and selected material arithmetic, but
returns before expensive lighting for unneeded aligned quads, after the five
material texture samples. Unsupported material classes retain ordinary shading.

All values below are medians in milliseconds; each main cell has 12 observations.

| Pose | O complete wall | L complete wall | G complete wall | G CPU admission | Paired G − O wall |
| --- | ---: | ---: | ---: | ---: | ---: |
| 32 | 2.973 | 99.310 | 96.958 | 79.028 | +93.985 |
| 36 | 2.986 | 94.052 | 95.219 | 77.536 | +92.191 |
| 40 | 2.958 | 93.869 | 94.324 | 75.810 | +91.099 |

All **36 paired G − O wall differences were positive**, ranging from +86.184 to
+98.390 ms. The candidate performs four required GPU-to-CPU admission reads,
**74,649,600 bytes per route**, then scans guides and status before repair. Its
explicit resource ledger is **395,032,840 bytes**; physical driver/pipeline
allocation is unmeasured. This layout is unsuitable as a shipping design even
though a narrower shader optimization could eventually prove useful.

Operational wall includes preparation/encoding, source shadows and grid,
required map completion, CPU quad/domain admission, repair, presentation, fixed
MSAA resolve and the production OutputPass through final completion. Evidence
file writes, SHA checks, publication of JSON, query observation and later image
readbacks occur outside this endpoint. They were not subtracted afterward.
Required CPU admission was retained inside the route, not hidden as observation.

The fixed schedule uses all six O/L/G permutations: one warmup cycle followed by
two measured cycles. Separate O and G outer/detailed/detailed/outer controls have
one warmup and two measured cycles. All **234 renders, 1,522 maps, 1,756 submissions
and 5,436 descriptor markers** completed. No best-run selection was applied.

## Timestamp limits

The private marker is a real 1×1 clear render pass with an end-fragment descriptor
timestamp. Its sum describes observed point-to-point spans, **not hardware-wide
utilized time or proof that unrelated pipeline work is serialized**. The logical
marker attachment is four bytes; actual physical allocation is unmeasured.

| Pose | O marker span sum | L marker span sum | G marker span sum | Paired G − L span | G added-detail ABBA overhead |
| --- | ---: | ---: | ---: | ---: | ---: |
| 32 | 0.553 | 3.603 | 3.539 | +0.056 | 2.431 |
| 36 | 0.143 | 3.158 | 3.192 | +0.110 | 2.273 |
| 40 | 1.006 | 3.455 | 3.476 | −0.049 | 2.159 |

G − L span signs vary within every pose, and added marker overhead is larger than
the proposed shader saving. The guard's isolated cost benefit is **inconclusive**.
No overhead quantiles are subtracted to manufacture a speedup. The complete route
loses decisively regardless of that narrower uncertainty.

## Correctness, failures and provenance

Fresh same-pose same-route output/sample comparisons and retained historical
hashes passed. Exact metadata, coverage, alpha and repaired samples remained
required; ordinary-versus-coarse radiance differences were retained. This does
**not** mean the reconstructed route equals original shading everywhere. Old
coarse residuals, appearance rejections and the limited historical sampled-still
review remain in force. E040 does not expand that visual scope.

The two preceding failures remain recorded:

- **E038:** the first O query returned six zero counters; native exit 101 and
  wrapper exit 1. Its 48 raw zero bytes and failure receipt are included. No
  accepted timing row or data analyzer ran. The subsequent private descriptor
  marker changed instrumentation only, not the production HAL or material math.
- **E039:** 116 valid records preceded a supervisor `FileNotFoundError` during live
  output enumeration while successful native raw files were being released.
  The wrapper exited 1 and reaped the native child with −9. Exact failing filename
  attribution was unavailable. E040 only tolerates disappearance during live
  enumeration; symlink, final inventory, file/count/total and disk limits stay
  strict. Ten deterministic lifecycle controls passed. The compiled artifact and
  234-render schedule were unchanged.

E040's actual managed session **34550 exited 0**. Native and analyzer exited 0;
native duration was 122.252 s, wrapper 124.934 s. Both owned PID/groups (46743 and
47405) were positively absent afterward and independently checked by root.
Cumulative native time including E038/E039 is 183.481 s. E038's 66,617,114 bytes and
E039's 80,491,161 bytes of failed output remain separately retained locally.

The [portable evidence](evidence/reconstruction-complete-route-cost/README.md)
contains exact original bytes for all 234 timing records, all 234 parity receipts,
all 234 raw timestamp files, and the result/resource records. Its bounded analyzer
reproduces the full published analysis without native execution. Root also
independently rehashed all 861 retained local output files (4,890,764 bytes) and
reproduced the original analysis excluding elapsed analyzer time.

Successful image payloads were released only after native exact comparisons;
they are **not** in this bundle. Older large captures deleted during authorized
disk cleanup are also unavailable. Digest/parity receipts are not substitute
pixels for new appearance approval. Artifact SHA-256 is
`d7c7f50972f18954464c0d66cf6847686a715ae7dd6d99f191fe1ee1eaeea23b`;
[provenance](evidence/reconstruction-complete-route-cost/provenance.json) records
source, proposal, archive and original-file identities. No experimental renderer
source is adopted by this documentation change.

## Next decision

Move required classification and bounded quad-domain admission onto the GPU,
remove admission readbacks and diagnostic-only resident buffers, and reduce
repeated passes while preserving ordinary fallback and the existing quality
oracle. That is a new unqualified variant, not a conclusion that GPU-only work
will beat the roughly 3 ms original route. It needs independent admission tests,
exact output/repair controls, actual motion review, then complete-route cost
measurement before any shipping integration or adoption.
