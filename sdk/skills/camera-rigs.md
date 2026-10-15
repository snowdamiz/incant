# Camera rigs

## Workflow

1. Query every authored Camera and its Transform, selected scene and intended target. Define one active camera/rig owner and explicit stable target IDs. Preserve projection and near/far constraints from the schema.
2. Choose a concrete rig: follow offset, orbit or first-person. Use fixed-dt smoothing with a defined reset/snap condition; interpolate positions and normalized quaternions, not arbitrary Euler components. The renderer looks along camera local -Z.
3. Read target positions from current bulk columns, then write the owned camera transform. Distinguish local from world transforms under parenting. Use a native physics query for obstruction only when the host exposes it; retain a minimum clearance and a predictable recovery policy.
4. Camera switches, teleports, scene changes and checkpoint restores must reset temporal/smoothing state explicitly. UI navigation must not also drive the gameplay rig.

## Verification

Record stationary targets, movement, rotation, obstruction, teleport, parent transforms and missing/deleted targets. Assert finite normalized transforms, expected offsets and projection stability. Review motion and clipping on a real render; a static screenshot cannot prove smooth camera motion.
