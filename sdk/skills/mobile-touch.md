# Mobile touch controls

## Workflow

1. Query the actual host input capabilities and authored action map. Design move/look/action zones with explicit pointer ownership and a keyboard/gamepad fallback. Do not fabricate a touch device or enable network services to test input.
2. Assign each pointer to one control until release/cancel. Normalize movement with dead zones, clamp sticks and distinguish absolute position from deltas. Handle gesture arbitration so a UI press cannot simultaneously move or rotate the player.
3. Keep control geometry tied to the current logical viewport and safe area when the host supplies it. Handle resize, rotation, focus loss and pointer cancellation by clearing held actions. Keep touch identifiers transient; never restore device handles from saved gameplay.
4. Submit gameplay action state through the supported native input mapping. If custom on-screen action injection is unavailable, report the missing capability rather than mutating runtime input snapshots or claiming controls work.

## Verification

Replay multi-pointer press/move/release/cancel, fingers crossing zones, resize and focus loss. Assert no stuck movement and one owner per pointer. Desktop simulation validates logic only; actual mobile-device feel and platform keyboard/haptics remain later device review, not a development gate on this Mac.
