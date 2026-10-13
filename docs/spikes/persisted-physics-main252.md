# Persisted physics, session and player composition on main252

This composition joins the independently accepted native, outer-world and game
packets into one default-off `native-checkpoints` path. A supported native game
can save bounded physics data and resume it in a fresh player process, with
explicit script state, hosted UI and offline PCM state. Existing component saves
retain their old physics refusal. Checksums bind data and current trusted code;
they do not authenticate history or grant capabilities.

The seven reviewed implementation commits apply without source conflicts.
Ninety-three implementation files exactly match the executed `2352679e` game
composition. All 3,745 compiler/build inputs match except the independently
reviewed main251 authoring unit-test isolation. That file is under `cfg(test)` in
`incant_source`; its standalone test binary is not used by these consumer suites.
The three deterministic, fast8 and ordinary dependency graphs match exactly.
Ordinary features still exclude the snapshot dependency.

Native and outer physics subtrees match their individual producers. Their only
root-lock difference is the reviewed game-runtime edge to already-locked `sha2 0.10.9`,
which is present in the executed game composition; package identities and versions
are unchanged. Three explicit ancestry joins preserve accepted immutable evidence
objects without changing the composed tree. No private unreviewed source is copied.

Twelve current read-only checks pass. They include selected graph comparisons,
manifest/convention/diff guards and complete retained native/game verifier replays.
The first native replay exposed an evidence-path portability defect: recorded
absolute paths were interpreted relative to the verifier's current checkout/home.
The narrow helper correction names the immutable recorded provenance roots while
keeping Git reads in the actual checkout. A replay under a different synthetic
HOME passes. The original failed replay and exact correction are retained; no
native source, raw receipt, payload or executable was changed. An initial
integration assertion mislabeled the existing checksum edge as belonging to
script; the retained lock diff shows it belongs to game-runtime. Correcting that
verifier label changes no lock or source bytes.

Producer execution stays attributed to its actual generation: native 182 planar
plus 200 spatial cases per mode; outer 320 ordinary cases plus one explicitly run
large-capacity control per mode; and 267 joined script/game/player cases per mode.
The game tests include actual separate processes, 104 uninterrupted versus 67+37
resumed ticks, and exact saved bytes and fractional PCM concatenation. Counts
across packets overlap and are not summed. No new compilation or native execution
was needed for this source-equivalent main composition.

Historical failed CCD premises and tests remain in the packets. The corrected
native domain admits bounded single-pass CCD, including ordinary default-enabled
and implicit fast-body clamping; unsupported multi-pass, joint, soft-body, custom
and unproved states refuse. Imported provenance never becomes a historical
`CompletedStep` witness. Ordinary source002's 238 passing cases remain historical;
its earlier uncommitted bytes are not reconstructed. Reused static-module and
Wasmtime fixtures establish compilation only, not shipped guest persistence.

This does not qualify reference phones, live device audio, GPU saves, arbitrary
Wasm heaps, cross-backend portability or a phase gate. Logical budgets are not
allocator/RSS ceilings. Save and PCM outputs are individually atomic, not a
multi-file transaction. Current-main final review and the PR fast gate remain
pending. Checkpoint30 independently stays frozen at selected main252 and must
obtain all 23 required successes before any release promotion.

[Manifest](evidence/persisted-physics-main252/manifest.json) and the read-only
[verifier](evidence/persisted-physics-main252/verify.py) retain the exact boundaries.
