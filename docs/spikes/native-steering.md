# Native local avoidance

Native gameplay can call `navigation.steerAgents` with one bounded group of
world-space agent samples and optional directed obstacle geometry. The result is
an owned array of velocity proposals sorted by stable ID. Gameplay applies them
through ordinary native movement commands or physics. Proposals are not a
physical separation guarantee, and no world state is changed by the query.

The existing reciprocal-avoidance solver now accepts either canonical authoring
ULID strings or fixed 16-byte native IDs. Its numerical algorithm is unchanged.
Native solving does not parse or allocate formatted IDs; the guest boundary
converts IDs when reading requests and publishing owned results. Existing
authoring schemas, ordering and the deterministic singular-overlap tie break
remain unchanged. This does not make the whole solver allocation-free: its
bounded working vectors and backend allocations still exist.

Positions are world-space feet with +Y up; velocity and obstacle vertices use
XZ. Input IDs correlate samples and need not identify entities in the scene.
Callers gather their own samples, convert parented local transforms, and supply
obstacles; this API does not extract colliders or move characters automatically.
Horizons use the current accepted script update's time step. The TypeScript
template at `sdk/templates/native-navigation/steering.ts` demonstrates this bulk
query surface.

## Admission and lifetime

- At most 128 requested agents and 128 requested obstacle edges across an entire
  update, shared across calls and authored sources. Each request permits up to
  32 obstacles and 32 vertices per closed convex polygon; open obstacles have two
  vertices. Empty-agent requests still consume their obstacle-work budget.
- Steering shares the existing 32 navigation calls and 1,024 general query-cost
  budget. Requests charge before native execution. A recursive guest getter
  cannot reset a limit or hide an already-failed capability.
- Fixed-field conversion bounds arrays before allocating them, accepts fixed
  numeric tuples or `Float64Array`, and checks original finite doubles before
  narrowing to the solver's floats. Native validation still checks distinct IDs,
  convex/simple polygons, edge lengths, vertical intervals and solver limits.
- No native loan spans guest getters. Results use owned arrays and null-prototype
  records, with own data properties bypassing guest prototype setters. Retained
  results survive updates; retained callbacks cannot reactivate in later ticks.
- A caught invalid request, quota failure or result-allocation failure fails the
  update and rolls back numeric writes and queued structural commands together.
  The existing deadline checks bracket native execution; QuickJS cannot interrupt
  inside the bounded solver itself.

No project document, command bus, JSON conversion or authoring validation enters
the native tick. The new dependency edge is only `incant_nav` to `incant_types`;
dependency versions and resolutions are unchanged.

## Evidence and remaining work

The exact source manifest and check receipts are recorded in
[`evidence/native-steering.json`](evidence/native-steering.json). Tests cover
native/string numerical parity, unchanged schemas, lease expiry including unwind,
actual native movement, directed obstacles and vertical separation, hostile guest
accessors, owned results, shared source budgets and atomic failures. The cook test
uses strict authored TypeScript and the shipped SDK template, then removes the
source directory before executing the compiled behavior.

This scope does not add the Wasm steering ABI, automatic collider-to-obstacle
collection, navigation debug drawing, a Core Sample, or phone performance
evidence. Source deletion in an in-process cooked-session test is not a fresh
shipping-player export, a platform test or a Phase 1 gate. Those requirements
remain open.
