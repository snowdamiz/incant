# Hierarchy keyboard focus after entity removal

Scope: editor DOM keyboard correctness after Delete, another hierarchy edit or
Undo removes the focused row. This is separate from the command-bus deletion and
duplication implementation and from native interaction qualification.

## Reproduction and behavior

Before the repair, two new DOM controls (Delete and removing the last row) and an
extended duplicate/Undo control failed because `document.activeElement` became
the body. Arrow keys then stopped reaching the hierarchy. The editor already
correctly dropped a selection whose entity no longer existed.

The row's stable React ref cleanup now records removal only if that DOM row still
owns focus immediately before detach. After the snapshot commits, the hierarchy
recovers to the next surviving visible row, or the preceding row if no later row
survives. A removed subtree is skipped. The replacement becomes the roving Tab
stop and is pinned and revealed using the existing virtualization path. Arrows
and subsequent Delete continue from that row.

Recovery does not select an entity or advance selection intent. The existing
Duplicate request/result and selection-generation fences are unchanged. The
repair also refuses recovery after a bridge/project change, a newer explicit
selection, an intentional blur to the body, filtering that only hides the entity,
or focus moving to the filter, inspector, dialog or Assets tab. It copies no old
row arrays: the adjacency search stops at the first surviving visible row.

Root review of the first local commit `f98d2af5` identified an uncovered Enter
route: reselecting the focused row advanced selection intent without a new DOM
focus event, so a later removal rejected recovery. An added Enter/Delete control
reproduced that failure while the other 14 focus controls passed. Enter now uses
the same movement handler as pointer/arrow activation, which refreshes owned
focus generation when reselecting an already focused row. Newer external
selection intent still vetoes recovery.

## Local evidence

Runtime: Node 22.23.3 at
`/Users/andreyyurlov/.local/share/fnm/node-versions/v22.23.3/installation/bin/node`.
Run from `editor/ui`, using that binary in place of `node`:

```sh
node ../../node_modules/vitest/vitest.mjs run src/components/HierarchyFocus.test.tsx src/components/HierarchyDuplicate.test.tsx --maxWorkers=2
node ../../node_modules/typescript/bin/tsc -b --noEmit
node ../../node_modules/vitest/vitest.mjs run --maxWorkers=2
node ../../node_modules/typescript/bin/tsc -b
node ../../node_modules/vite/bin/vite.js build
```

The focused run passed **52 tests**. The full UI/bridge run passed **1,040 tests
in 50 files**. TypeScript checking and the production UI build passed. The
integrated source includes main344 `af7aa33a4b37f0136e2267edd3c35d21a7bb4573`;
the full UI suite and TypeScript check were repeated after that integration,
the allocation-free adjacency scan and the Enter-route correction.
`git diff --check` passes.

The 15 new focus controls include pointer/Enter reselection of the focused row,
successive deletion of an unselected recovery row, ancestor/subtree removal,
intentional focus escape, same-label bridge replacement, project replacement,
newer Problems selection and a replacement initially outside the mounted window.
Two existing duplicate controls now answer Undo and prove subsequent arrow
navigation for a leaf copy and an expanded copied subtree. Existing late duplicate
outcome and newer-selection controls pass unchanged.

jsdom emits its existing optional canvas-context diagnostic; all assertions pass.
These tests use explicit snapshot/command doubles and mocked geometry. They prove
observable DOM focus and command intent, not a real native command transaction,
browser layout, target timing or performance. No native build, app window,
browser window, screenshot or CUA interaction was launched for this repair: the
Mac desktop remains locked. Native Delete/Undo keyboard interaction remains
pending an unlocked desktop. No phase or release gate is approved.
