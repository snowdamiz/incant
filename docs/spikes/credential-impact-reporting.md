# Credential dependency impact reporting

This slice closes missing credential-workflow source triggers and records a
conservative dependency report. It does not skip a test, change a matrix identity,
change cache policy, or filter the main general workflow. The `storage` matrix
still executes the same locked package tests and actual platform credential
persistence probes on macOS, Linux and Windows. Its original required proof
artifact remains separate, with missing-file errors unchanged.

`tools/credential_impact.py` reads Git blobs and Cargo.lock without running Cargo,
build scripts, network requests or project-provided executables. It accepts recorded full
commit IDs, requires an available ancestor baseline, and reads a complete
NUL-delimited Git diff. Rename detection is disabled so both old and new paths
appear. Shallow/missing history, unknown lock formats, missing or ambiguous package
edges, non-PR events and every non-lock input report a full run. An unexpected
diagnostic-step failure also leaves the real test steps enabled.

The only positive classification is an exact `Cargo.lock`-only change whose
reachable package identities, checksums, sources and dependency edges are
identical. Workspace manifests and feature declarations must therefore be
unchanged. Every lock edge is retained, including target, optional, build and
transitive dev dependencies. This deliberately overestimates a host's actual
package-test graph; it is not a feature resolver. The report's execution decision
is always `run_all`, including an `unaffected` report. Enabling actual selection
would require a separate review and broader host/build evidence.

At main57, the overestimate contains 559 packages and 14 local packages, compared
with 506 packages in the previously measured Cargo metadata closure. The wider
set includes transitive dev-only physics/core inputs. The credential PR trigger
now covers all 14 local directories, nested manifests/configuration/toolchains,
and the report code/tests. A regression test compares the current locked local
closure with tracked manifests and these trigger roots so a new dependency cannot
silently outgrow the list. The existing workflow-level path filter is still a
separate, explicit list; this report does not claim arbitrary unlisted files are
already tested by that workflow.

The committed historical fixture records the complete main56-to-main57 diff.
Although the narrower external metadata closure was unchanged, that change
modified reachable source and manifests and correctly reports a full run. The
lock overestimate also grows from 558 to 559 packages. Synthetic tests cover
unrelated lock records, transitive checksums, Git revisions, every edge category,
unknown/ambiguous lock records, source/config/fixture changes, exact commit
selection, deletion/rename with an embedded newline, and missing baselines.

Local validation: 53 Python tools tests, pinned actionlint, generated conventions
and whitespace checks pass. Hosted report collection remains pending. This is
coverage repair and measurement groundwork, not a measured latency improvement.

## Main producer filtering remains deferred

The fresh repository policy inspection returned no main branch protection or
rulesets. A prepared main-general cache-input path filter is therefore not
applied: main still receives actual general source validation. No branch policy,
runner tier, paid resource, extra dispatch or broad skip heuristic is introduced.

## Registry rollout evidence

The preceding reviewed cache policy now has successful natural main publishers:
454,726,422 bytes across three registry/git archives. Verified obsolete native
generations were retired under the bounded replacement policy; the resulting
inventory retained 6,968,145,208 bytes, with every other entry preserved. The iOS
target family is currently absent and a later substantive consumer must establish
actual restoration. [Exact metadata, log hashes and cleanup boundaries](evidence/ci-registry-rollout-2026-10-11.json)
include the earlier concurrent disappearance and the separately preserved user
PLAN merge whose historical merge message lacks an author trailer.
