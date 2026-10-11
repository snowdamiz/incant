# Color mip roles: bounded appearance delta

Claude Opus5.5 with Max explicitly set and verified through ACP owns this review.
Astra owns the nonvisual production patch and numerical tests. Read AGENTS.md and
`docs/spikes/color-mip-roles.md`. Preserve0040/0041 original evidence unchanged.
This packet remains CPU-only until Astra supplies exact frozen baseline/candidate
binaries and a per-subprocess capture guard with an identity-specific grant.
Use only `tools/engine.py` configure("baseline"/"candidate") for CLI calls; it
reuses the original harness under separate immutable binary identities and
rechecks the specific grant before EVERY process. Do not run Cargo, build a renderer, open a native app, bypass a gate, create the
grant, change production source/style/shaders, or publish/merge anything.

The baseline is the accepted35f3f0d tangent implementation from PR77; its generated
tangent fix is shared by the candidate. The candidate's separate cook change
introduces alpha-independent OPAQUE base/emissive/Color environment mips, keeping
existing MASK/BLEND/sprite filtering. Both policies use identical source level0.
New model3/Color manifest3 recipes require recook; immutable legacy versions retain
the historical result and source-free readers cannot repair them.

Build a small bounded delta using existing0041 material controls where practical.
The byte-identical `references/0041-fixtures.py` and its provenance note are
available without executing the old runner.
Include mixed-alpha and all-zero-alpha color under OPAQUE minification (not just a
single fallback all-zero case), emissive maps including a blended material, and
one source shared by OPAQUE/MASK/BLEND consumers. Compare against an all-alpha-one
RGB control where alpha is ignored. Check that MASK/BLEND holes/edges and sprite
coverage preserve their intended appearance, including one standalone Color
asset used by both a sprite and the environment. Include ordinary opaque-alpha
input, normal-map/Linear controls, old retained cache versus command recook, and
fresh source-free replay. Do not treat the expected historical old-cache defect
as a source-free runtime regression.

Capture only via Astra's supplied guard. Each subprocess must recheck the slot
and exact binary hash immediately before spawning, enforce a hard timeout and
reap its own child. Proposed cap:20 engine frames, each at most960x540; stop and
report a minimal repro if a defect is found, rather than expanding the matrix.
No foreground input, accounts, GUI changes or performance measurement is needed.
After the last renderer child exits, remove no files except your own generated
scratch, write a captures-complete ledger with all PIDs/statuses/hashes, and tell
Astra immediately. Report writing must not retain the graphics slot; no further
renderer may launch after release.

Commit only packet helpers, bounded evidence/captures and result.md with
Built-by: claude. Report exact source/binary identities, ACP model+Max proof,
scoped accept/reject, preservation controls, original-versus-new cache policy,
remaining limits, and all known timing/slot exceptions honestly. No phase gate,
reference-phone or full FBX-support claim is implied.
