# Authored Wasm save/load through ordinary shipping export

An ordinary exported plain player restored explicitly declared Wasm state into
fresh VMs and reproduced uninterrupted gameplay and PCM exactly. Four fresh
processes covered 40 uninterrupted ticks, a 13-tick prefix, a 27-tick continuation,
and a repeated continuation. Six additional processes rejected damaged or
incompatible checkpoints and existing output destinations. This is desktop
shipping evidence for the opted-in [host-owned persistent-state route](native-wasm-persistent-state.md),
not arbitrary Wasm memory capture.

The actual native producer was `b733a36623790b076b049763878c06dd5ab09450`, tree
`b96bead4d24bdff24029bb80474c19e1aca7fa9f`, based on main385. It used Rust 1.99.0,
WABT 1.0.42 and `aarch64-apple-darwin`. The native wrapper session49452 exited **1**
after 598.772 seconds: all native stages had passed, but final data replay raised
`KeyError('stderr')`. Seven shipping symbol commands intentionally merge stderr
into stdout; the verifier incorrectly required a separate stream for every call.

Verifier-only commit `350702062dbd2b84514dfbe1fe0ef1ec2811e215` accepts only the exact
seven-command symbol sequence and still validates its merged bytes/hashes.
Corrected replay passed against the original retained records, without rebuilding
or running the player again. Root independently replayed all 168 original files,
ran the 20 focused controls and rechecked process absence. The native exit-1
result, original source/tool postcheck and later data-only acceptance remain
separate; this report and its portable verifier are not new native producers.

The probe initializes a project, publicly imports its newly compiled SDK guest
twice and requires an unchanged-revision cache hit, then imports a deterministic
looping audio clip. One public `command.execute` transaction creates a mover and
a nonspatial audio entity and selects the scene program. Undo, Redo, save, graceful
RPC shutdown and a new journal reader verify the authored result. No project JSON
is written directly. The unchanged ordinary exporter then builds a fresh cooker
and shipping player, without a shared session, fixture feature or private module
selector.

| Executed stage | Evidence |
| --- | --- |
| Fresh normal headless | `ci`, `default,physics-deterministic`; no Wasmtime feature |
| Fresh ordinary player | `shipping`, `project,render-input,physics-deterministic,static-module` |
| Shipping audit | Actual Cargo codegen, normal dependency graph and symbol logs; no runtime authoring/JIT dependencies or source compiler symbols |
| Public runtime route | `--package`, `--save-checkpoint`, `--load-checkpoint`; fresh processes with restricted environment |
| Continuation | Exact final checkpoint bytes and uninterrupted PCM = prefix PCM + resumed PCM; repeated suffix identical |
| VM freshness | `fresh-guest` at the first callback of every process, including resumed absolute tick14 |
| Refusals | Checksum, program fingerprint, state schema, world/state clock mismatch, existing PCM and existing checkpoint |

The guest declares 24 host-owned bytes: tick count, movement count and score. Held
Right moves the entity by 1/16 per tick; Space press edges increment score. At
tick13 its position is `[0.3125,1,0.203125]`; at tick40 it is `[0.9375,3,0.625]`.
All controls are neutral at the split, and suffix events are rebased. An independent
arithmetic oracle checks every observed tick, final world transform, explicit
state values and v7 checkpoint framing/digest. Equality alone cannot satisfy it.
A nonpersistent VM counter supplies the freshness marker.

The looping clip runs at 48 kHz output with a 60 Hz scene: 800 frames per tick.
State-driven gain changes, seek, stop and restart occur across the split. The
baseline produces 256,000 bytes of finite stereo f32 PCM; prefix and suffix produce
83,200 and 172,800 bytes. The oracle requires audible output, exact silence during
ticks26–28 and audible transport after restart at tick29. It compares every PCM
byte, not only digests. This does not test fractional output-frame timing or device
audio.

Before export, owned guest sources/Wasm and guest build outputs were recorded and
deleted. Before the four proof runs, the authoring project, Wasm/audio caches,
private tool copies, debug maps and fresh exporter build storage were removed.
The immutable runtime package contains the player and seven cooked data files.
The engine checkout and separate normal-headless target remain identified; this
source-free controlled environment is not an OS filesystem sandbox.

The four incompatible checkpoints must fail at their specific admission boundary
without publishing PCM or a new checkpoint. Existing destination bytes and modes
must remain unchanged, with no abandoned staging files. PCM and checkpoint writes
are separate no-clobber transactions: checkpoint collision can leave a newly
published PCM file. The proof does not claim atomic publication across both files.

The player SHA-256 is
`21df50b0b213b0714e22169d03180b60290a22120dc7211f8108ceb04bdb14bb`
(9,883,664 bytes); the fresh headless is
`bd945d69fe55609dc7a50e67c62ec5894ff779991c8442ed60b7d5d27670b418`
(62,900,880 bytes). The original guest is
`ddfaeed957a0296f8dad05a845e5549baa431f0c41d511545509e8b48230c102`.
The [manifest](evidence/native-wasm-checkpoint-export.json) binds these external
artifacts, the 39 SDK/ABI inputs, full 22,220-entry producer inventory, tool
identities and original failures. Retained artifacts live under
`~/.codex/incant-resume-20261017/wasm-checkpoint-shipping-001-hmc3n16q/output`.

All 37 managed commands and two RPC owners closed. The append-only observed
inventory includes the wrapper and sampled descendants: 542 distinct PIDs and
39 managed groups were absent at post-terminal closeout and root's fresh check.
Polling cannot prove every short-lived descendant was observed. Trusted children
must retain managed groups; these are sampled process guards, not kernel quotas.

The fresh target's baseline was zero, with a 12 GiB allocated ceiling; scratch,
retained data and sampled RSS limits were 8, 2 and 8 GiB. The aggregate limit was
5,400 seconds, at two Cargo/Rayon jobs, with 28 GiB launch admission and a 6 GiB
continued free-space floor. Sampled peaks were target2,370,228,224 B,
scratch3,748,732,928 B, retained373,768,192 B and RSS7,510,245,376 B. The full failed
attempt remains charged. CPU2 was released; later target reuse is outside this
producer's measurements. Native binaries and debug bundles are not in Git.

Portable replay uses 210 raw UTF-8 records and base64 PCM/checkpoint records,
including source only as inert text: 4.13 MB compressed and 53.22 MB expanded,
bounded at 8/96 MiB. It rechecks recorded
provenance, public commands, builds, observations, exact PCM/checkpoints, refusals
and the complete retained cleanup inventory. It does not execute archived code,
rehash absent native payloads, re-observe historical deletion/process absence or
authenticate records against coordinated fabrication. The local full-artifact
replay additionally reads actual runtime file bytes/modes and checks owned source
and private-build absence. Those claims are explicitly separate.

From the repository root, data-only commands are:

```sh
PYTHONPATH=tools python3 -B -m probes.native_wasm_checkpoint.portable
python3 -B -m unittest discover -s tools/tests -p 'test_native_wasm_checkpoint*.py'
```

The portable controls include coherently rehashed missing/changed streams,
source/SDK/correction provenance, falsely relabeled native success, omitted
observed PIDs and changed recorded player identity. The existing 20 focused
controls cover independent arithmetic, state/schema/clock, PCM, VM freshness,
public transactions, refusal precision and merged symbol-stream admission.

There is no physics/UI in this scene, native-window/GPU execution, phone/device
measurement, performance claim, signing, arbitrary global/heap snapshot, full SDK
or phase-gate claim. Non-opted Wasm modules retain their explicit checkpoint
refusal; this proof uses the declared host-owned persistence contract.
