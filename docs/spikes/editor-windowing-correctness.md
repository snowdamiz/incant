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

## Whole-app integration follow-up

Astra reviewed Claude's whole-app commit `1608e88` and reproduced additional
keyboard and navigation failures before patching them:

- Hovering a destructive menu action while another item held DOM focus could
  activate the wrong item. Pointer selection now synchronizes roving focus, and
  keyboard activation uses the focused item.
- Diagnostic links could not reach unmounted compound parts, string-list items
  or numeric grid cells, or open a collapsed component. An Inspector-local reveal
  registry commits ancestor expansion, list selection and scroll before resolving
  the exact field. Registrations clean up on unmount and use current callbacks.
- Field IDs aliased distinct component/key paths. IDs now encode every unsafe
  UTF-16 code unit and reserve a terminal namespace; JSON Pointer decoding preserves
  empty keys and literal percent characters and rejects malformed escapes.
- Shrinking a paginated collection clamped only its rendered index; later growth
  restored an obsolete page. The clamped page is now committed. Activating a
  disappearing New/Now anchor transfers focus to the stable final-page control.

These changes preserve Claude's layout and styling. The native window title and
bridge's unavailable-agent explanation also remove obsolete Phase 0 wording, as
requested in Claude's integration notes; agent availability is unchanged.

The final integrated UI passes **492 tests in 27 files**, TypeScript checking and
the production Vite build. Original failing cases and patch-review records are
retained locally in the 2026-10-10 UI-correctness packet. Native recheck and
Claude's rendered acceptance follow the rebuilt app; unit tests do not count as
that evidence. Full bridge payloads remain unbounded; paging currently bounds
mounted UI content and preserves all records, not transport cost.

## Main-based whole-app integration review (2026-10-10)

The full editor design is now based on main PR #49 independently of the pending
pixel-perfect renderer. Claude Opus 5.5 at explicitly verified Max corrected the
Camera fixture/native parity and retained the whole-app design and bounded lists.
Its browser evidence covers 96 states; native acceptance still requires the
rebuilt app and Claude review.

A separate Astra correctness review reproduced two additional state-transition
bugs. The agent transcript now attaches its overflow observer to the actual DOM
mount, so an unavailable-to-ready transition and a later remount keep overflowing
content keyboard-focusable and clean up the old observer. A collapsed navigation
grid now reveals the exact cost values if updated dimensions become invalid;
returning to a valid grid preserves the earlier collapse choice. The regression
tests failed before their fixes and cover resize/cleanup and diagnostic retention.
These changes preserve Claude's appearance and capability decisions.

The integrated main-based source passes all 500 UI tests in 29 files, strict
TypeScript checking and the production build. Bridge generation and repository
conventions match, and all 25 Python tool tests pass. UI logs:
`/tmp/incant-ui-main-final-{typecheck,tests,build}.log`. Native build and functional
verification follow this source freeze; no native acceptance or phase/device gate
is claimed by these browser tests.
