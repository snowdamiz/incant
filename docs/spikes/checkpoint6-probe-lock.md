# Checkpoint6 isolated physics lock correction

Frozen release checkpoint6 selected main110. Its macOS, Linux and Windows native
physics jobs passed the earlier candidate/parity controls, then failed before
native-physics Clippy because `--locked` refused its stale isolated lockfile.
Local `cargo metadata --locked --offline` reproduces the failure. Cargo's minimal
offline refresh adds one existing `serde` edge to `incant_runtime`; no selected
package, version or checksum changes.

The existing fast structural guard only followed required local paths. It now
also verifies required registry dependency names, including inherited string/table
specifications, renamed packages and target sections. Three regression cases fail
against the old guard; all nine pass after correction. Optional feature resolution
and full version/source/target resolution remain Cargo's responsibility.

All four committed standalone probe manifests resolve with `--locked --offline`.
The exact failed native-physics release/all-target Clippy command passes locally.
Full tooling passes 218 Python cases with four skipped and 19 Node cases. The root
independent source review accepts the bounded patch. No engine behavior, physics
arithmetic, workflow coverage or release context changes.

Checkpoint6 stays failed and frozen while other jobs finish. Actual hosted repair
confirmation belongs to the next deliberate candidate refresh; this local result
does not turn failed checks into passing ones. The separate desktop observation
routing failure is outside this correction.

[Source, failed logs and local checks](evidence/checkpoint6-probe-lock.json),
[independent review](evidence/checkpoint6-probe-lock-root-review.json).
