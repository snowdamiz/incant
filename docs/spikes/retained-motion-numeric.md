# Retained motion GPU numerical validation

The two frozen default-feature tests now execute once each and pass under Claude
Opus 5.5 with explicitly verified Max effort. They validate accepted/failed
submission ownership, previous rigid transforms, mirroring, resets/replacement,
source-deleted assets and exact live skinned/IK buffers using numeric GPU readback.
At most nine tiny frames were submitted. Both child processes and groups were
reaped; no Cargo target was used. The original wrong exact-test filter was caught
before GPU execution, corrected by Astra and rerun under a new reviewed grant.

[Evidence](evidence/retained-motion-numeric.json) retains the two raw test outputs,
process ledger/releases, both grants, preflight diagnosis, Claude result and
independent final acceptance. The frozen producer source and prior CPU evidence
remain in [the motion contract](retained-motion-history.md).

The existing tests print pass/fail assertions, not adapter identity or scalar
values. This is scoped numerical evidence, not appearance, a specific backend,
reference-phone performance, full temporal antialiasing or phase-gate approval.
Default rendering remains unchanged; native cooked combinations still refuse
this opt-in path. Temporal color accumulation is validated separately.
