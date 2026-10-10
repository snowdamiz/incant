# Character movement — in progress

A read-only `api.computeCharacterMotion` capability now computes collision-aware
movement for a primitive collider attached to a velocity-kinematic body. It uses
Rapier’s controller for wall sliding, slope limits, optional stair stepping,
ground detection and optional snap-down. The calling TypeScript behavior supplies
gravity/jumping and applies the returned displacement divided by the fixed `dt`
through an ordinary Velocity command. There is no separate mutation interface.

Queries exclude the character and sensors, honor its authored collision masks,
and return stable collision IDs. Scene-root/unit-scale physics restrictions still
apply; rotating character bodies are rejected. The world’s up direction is +Y.
No mesh terrain, animated character, ragdoll or native play mode is claimed.
Moving-platform behavior and representative game performance still need review.

An unsimulated author edit uses a separate collider/BVH snapshot, preserving the
live solver’s modification flags. Stepped worlds borrow their existing query
pipeline. Each movement query costs 16 units in the shared 256-unit physics-query
budget. Native query return and script commit both check the script deadline;
catching a query error cannot commit an expired tick’s state or logs.

Five physics cases pass: wall slide/ground/read-only behavior; low-step versus
tall-obstacle traversal; slope limits and snap-down; mask/sensor/immediate edits;
and invalid queries without consuming live solver changes. Two additional
script integration cases pass for movement through commands, hot reload, author
isolation and the shared query budget. An injected native-query deadline test
verifies that an expired tick commits neither state nor logs; it is not a live
provider or device gate. Full integration, public CLI, performance, visual review
and hosted checks remain open. Claude handoff 0023 owns actual rendered motion.
