# Core stress cursor loop

The sample keeps the same immutable row metadata, actual coordinate writes and
actual readback checksums. The loop advances a cursor through each four-number
metadata tuple and computes the per-chunk count once. It still reads fresh chunk
views and stride on every tick, and invalidates metadata when attachment membership
or row identity changes. Gameplay and the controller are unchanged.

Source commit: `0978f65418e24f897e2abad726129e3f97951222`.
Independent source review: `e6aea675bfaf6800e7a79a31441023eec11f7c9c88f18ac83f69d375762633ad`.

Pinned Node 22.23.3 strict TypeScript and stress controls passed, including all 9,999
owned rows at four phases, new/detached views, changed stride, reordered rows,
attachment remapping, five refused memberships and three independent omitted-write
mutations. Existing public-authoring helper tests and diff checks also passed.
The portable packet retains exact sources, tool identities, argv and complete logs:

```sh
python3 docs/spikes/evidence/core-cursor-loop/verify.py
# On the producer host, also verify the retained originals and executable tools:
python3 docs/spikes/evidence/core-cursor-loop/verify.py --originals --tools
```

The earlier standalone QuickJS diagnostic motivated this change; it is not native
shipping evidence. This source has not been measured in a shipping package. The
previous Core 10k result still fails the unchanged 2 ms script p95 gate. Applying
this sample to authored project content must use the public command flow.
