# Result: 0007 landing page, revision 5 (graphics-first redesign)

Dated 2026-10-08. Earlier results are kept unchanged as history:

- **`result-1-palette.md`:** the neutral palette.
- **`result-2-expansion.md`:** the modeling and workflow expansion, `317cf6b`.
- **`result-3-density.md`:** the density pass, `80e3649`. It is integrated as `1acb2d5`, and its
  23 hosted browser tests passed.

## Status

The redesign is implemented, type-checked, built and committed with `Built-by: claude` for
Astra to integrate into PR #2. Nothing was pushed, published, deployed or merged, and no phase
gate is claimed or approved.

**This revision has not been seen rendered by anyone.** The screen-capture ban held
throughout: no computer use, browser, Playwright run, native automation or capture. The only
network use was `npm pack` for the icon packages, run from a terminal into an isolated temporary
directory. Every visual judgment below comes from source review, coordinates worked out by hand,
contrast arithmetic and a text-only render.

## Model

Claude Opus 5.5 (`claude-opus-5-5`), running in Claude Code.

## Director feedback acknowledged

The director said: "landing page it still too text heavy. Needs more graphical things… the
platforms can be represented as their actual svg icons instead of text. REdesign again with
this in mind". Both new sections now explain through pictures, and their remaining lists of
terms are gone.

The director also said: "next time dont ask, just continue". Routine design choices were made
without asking. The screen-capture restriction was still respected.

## Graphical changes

1. **Platform marks replace the word row.** The six large platform names became the real
   platform marks:
   - The Windows logo.
   - Apple's mark, shown twice, for macOS and iOS.
   - Tux for Linux.
   - The Android robot.
   - A globe for the web.

   Each mark has a small visible name, which is also what assistive technology reads. That keeps
   macOS and iOS distinct even though they share Apple's mark. The paths are copied unmodified.
2. **A strip of illustrated viewports replaces the toolset index.** The six term-and-gloss
   entries became one connected charcoal strip in the same chrome as the editor panels. It holds
   six small drawings, each with a label of two or three words:

   | Label | Drawing |
   |---|---|
   | Retopology · UVs | A quad-wire clay sphere beside its unwrapped UV islands |
   | LODs · Decimation | One rock at three decreasing polygon counts, each with an interior edge fan |
   | Shader graphs | Two texture nodes wired into a shaded stone sphere |
   | Terrain · Foliage | A layered ridge of rock, grass and sand, with trees and a dashed brush cursor |
   | Rigs · IK · Timelines | A walking rig with an IK target on one foot, above a keyframed timeline with a playhead |
   | Image to 3D | A flat picture of a crate turned into a wired 3D crate |

   Each drawing has its own accessible description. The strip is two columns on phones, three
   on tablets and six from 1280 px.
3. **Full detail sits behind a disclosure.** A closed "The whole toolset" disclosure holds the
   complete Phase 4 list, including geometry operations, baking, sculpting, WGSL, lightmaps,
   VFX and glTF or FBX import. Nothing is lost, but nobody is made to read it.
4. **A pipeline diagram replaces the workflow track.** The verb-and-artifact track became six
   rounded charcoal nodes on one wire. Each node has a glyph:
   - Model: a box.
   - Surface: blended circles.
   - Script: braces.
   - Review: a file diff.
   - Play-test: a gamepad.
   - Ship: a package.

   Each node has a one-word label, and Ship is ringed in the accent. When motion is allowed, a
   soft accent pulse travels along the wire. The diagram is horizontal from 768 px and becomes a
   vertical rail on phones. The artefact each step leaves is read only to screen readers. The
   workflow's sub-line was removed, so the heading, the diagram and then the platform marks tell
   the story.
5. **Principles open with diagrams.** Each principle now opens with a small ink-on-paper
   drawing, and these replace the numerals:
   - **History:** your edits in ink and the agent's in accent on one line, with the last one
     hollow and an undo arrow over it.
   - **Plain text:** a document diff with one added line.
   - **Evidence:** three play-test frames tracing a jump, ending in a pass check.

### Copy

These counts are words in the text-only render. Collapsed disclosure text is counted but not
visible.

| Section | Words in revision 4 | Words in revision 5 | Note |
|---|---|---|---|
| Workflow | 57 | 32 | Six one-word labels, six platform names and one caption line |
| Modeling | 148 | 189 | About 70 of these are inside the closed disclosure |

In the modeling section, the visible additions are only the six vignette labels and the
disclosure summary.

## Sources and licenses for the new SVG assets

| Asset | Source | License | Where |
|---|---|---|---|
| Windows, Apple, Linux and Android marks; web globe | Font Awesome Free 7.3.1, `@fortawesome/fontawesome-free`. The brand marks are in `svgs/brands` and the globe in `svgs/solid`. | Icons CC BY 4.0. The Android robot is also CC BY 3.0, by Google. | `website/src/icons/platforms.ts`; license in `website/public/licenses/font-awesome-free-LICENSE.txt` |
| Pipeline glyphs: box, blend, braces, file-diff, gamepad-2, package | Lucide 1.53.0, `lucide-static` | ISC | `website/src/icons/pipeline.ts`; license in `website/public/licenses/lucide-ISC.txt` |
| Toolset vignettes and principle diagrams | Original, hand-written SVG | Part of this site | `ToolVignette.vue` and `PrincipleDiagram.vue` |

- **Attribution:** the footer credits Font Awesome under CC BY 4.0, Lucide under ISC and Google
  for the Android robot under CC BY 3.0. It also notes that platform names and marks belong to
  their owners. The credit is plain text, because the existing tests allow outbound links only
  to the repository.
- **Source headers:** both icon modules record their source, version and license in a header
  comment.
- **Windows mark source:** Simple Icons 16.34.0, which is CC0, was checked first. It has no
  Windows mark, so all five platform glyphs come from one family for visual consistency.
- **Trademark use:** the marks name export targets only, and their owners' guidelines still
  apply. That is a legal review item for the director, not something code can settle.

## Changed paths

**Added:**
- `website/src/icons/platforms.ts`
- `website/src/icons/pipeline.ts`
- `website/src/components/ToolVignette.vue`
- `website/src/components/PrincipleDiagram.vue`
- `website/public/licenses/font-awesome-free-LICENSE.txt`
- `website/public/licenses/lucide-ISC.txt`

**Modified:**
- `website/src/content.ts`
- `website/src/components/ModelingSection.vue`
- `website/src/components/WorkflowSection.vue`
- `website/src/components/PrinciplesSection.vue`
- `website/src/components/SiteFooter.vue`
- `website/src/components/ModelingStage.vue`
- `website/src/style.css`

**Handoff files:** `result.md`, the new `result-3-density.md`, and Astra's updated `brief.md`,
which is committed as received.

**Two existing changes:**
- **ModelingStage.vue:** its decorative wire line moved out of the `<ol>` into a wrapper, so the
  list holds only list items. Its behaviour is unchanged.
- **style.css:** it gains one `flow` keyframe animation.

No runtime dependency was added. The icons are inlined as path data.

## Commands and results

All commands ran from `website/` on 2026-10-08.

| Command | Result |
|---|---|
| `npm pack simple-icons lucide-static @fortawesome/fontawesome-free` | Run in `/tmp/incant-icons/dl` to read sources and licenses. Data only, and no package scripts were executed. |
| `npm run typecheck` | Pass. Strict `vue-tsc` includes `tests/`. |
| `npm run build` | Pass |
| `PAGES_BASE_PATH=/incant/ npm run build` | Pass. Script, stylesheet and favicon URLs are prefixed with `/incant/`, and both license files are in `dist/licenses/`. |
| Structural checks | Pass. The page was built with Vite's SSR mode into `/tmp`, rendered by Vue's server renderer and parsed as text. No browser was involved. |

The structural checks found the following:

- There is one h1, and heading levels never skip.
- There are no duplicate ids, and every in-page anchor resolves.
- The only external links are the repository and PLAN.md.
- None of the future-tense or access-qualifier words appear.
- There is one tab list with 4 tabs and 1 tab panel. There are 7 pressed-state graph buttons.
- Every `ul` and `ol` contains only `li`, and every `dl` contains only `div`, `dt` and `dd`. A
  stack parser checked this.
- All 6 platform paths and all 6 pipeline glyphs rendered. The glyphs have 3, 2, 2, 4, 5 and 4
  elements, respectively.
- The page has 8 images with descriptions and 8 disclosures. The disclosures are the 7 FAQ items
  and the toolset.

**Payload:** JS is 131.9 kB raw and 48.1 kB gzip, against a 100 kB gzip budget. The increase
is mostly the Tux path. CSS is 8.2 kB gzip, and `dist/` totals 444 kB.

**Contrast:** the new text pairs were computed with the WCAG formula, and all pass AA.

| Pair | Ratio |
|---|---|
| Vignette labels, muted text on the panel | 8.28 |
| Platform labels and footer credit, muted text on deep paper | 5.25 |
| Ship glyph, `#a9b0ff` on the panel | 9.06 |

**Motion:** the pulse is wrapped in `motion-safe`, so it doesn't exist when reduced motion is
requested.

**Browser tests were not run,** under the restriction. Astra's hosted CI will run them.

- **Unchanged:** all 23 tests are as they were. Nothing they select was renamed: the tabs, the
  graph buttons in `#create`, the FAQ items and the mobile menu.
- **No new tests:** the new disclosure is native `<details>`, and the diagrams are static.

## Screenshots

**None were taken.** Visual verification is deferred at the director's request. The images in
`screenshots/` show the palette revision only. A CI run of `landing.spec.ts` writes fresh
renders to `test-results/visual/`.

## Deferred visual checks

1. **Vignettes:** check legibility and balance of all six at their real cell widths. Those are
   about 175 px on phones, about 300 px at tablet, and about 200 px at 1280 px and up. The
   drawings were placed by coordinates and have never been seen.
2. **Platform marks:** check optical balance. The source glyphs have different proportions:
   Android is wide and short, while Tux and Apple are tall. They share one height, which may
   need per-mark nudges.
3. **Pipeline:** check that the wire passes through the node centres at 768 px and up, and that
   the rail does so on phones. Check that the pulse reads as subtle.
4. **Principle diagrams:** check their weight against the headings beside them.
5. **Overall pacing:** check the page from top to bottom at 1440, 768 and 390 px.
6. **Test suite:** run the full suite, including axe, overflow and reduced motion.

## Promise-to-page map

| Page element | PLAN.md source |
|---|---|
| Graph stage: primitive, taper, extrude, array, boolean, scatter with noise; output with retopology, UVs and LODs | Phase 4; App. B `GeometryGraph` |
| Retopology · UVs; LODs · Decimation | Phase 4 mesh tools |
| Shader graphs | Phase 4 shader graph to WGSL |
| Terrain · Foliage | Phase 4 terrain |
| Rigs · IK · Timelines | Phase 4 animation tools |
| Image to 3D | Phase 4 asset generation v2; 6.6 |
| "The whole toolset" disclosure | Phase 4 in full; Phase 1 glTF and FBX import; light sculpting only |
| Pipeline: model, surface, script, review, play-test, ship | 1.1; 1.2; 2.4; Phase 1 TypeScript and headless runner; Phase 3 |
| Platform marks: Windows, macOS, Linux, iOS, Android, Web | 1.1; Phase 5 |
| Xcode and Gradle projects, Steam, crash reporting, dedicated servers | Phase 5; 6.7; Phase 6 |
| Principles: one history; diffable and mergeable documents; evidence | 1.2 #1–#4; 2.3 |
| Hero, statement band and FAQ claims | Unchanged from revision 4; mapped in `result-3-density.md` |

## Limitations

- **Unverified visually:** no render of this revision has been seen. See the deferred checks
  above.
- **Trademark guidelines:** each platform owner's guidelines still apply, and a human should
  review that use before any public launch.
- **Payload growth:** JS grew by 7.7 kB gzip. It is still under budget.
- **Inherited limitations:** the finished-product voice, the credential wording and the tablet
  crop of the hero scene carry over from earlier results.
