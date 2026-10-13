# Exact private island construction

The default-off checked constructor selects the rigid witness from an admitted
NumericFrame and constructs IslandManager privately. Independently supplied
identity/dimension must match. It preserves active/persistent VecMap slot tails,
free-stack order, body/contact order, epochs, pending split/cooldown counters,
sleeping flags and every historical sleep-scan slot. Unsupported joints, pending
removal/scans and bootstrap state remain refused by the checked witnesses.

Every persistent allocation is charged to the same cumulative non-Clone permit
before allocation. Ordinary vectors use fallible exact reserves. Pinned VecMap
provides only infallible reserve APIs, so its precharged with_capacity request
retains ordinary process-OOM behavior. Safe temporary insertion/removal establishes
the exact empty slot extent without another allocation; an explicit control
checks live count, vacant tails, capacity and subsequent reuse. This is logical
requested storage, not allocator overhead or RSS. Failure drops staged values.

Scratch reset is based on native reads/overwrites, not serde(skip):
`substep_groups::update_substep_groups` clears solve groups, resets union-find and
clears/repopulates key/scatter buffers. `local_split::search` increments its stamp,
clears visited entries on wrap, clears both frontier/reached lists and seeds new
visits before matching; links are cleared before collection. Global
`split_island_now` resets union-find/counts, advances its stamp (clearing on wrap),
writes every member before lookup, and clears node/link/root buffers. Sleep-scan
stamps lack an equivalent omission proof and are restored exactly.

A shared actual-native fixture is stepped independently by the existing joined
frame observer and the private constructor tests. Four completed frames per
dimension encode/decode through the existing bounded whole-frame codec, then
replace only IslandManager. Other native owners are explicitly trusted typed
clones. Focused controls compare serialized owner bytes, exact sleep scans and
pipeline observations for 156 paired steps per dimension across sleep/wake,
removal/reuse, body type, disable/enable and extra-iteration changes. Short
allocation budgets, wrong identities/dimensions and truncated frames refuse. A
zeroed sleep-scan negative control differs in observed state; it is not a claim
of trajectory divergence.

The first fixture attempted capture on every mutation tick and correctly hit the
existing PendingMutation refusal. It is retained; selected checkpoints 0/16/32/48
pass without weakening admission. A later test-only Result callback compile
failure is retained separately. Six focused native tests pass, and source review
`56edc9aef6b0961d35385c4f0dd78effb2f4c151eb5c0e506bbc39f4cb75c9d9`
is accepted. The full deterministic snapshot suite also passes 218 tests (26 groups,
24 nonempty; 0 failures/ignored). An explicit user pause stopped the orchestrator
at that completed check boundary before fast8. Fast8, full private checked
suites, strict Clippy and remaining scoped guards are pending, as is final
evidence review. The [pause index](evidence/physics-native-islands-pause.json) records retained raw
evidence identities and the intentional process stop; no new large archive was
created for the pause. [Resume notes](physics-native-islands-paused.md) list the
remaining checks. This is an incomplete qualification checkpoint.

Exact broad/narrow/CCD managers, complete global arithmetic/work admission,
generation fencing, authenticated capture and stable-ID/event composition remain
required. There is no whole-world record-to-live installer, save admission, GPU,
device or performance claim.
