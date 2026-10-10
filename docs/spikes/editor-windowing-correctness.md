# Editor windowing correctness follow-up

2026-10-10, Astra. Nonvisual correctness review of handoff
`0033-editor-design-review`, based on Claude's `b7ba9bb`.

## Reproduced failures

The new regression cases were run against the original implementation before
changing the helpers. All six selected cases failed:

- A viewport beyond a collection's new end returned an unclamped last index.
  Counts of 0, 1, 10 and 1,000 reproduced it. Narrowing a scrolled hierarchy
  filter then crashed at `row.id` because mounting included nonexistent rows.
- A hierarchy filter with zero matches removed its scroller but left the hook
  subscribed to the old DOM node. Clearing the filter retained the old viewport
  and failed to mount the beginning of the replacement tree.

Command: `npm test -- src/components/Windowing.test.tsx -t 'shrinks|narrow a filter|replacement scroller'`
from `editor/ui`.

## Correction and caller audit

`rowRange` clamps the first endpoint before computing the nondecreasing last
endpoint. Both remain within the current collection, including an empty one,
while the browser adjusts its scroll offset after content changes. Valid pinned
rows remain mounted.

`useViewport` returns a stable React callback ref. Attaching it measures and
subscribes to the actual node; detaching removes its scroll listener, disconnects
its resize observer, cancels pending animation frames and clears the imperative
ref. Its identity changes only with the imperative ref or measurement settings.
Ordinary renders and scroll updates therefore do not resubscribe observers.
Measurements retain the existing scroll-step quantization and frame coalescing.

All five callers attach that callback:

- Hierarchy tree, object lists and string lists can remove/recreate their
  scroller while the owning component remains mounted.
- Asset lists and numeric tables unmount their owning component on the relevant
  empty-filter or hide-values transition; their recreated nodes are also covered.

## Validation

From `editor/ui`:

- `npm run typecheck`: passed.
- `npm test -- src/components/Windowing.test.tsx src/shell/windowing.test.tsx`:
  22 tests passed in 2 files.
- `npm test`: 441 tests passed in 18 files.
- `npm run build`: passed (TypeScript and Vite production build).
- `git diff --check`: passed from the repository root.

Regression coverage includes scrolled filter shrink, empty-filter recovery,
object/string lists becoming empty and smaller, asset-filter recovery, numeric
table hide/show, initially absent scrollers, changed measurement settings and
React Strict Mode cleanup. Lifecycle tests verify that stable-node rerenders do
not create new subscriptions, detached listeners stop scheduling work, pending
frames are canceled and replacement nodes respond to scroll and resize events.
The existing 100,000-entity, 20,000-asset and 65,536-value fixtures retain bounded
mounted-row/cell assertions and keyboard/rename behavior.

These are jsdom behavior checks with explicit browser notification doubles.
The suite reports jsdom's existing missing canvas `getContext` implementation;
it does not verify rendered pixels, native browser timing or device performance.
No visual assets, styling, screenshots or phase-gate evidence changed.
