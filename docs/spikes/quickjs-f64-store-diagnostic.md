# QuickJS Float64 store: functional diagnostic

Source `831c89a8fedf977df737d78493b9e4a26396c27a` adds a diagnostic-only
numeric Float64 store branch to private copies of pinned rquickjs-sys 0.14.0.
The normal dependency graph, registry source, engine, authored stress callback,
VM limits and shipping gates remain unchanged. This is not an adopted interpreter
fork or a measured optimization.

The final original and candidate executables produce identical records for all
58 semantic controls (54 original cases plus four direct numeric resize cases).
Each runs the unchanged actual stress callback for 500 ticks, checking all 9,999
rows and finite sentinels after every tick, followed by three changed-view/layout
controls. Three independent omitted-write/NaN oracle negatives are refused. The
safe owned ArrayBuffer detach control also passes. The deliberately wrong store
fails the first numeric assertion, proving reachability of the candidate branch.
No intervals were collected: both final outputs have `ns: []`.

The final `execution-004` uses Rust 1.99.0, Apple clang, explicit
`aarch64-apple-darwin`, CI profile, offline locked dependencies, two build jobs
and Rayon threads. Each variant has a fresh target directory. The copied compiled
`quickjs.c` exactly matches its variant; the three executable hashes differ:

| Variant | SHA-256 |
| --- | --- |
| Original | `58bb741d89b3eb63827ec97420cd39b815dd011460091e29adfb1691a5a4a2f7` |
| Candidate | `0b75ca98f19acd8560af909a3771ba1794bbdfe4281875e03594d8d8751b28b6` |
| Deliberate wrong store | `ebb801ef13631ea59e8defe70c59bd530a8e5e8231f4dafd9e966454a7ed92c9` |

All three reconciled locks are identical, and every registry package identity is
already present in the unchanged root lock. The local sys copy and diagnostic
root are the only permitted package-identity exceptions. Full original/copied sys
source and license inventories, compiler logs, immediate executable copies,
prepared callback hashes and native object identities are retained.

Three unsuccessful generations are preserved without being counted as candidate
qualification:

1. `execution-001`: `metadata --no-deps` left the copied lock unreconciled. The
   identity checker refused it before compiling. Full offline metadata resolved
   it while retaining the pinned-identity check.
2. `execution-002`: `Value::new_float(7.0)` canonicalized to an integer tag. The
   Rust witness failed before JavaScript controls. It now uses `7.5`, retains the
   tag assertion, and prints original JavaScript failures.
3. `execution-003`: the common target reused the original executable for all
   variants. The deliberately wrong store unexpectedly passed, so the reachability
   assertion failed. Equal binary hashes and the 0.01-second reuse logs remain;
   these are original-only executions, not candidate evidence. Fresh targets and
   exact compiled-source checks corrected the preparation.

The [portable manifest](evidence/quickjs-f64-store/manifest.json) binds 4,147 records
and 463 deduplicated payloads. Executables/native objects are retained externally
by exact hash, avoiding unnecessary archive duplication. Replay without running
native code:

```sh
python3 docs/spikes/evidence/quickjs-f64-store/verify.py
python3 docs/spikes/evidence/quickjs-f64-store/verify.py --external --git-source
```

Both commands passed. External originals are under
`~/.codex/incant-evidence/quickjs-f64-store-001`; final executables are in
`execution-004/binaries`. Supervisor 71965 was terminal zero and reaped; all
managed children were cleaned before return, and the CPU lease was released.

These controls do not prove full JavaScript conformance, native borrowed-loan or
rollback safety, profitability, shipping performance, or the 2 ms Core gate.
Any future production fork needs separate upstream/security maintenance, target
and conformance review, and actual hardened-host integration qualification. A
matched standalone timing protocol is being prepared separately; it is unexecuted
and cannot approve an engine or shipping gate.
