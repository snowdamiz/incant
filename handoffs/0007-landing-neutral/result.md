# Result: 0007 landing page, revision 4 (density and pacing pass)

Dated 2026-10-08. Earlier results are kept unchanged as history:

- **`result-1-palette.md`:** the neutral palette work, `6090702` and `c5999d3`.
- **`result-2-expansion.md`:** the modeling and workflow expansion, `317cf6b`. It is integrated
  on the landing branch as `11abac8`.

## Status

The design pass is implemented, type-checked, built and committed with `Built-by: claude` for
Astra to integrate into PR #2. Nothing was pushed, published, deployed or merged, and no phase
gate is claimed or approved.

**This revision has not been seen rendered.** The screen-capture ban was in force throughout.
No computer use, browser, Playwright run, native automation or capture of any kind was used.
Every judgment below comes from source review, layout arithmetic and a text-only render in
Node.

## Model

Claude Opus 5.5 (`claude-opus-5-5`), running in Claude Code.

## Director feedback acknowledged

The director said: "some of the new sections are way to text heavy and close together overall,
not good enough". That is correct. I diagnosed four causes in revision 3:

1. **Six prose paragraphs under the modeling stage:** the paragraphs averaged 35 words and each
   had its own heading, which made the section read like documentation.
2. **A workflow that retold the stage:** each of its six steps had a numeral, a heading, a
   sentence and an artifact. A separate sub-heading and paragraph then introduced the export
   targets.
3. **The same pattern three times running:** modeling, workflow and principles all used a
   heading, then a list of numbered or titled prose items, so the page rhythm went flat.
4. **Tight spacing:** each heading had its description beside it in the same row, and the gaps
   between groups were 56 to 80 px, which crowded each section's opening.

The finished-product voice, the neutral palette, the restrained accent, the wisp and the
connected-panel chrome are all kept.

## What changed

### Copy density

The counts below are visible words and headings per section, from the same text-only render
before and after this pass.

| Section | Words before | Words after | Headings before | Headings after |
|---|---|---|---|---|
| Modeling, including about 60 words of editor UI | 324 | 148 | 7 | 1 |
| Workflow | 193 | 57 | 8 | 1 |
| Principles | 91 | 54 | 5 | 4 |
| FAQ, all answers counted though most start collapsed | 306 | 213 | 8 | 8 |
| Hero, including the agent stage UI | 160 | 147 | 1 | 1 |

- **Modeling:** one sentence now follows the heading. The six prose groups became a two-column
  index with no headings. Each entry is a term with a gloss of 4 to 6 words, such as "Mesh tools:
  Retopology, UVs, LODs, baking, light sculpt". One display line leads into the index: "From
  first primitive to a game-ready asset."
- **Workflow:** each step is now just a verb in large display type and the artifact it leaves,
  such as **Model** `lighthouse.geo` or **Review** `4 edits · 1 transaction`. The step sentences,
  numerals and the export sub-heading are gone. "Ship" leads straight into the large platform
  row, which has one caption line.
- **Principles:** three ideas of one line each, in a single row. "Yours to run" was removed
  because the statement band already makes that point with "Works offline" and "Credentials stay
  local".
- **Hero and FAQ:** the hero paragraph is two short sentences. Every FAQ answer is one or two
  sentences.

### Hierarchy

- **One idea per section opening:** the modeling and workflow headings are larger, rising from
  4.4 to 5.2 rem on desktop. Each now stands alone with a single short line under it, instead
  of sharing a row with a paragraph.
- **Illustrations carry the explanation:** in modeling, the interactive graph stage is the
  centre of the section, and the index is set clearly smaller beneath it. In workflow, the track
  and the platform row are the content.
- **Fewer competing headings:** 13 headings were removed from the two new sections. The index
  uses a definition list and the workflow steps are list items, so screen-reader heading
  navigation now lands only on real section starts.
- **Pattern variety:** the three middle sections now look distinct. Modeling has an
  illustration and an index, workflow has a track and large type, and principles is a short
  three-column row.

### Spacing and transitions

| Gap | Before | After |
|---|---|---|
| Section padding, modeling and workflow | 96–128 px | 112–192 px, by breakpoint |
| Heading to modeling stage | up to 80 px | 96 px from `sm` |
| Stage to index | 96 px | 128 px |
| Heading to workflow track | 80 px | 112 px |
| Track to platform row | 96 px | 128 px |
| Principles padding | 96–128 px | 112–160 px |

- **Transitions:** the workflow band and the principles section no longer have hairline rules
  at their tops. The change between paper and deep paper marks those transitions on its own.
  Modeling keeps its single rule below the hero.

### Mobile

- **Graph stage:** below 768 px, the seven graph nodes are a horizontal strip of node names
  under the viewport. The parameters, the panel label and the agent prompt line are hidden at
  that size. Before, the stage stacked into a column about 350 px tall below the viewport.
- **Workflow and index:** the workflow keeps its vertical rail, now with only a verb and an
  artifact per stop. The tool index stacks to one column.

### Accessibility and behaviour

- **Graph nodes:** these are still toggle buttons with a pressed state. The live announcement
  and the changing preview description are unchanged. The scrollable strip on phones contains
  only focusable buttons.
- **Agent walkthrough:** the tab list, its keyboard handling and its screen-reader descriptions
  are untouched.
- **Contrast:** no new colour pairs were added. Every text colour reuses pairs computed in the
  previous revision.

## Changed paths

**Code:**
- `website/src/content.ts`
- `website/src/components/ModelingSection.vue`
- `website/src/components/ModelingStage.vue`
- `website/src/components/WorkflowSection.vue`
- `website/src/components/PrinciplesSection.vue`
- `website/src/components/HeroSection.vue`
- `website/src/components/StatementBand.vue`

**Handoff files:** `result.md`, the new `result-2-expansion.md`, and Astra's updated `brief.md`,
which is committed as received. The tests are unchanged, because no new interaction was added.

## Commands and results

All commands ran from `website/` on 2026-10-08.

| Command | Result |
|---|---|
| `npm run typecheck` | Pass. Strict `vue-tsc` includes `tests/`. |
| `npm run build` | Pass |
| `PAGES_BASE_PATH=/incant/ npm run build` | Pass. Script, stylesheet and favicon URLs are prefixed with `/incant/`. |
| Structural check | Pass. The page was built with Vite's SSR mode into `/tmp`, rendered with Vue's server renderer and parsed as text. No browser was involved, and the temporary entry file was deleted. |

The structural check found the following:

- There is one h1, and heading levels never skip.
- There are no duplicate ids, and every in-page anchor resolves.
- The only external links are the repository and PLAN.md.
- None of these words appear: planned, future, early development, not yet, coming soon, roadmap,
  private, may require access, keychain, beta.
- There is one tab list, with 4 tabs and 1 tab panel. The graph has 7 pressed-state buttons.

**Payload:** JS is 110.8 kB raw and 40.4 kB gzip, against a 100 kB gzip budget. CSS is 8.1 kB
gzip, and `dist/` totals 412 kB.

**Browser tests were not run,** under the restriction. Astra's hosted CI will run them.

- **Unchanged:** all 23 tests are as they were, including the graph-node keyboard test from the
  previous revision.
- **Expected to pass:** the selectors they rely on are all still present. These are the tab ids,
  `#create` buttons with a pressed state, the closed offline FAQ item, and a FAQ link in the
  mobile menu.

## Screenshots

**None were taken.** Visual verification is deferred at the director's request. The images
under `screenshots/` show the palette revision only. A CI run of `landing.spec.ts` writes fresh
renders to `test-results/visual/`.

## Deferred visual checks

1. **Overall rhythm:** check the page top to bottom at 1440 and 390 px. In particular, check that
   the new space opens up the two sections without making the page feel thin.
2. **Workflow track at 1024 to 1280 px:** at about 1024 px, the artifact `4 edits · 1 transaction`
   is expected to wrap to two lines. Check that it still reads well.
3. **Platform row:** at 4.2 rem on desktop, the six names are expected to wrap onto two lines.
   Check where the wrap falls and whether a slash ends a line.
4. **Graph strip on phones:** check the horizontal scroll affordance. On mount, the strip alone
   scrolls to its end, so the default Output node starts in view. Check that this happens
   without moving the page.
5. **Lighthouse geometry:** the previous revision's open item still stands. The model was placed
   by computed coordinates and has not been observed.
6. **Test suite:** run the full browser suite, including axe at 390 and 1440 px and overflow at
   four widths.

## Promise-to-page map

| Page claim | PLAN.md source |
|---|---|
| 3D and 2D modeling tool, game engine and editor in one; agent works through the same commands | 1.1; 1.2 #1 |
| Geometry graphs: booleans, bevels, lofts, arrays, scatter; graphs are text the agent can write | Phase 4; App. B `GeometryGraph` |
| Mesh tools: retopology, UVs, LODs, baking, light sculpt | Phase 4 |
| Materials: shader graphs compiled to WGSL | Phase 4 |
| Terrain: heightmaps, layered materials, foliage | Phase 4 |
| Animation and VFX: retargeting, IK, timelines, particles | Phase 4 |
| Generated assets: image-to-3D, texture sets, motion | Phase 4; 6.6 |
| Graph stage steps: primitive, taper, extrude, array, boolean, scatter with noise, then retopology, UV and LOD output | Phase 4 |
| Workflow: model, surface, TypeScript script, review as one transaction, headless play-test, ship | 1.1; 2.4; Phase 1; Phase 3; App. A `play.run` |
| Six platforms; native Xcode and Gradle projects; Steam, crash reporting, dedicated servers | 1.1; Phase 5; Phase 6; 6.7 |
| One undoable timeline shared by user, scripts, team and agent | 1.2 #1 and #4; 2.3 provenance |
| Scenes, materials and graphs as documents you can diff, review and merge | 1.2 #2; 2.3 |
| Agent plays the game and shows proof | 1.2 #3; Phase 3 tools |
| Agent acts only as far as you allow | Phase 3 approval modes |
| OpenAI, Anthropic, Google or local model, per task; cost shown per turn | 3.1; 3.4; Phase 3 |
| Credentials stay on your machine | 3.1, as amended by the director: owner-only local files on macOS and native stores elsewhere |
| Offline editor, no account; agent needs a provider or local model | 3.3 |
| Concurrent edits merge; account adds cloud sync and invitations | 2.3 CRDT; 3.2 |
| Detailed sculpting stays in dedicated tools; glTF and FBX import | Phase 4; 9 out-of-scope list; Phase 1 |
| 2D and 3D games in TypeScript, templates, online multiplayer and cross-play | Phase 1; Phase 2 templates; Phase 6; 1.3 |
| No shell access for the agent | 3.5 |

## Limitations

- **Unverified visually:** no render of this revision has been seen. See the deferred checks
  above.
- **Finished-product voice:** the copy presents the full plan as shipped, as the director
  instructed. The repository remains in Phase 0, and this page approves no phase gate.
- **Credential wording:** this follows the director's update. The worktree's PLAN.md copy still
  says keychain, and I didn't change it.
- **Inherited limitations:** the tablet crop of the hero scene and the lack of Firefox, Safari
  and real-device testing carry over from earlier revisions.
