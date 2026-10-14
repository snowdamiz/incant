# Immutable bulk-query row labels

The script host caches each chunk's frozen array of stable row IDs while its
complete ordered ID sequence remains equal. This avoids converting every ULID
to a string and rebuilding those arrays every tick. Numeric component views,
their backing pointers, transaction undo data and tick capabilities are never
cached. Their existing creation, detachment and rollback paths remain intact.

The comparison uses all ordered IDs rather than a hash or table index. Despawn,
swap removal and archetype changes therefore cannot leave an ID associated with
the wrong numeric row. Cache entries beyond the current chunk count are dropped.
A replacement is published only after its array is fully built and frozen; old
guest-retained arrays remain unchanged. Array elements are own properties, so
inherited index setters cannot alter construction. The captured host freeze
function is used, not a guest replacement. Persistent roots drop before the VM.

Two new observable controls check identity reuse with fresh detached views and
garbage collection, immutable labels, structural movement, ID reuse, retained
history and correct row-to-transform writes. The entire script library and
project-binding integration suite pass in both deterministic and fast physics
modes: **138 + 12 cases per mode**, including existing hostile metadata exposure,
allocation failure, deadline, rollback and capability revocation tests. Strict
all-target Clippy passes in both modes. All six commands use the pinned wrapper,
offline locked resolution, native aarch64 target, CI profile and two build jobs.

The source is based on `bff83af6`, the frozen Mac shipping baseline. Its four
changed source paths were frozen before qualification and checked before and
after each command. Initial unqualified `rustfmt` lookup failed; the pinned
1.99.0 formatter then passed. Commands, logs and immediate executable identities
are retained in the [portable packet](evidence/script-chunk-id-cache/manifest.json):

```sh
python3 docs/spikes/evidence/script-chunk-id-cache/verify.py
python3 docs/spikes/evidence/script-chunk-id-cache/verify.py --external
```

These are CPU correctness checks, not a full workspace test or a measured
shipping improvement. The workload, VM limits and performance thresholds are
unchanged. Current-main composition and matched 10,000-script shipping
measurements remain separate work.
