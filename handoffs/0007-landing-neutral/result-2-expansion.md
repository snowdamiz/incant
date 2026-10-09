# Result: 0007 landing page, revision 3 (expanded around the full plan)

Dated 2026-10-08. The earlier palette result is kept unchanged as history in
`result-1-palette.md`. Its design commit is `6090702`, and its capture-stop note is `c5999d3`.

## Status

The expanded page is implemented, type-checked, built and committed with `Built-by: claude`.
Astra can integrate it into PR #2. Nothing was pushed, published, deployed or merged, and no
phase gate is claimed or approved.

**This revision has not been seen rendered by anyone.** The director's screen-capture ban
was in force for the whole revision. Every visual judgment below comes from source review,
geometry worked out by hand, contrast arithmetic and a structural render to HTML text.

## Model

Claude Opus 5.5 (`claude-opus-5-5`), running in Claude Code. No other model and no
image-generation service was used. All new artwork is hand-written SVG and CSS.

## Director feedback acknowledged

1. **"Add more to the landing page… based off of what the plan document promises… missing the
   3d modeling capabilities":** applied. The page has a new modeling section with an interactive
   geometry-graph illustration, a six-group content-tools overview and an end-to-end workflow
   with the six export targets.
2. **"Design it as if it all already shipped… labeling as future is useless":** applied. Public
   copy is in the present tense, as a finished product. An interrupted earlier pass had added
   "Can I use it today? Not yet" and "planned" wording, and all of it is gone. There is no
   current-versus-roadmap section. A text search of the rendered HTML found none of these
   words: planned, future, early development, not yet, coming soon, roadmap, beta.
3. **The repository is public:** every "may require access" and "private" note is removed from
   the visible and screen-reader copy.
4. **Credential storage changed:** the copy now says credentials stay on the user's machine. The
   keychain claim is gone. PLAN.md and the auth code were not touched.
5. **Screen-capture ban:** obeyed throughout. No computer use, no browser or native automation,
   no Playwright run and no screenshots or capture of any kind were used, headless included.

## What changed on the page

The page order is now:

1. Hero
2. **Modeling** (new)
3. **Workflow** (new)
4. Principles
5. Statement band
6. FAQ
7. Closing

The navigation reads How it works, Modeling, Workflow and FAQ.

- **Hero:** the paragraph now says Incant is a 3D and 2D modeling tool, game engine and editor
  in one. The headline, interactive agent stage and calls to action are unchanged. The caption
  reads "Product illustration." The stage's screen-reader descriptions still say "not a
  screenshot", which keeps the image description truthful.
- **Modeling section:**
  - **Heading:** "Shape it where you ship it."
  - **Graph editor:** a second editor window shares its title bar with the hero stage, so both
    read as one product. Its node list holds seven steps:
    1. Cylinder
    2. Taper
    3. Extrude
    4. Array
    5. Boolean
    6. Scatter
    7. Output
  - **Clay viewport:** selecting a node shows the model at that step of the graph. Each node's
    geometry is outlined in the editor accent while it is selected.
  - **Output step:** the last node shows the cleaned-up topology as a wireframe, with an "LOD 0
    of 3" label.
  - **Toolset:** below the window, six groups are set as editorial prose under hairline rules,
    not cards. They cover geometry graphs, mesh tools, materials, terrain and light, animation
    and effects, and generated assets.
- **Workflow section:** six steps run on one hairline track, horizontal on desktop and vertical
  on mobile. The steps are Model, Surface, Script, Review, Play-test and Ship. Each step ends
  with its concrete artefact, such as `lighthouse.geo`, `lamp.ts` or "4 edits · 1 transaction".
  A large typographic row of the six export targets follows.
- **Principles:** collaboration and mergeable documents are now part of the existing four
  principles.
- **FAQ:** there are seven questions.
  - **New:** team collaboration, "Do I still need Blender?", and what kinds of games it is for.
  - **Updated:** the provider answer covers multiple providers, picking a model per task,
    credentials staying local and the cost budget.
  - **Unchanged position:** the offline question is still closed by default.

The palette, the wisp artwork and the existing interactions are unchanged.

## Changed paths

**Added** in `website/src/components/`:
- `ModelingSection.vue`
- `ModelingStage.vue`
- `LighthouseModel.vue`
- `WorkflowSection.vue`
- `WindowBar.vue`

**Modified:**
- `website/src/content.ts`
- `website/src/App.vue`
- `website/src/components/EditorStage.vue`, which now uses the shared title bar
- `website/src/components/HeroSection.vue`
- `website/src/components/SiteHeader.vue`
- `website/src/components/SiteFooter.vue`
- `website/src/components/FaqSection.vue`
- `website/tests/landing.spec.ts`, which gains one behavioural test

**Handoff files:** `result.md`, the new `result-1-palette.md`, and Astra's updated `brief.md`,
which is committed as received.

No dependencies were added. PLAN.md, the auth code, the native code and the CI files were not
touched.

## Commands and results

All commands ran from `website/` on 2026-10-08.

| Command | Result |
|---|---|
| `npm run typecheck` | Pass. Strict `vue-tsc` includes `tests/`. |
| `npm run build` | Pass |
| `PAGES_BASE_PATH=/incant/ npm run build` | Pass. Script, stylesheet and favicon URLs are prefixed with `/incant/`. |
| Structural check | Pass. The page was rendered with Vue's server renderer into a temporary directory and parsed as text. No browser was involved, and the temporary entry file was deleted. |

The structural check found the following:

- There is one h1, and heading levels never skip.
- There are no duplicate ids, and every in-page anchor resolves.
- The only external links are the repository and PLAN.md.
- None of the banned qualifier words appear.
- There is exactly one tab list, with 4 tabs and 1 tab panel. The graph nodes are 7 toggle
  buttons.

**Payload:** JS is 113.7 kB raw and 41.4 kB gzip, against a 100 kB gzip budget. CSS is 8.0 kB
gzip, and `dist/` totals 412 kB.

**Contrast:** the new colour pairs were computed with the WCAG formula, and all pass AA.

| Pair | Ratio |
|---|---|
| Muted text on the selected node | 6.09 |
| Accent on deep paper | 5.24 |
| Muted text on deep paper | 5.25 |
| Muted text on the viewport | 8.62 |
| Editor accent on the viewport | 7.17 |

The unselected graph nodes use muted colour rather than opacity, so they keep that contrast.

**Browser tests were not run.** The director's restriction covers them. The suite now has 23
tests, 22 existing and 1 new, for Astra's hosted CI to run. The new test checks that a graph
node can be selected with the keyboard, that exactly one node is pressed at a time, and that
the preview's accessible description changes. It does not assert copy or styles.

**Expected to pass:** I expect Astra's own `site.spec.ts` to pass unchanged. The page still has
a single tab list, the offline FAQ stays closed, the mobile menu still has a FAQ link, and every
anchor resolves.

## Screenshots

**None were taken for this revision.** Visual verification is deferred at the director's
request. The images on disk under `screenshots/before/` and `screenshots/after/` show the
palette revision only, not this page. Note that running `landing.spec.ts` in CI will write fresh
screenshots to `test-results/visual/`. Those would be the first renders of this revision.

## Deferred visual checks

These need someone to look at a render once capture is allowed again:

1. **Lighthouse model:** check silhouette, proportions and draw order at each of the seven graph
   steps. Also check the accent outline on each step and the wireframe overlay. All of this was
   placed by computed coordinates, not by eye.
2. **Graph stage at 320 and 390 px:** the viewport stacks above the node list. Check the
   truncation of node parameters and the title bar.
3. **Workflow track:** check dot alignment on the hairline at desktop and mobile, and the
   wrapping and slash spacing of the export row at 320 to 768 px.
4. **Page rhythm:** check spacing and hierarchy across the two new sections against the hero.
   Also check that the deep-paper workflow band sits well between two paper sections.
5. **Desktop header:** check that four navigation items and the GitHub link fit at 768 px.
6. **Test suite:** run the full browser suite, including both axe scans, the overflow checks at
   four widths, reduced motion and the new graph-node test.

## Promise-to-page map

| Page claim | PLAN.md source |
|---|---|
| One application: 3D/2D modeling tool, engine and editor, with an agent on the same project | 1.1 |
| Geometry graphs: primitives, booleans, extrude, bevel, subdivision, array, scatter, noise, curves, lofts, instancing; text-serialized and agent-authorable | Phase 4; 2.2 `incant_geo`; App. B `GeometryGraph` |
| Mesh cleanup, auto-retopology, auto-UV, LODs, decimation, normal/AO baking, light sculpt for adjustments | Phase 4 |
| Shader graph to WGSL, nodes for PBR inputs, math, textures, UV operations, vertex animation; WGSL written directly | Phase 4 |
| Terrain heightmaps, layered materials, foliage scatter, streaming; baked lightmaps and probe volumes on every tier, screen-space GI on desktop | Phase 4 |
| Retargeting, IK rigs, animation graphs, cinematic timeline; GPU particles with a graph editor | Phase 4 |
| Image-to-3D with cleanup, texture sets, humanoid animation from text; same cook pipeline as imports; licensing metadata | Phase 4; 6.6 |
| glTF and FBX import | Phase 1 |
| "Do I still need Blender": detailed sculpting stays in other tools by design | Phase 4 ("not full Blender sculpting"); 9 out-of-scope list |
| TypeScript gameplay, types from the schema, hot reload | 1.2 #7; Phase 1; 6.3 |
| Agent edits as one transaction with a readable diff, beside your own edits; undo in one step | 1.2 #4; 2.4; Phase 3 chat UI |
| Approval choice ("whether it asks first") | Phase 3 approval modes |
| Headless play-test returns frames, logs and assertions | Phase 1 headless runner; App. A `play.run` |
| Six export targets; generated Xcode and Gradle projects | 1.1; Phase 5; 6.7 |
| Steam integration, crash reporting, dedicated server build | Phase 5; Phase 6 |
| Multiplayer with server-authoritative and rollback modes, cross-play between desktop and mobile | Phase 6; 6.8; 1.3 |
| Templates: third-person, first-person, 2D platformer, top-down | Phase 2 |
| Mergeable documents; teammates and the agent edit at once; account adds sync and invitations | 2.3 CRDT; 3.2; 3.3 |
| Offline editor with no account; nothing behind a login | 3.3 |
| OpenAI, Anthropic, Google or local model, chosen per task; calls go direct; cost against a budget | 1.2 #5; 3.1; 3.4 |
| Credentials stay on the user's machine | 3.1, as amended by the director (see below) |
| Agent has no shell access; sandboxed scripts | 3.5 |

## Limitations

- **Visual quality is unverified:** see the deferred checks above. The geometry is reasoned,
  not observed.
- **Finished-product voice:** the copy presents everything in PLAN.md as shipped, as the
  director instructed. The repository is in Phase 0, so the page describes the product's
  intended scope, not its verified state. This marketing presentation approves no phase gate.
- **Credential wording:** this follows the director's verbal update. The worktree's PLAN.md
  copy still says keychain, and I didn't change it.
- **Driftwake left out:** the launch game is not named on the page, because PLAN.md calls the
  name a placeholder.
- **Payload growth:** JS grew by 5.9 kB gzip from the new sections. That is still well under
  budget.
- **Inherited limitations:** the tablet crop of the hero scene and the lack of Firefox, Safari
  and real-device testing carry over from earlier revisions.
