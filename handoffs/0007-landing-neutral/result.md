# Result: 0007 landing page, revision 6 (workflow middle ground)

Dated 2026-10-08. Earlier results are kept unchanged as history in this folder:

- `result-1-palette.md`
- `result-2-expansion.md`
- `result-3-density.md`
- `result-4-graphics.md`: the graphical pass, `81a7f9b`. It also records the platform-icon
  sources and licenses, which are unchanged.

## Status

This focused correction is implemented, type-checked, built and committed with
`Built-by: claude` for Astra to integrate into PR #2. Nothing was pushed, published or merged,
and no phase gate is claimed or approved.

**No render of this revision has been seen by anyone working on it.** The screen-capture ban
held throughout. No computer use, browser, Playwright run, native automation or capture was
used. The director's attached screenshot was read as feedback only.

## Model

Claude Opus 5.5 (`claude-opus-5-5`), running in Claude Code.

## Director feedback acknowledged

The director said: "This section was taken to far in terms of removing text. It must find a
good middle ground". They showed six icons with one-word labels, a large empty band and the
platform row, and that read as under-explained.

This correction changes only the workflow section and its content. The graphical pipeline and
the real platform SVGs stay. `SiteFooter.vue` and the browser tests were not touched, so
Astra's integration changes to them are preserved.

## What changed

### Visible copy is back, at a middle length

**New introduction:** "Six steps in one project and one history. Do any of them by hand, hand
them to the agent, or mix both."

Each step now has one concrete line of 11 to 14 words about what the visitor does or gets. Each
line covers a different part of the product:

| Step | Line |
|---|---|
| Model | Build meshes in a geometry graph, or import your own and clean them up. |
| Surface | Layer materials in a shader graph that compiles to WGSL for every platform. |
| Script | Write gameplay in TypeScript, typed from your scene, with hot reload. |
| Review | Read the agent's work as one diff, then keep it or undo it in a step. |
| Play-test | A headless run returns frames, logs and passed checks as proof it works. |
| Ship | Export the same project to desktop, mobile and the web. |

The screen-reader-only step descriptions are gone. Everyone now reads the same visible text,
and the workflow section has no screen-reader-only content left.

| Workflow version | Visible words |
|---|---|
| Revision 3, documentation-length | 193 |
| Revision 5, icons and single words | 32 |
| Revision 6, this pass | 131 |

### Hierarchy: each explanation belongs to its icon

- **Layout:** each step is one unit of icon node, title and line.
- **From 1280 px:** the horizontal wire runs through the node centres, with each title and line
  centred under its node. At that width the six columns are about 190 px, wide enough for a
  sentence. Columns at 768 to 1024 px would have been 115 to 155 px, too narrow.
- **Below 1280 px:** the steps sit on a vertical rail with the text beside each node, capped at
  42 rem wide. That covers phones, tablets and small laptops.
- **Rail connectors:** each step draws its own connector from its node centre to the next node
  centre. The rail therefore stays continuous and stops exactly at the last node, however many
  lines a step's text wraps to.
- **Step titles:** these are now `h3`s set in 17 px semibold sans. They are clearly subordinate
  to the serif section heading, and the heading order stays valid.
- **Introduction placement:** it sits beside the heading on desktop and below it on phones.

### Spacing: the dead band is gone, with breathing room kept

| Gap | Before | After |
|---|---|---|
| Section padding | 112–192 px | 96–128 px |
| Heading to pipeline | 96 px | 80 px from `sm` |
| Pipeline to platform row | 128 px plus 64 px rule padding | 64 px plus 48 px rule padding |
| Platform icon height | 44–56 px | 40–48 px |
| Platform row to caption | 56 px | 40 px |

- **Section padding** was trimmed so the band no longer reads as empty space.
- **Pipeline to platform row:** the gap was roughly halved, so the platform row reads as where
  "Ship" lands.
- **Platform icons** were reduced slightly so they don't outweigh the pipeline nodes. They keep
  their small visible names.

## Changed paths

- `website/src/components/WorkflowSection.vue`
- `website/src/content.ts`, where the workflow steps gain a visible `body` and drop the
  screen-reader-only `artifact`
- `handoffs/0007-landing-neutral/result.md`
- `handoffs/0007-landing-neutral/result-4-graphics.md`, the preserved previous result
- `handoffs/0007-landing-neutral/brief.md`, which is Astra's update, committed as received

## Commands and results

All commands ran from `website/` on 2026-10-08.

| Command | Result |
|---|---|
| `npm run typecheck` | Pass. Strict `vue-tsc` includes `tests/`. |
| `npm run build` | Pass |
| `PAGES_BASE_PATH=/incant/ npm run build` | Pass. Script, stylesheet and favicon URLs are prefixed with `/incant/`. |
| Structural checks | Pass. The page was built with Vite's SSR mode into `/tmp` and rendered by Vue's server renderer as text. No browser was involved. |

The structural checks found the following:

- There is one h1, and heading levels never skip.
- There are no duplicate ids, and every in-page anchor resolves.
- The only external links are the repository and PLAN.md.
- None of the future-tense or access-qualifier words appear.
- There is still one tab list, with 4 tabs and 1 panel. The 7 graph buttons are unchanged.
- Every list contains only `li` children.
- The workflow section has no screen-reader-only text.

**Payload:** JS is 132.6 kB raw and 48.4 kB gzip, against a 100 kB gzip budget. CSS is 8.3 kB
gzip, and `dist/` totals 444 kB.

**Contrast:** no new colour pairs were added. Muted text on deep paper is 5.25:1, and ink on
deep paper is above 15:1.

**Browser tests were not run,** under the restriction. Astra's hosted CI will run them.

- **No change to tests:** nothing they select changed, and there is no new interaction.
- **Heading-order check:** the landing heading-order test now meets six more `h3`s after the
  workflow `h2`, which is a valid h2 to h3 step.

## Screenshots

**None were taken.** Visual verification is deferred at the director's request.

## Deferred visual checks

1. **At 1280 to 1440 px:** check that the six centred lines sit evenly under the wire. Each wraps
   to two or three lines.
2. **At 768 to 1279 px and on phones:** check that the rail connectors meet node centres and
   that the 42 rem text column reads comfortably.
3. **Overall balance:** check that the section no longer shows a dead band, and that the
   platform row reads as the result of "Ship".

## Promise-to-page map for the changed copy

| Claim | PLAN.md source |
|---|---|
| Geometry graphs; import and clean up meshes | Phase 4 geometry graph and mesh tools; Phase 1 import |
| Shader graph compiling to WGSL for every platform | Phase 4 shader graph; 2.1 wgpu backends |
| TypeScript, typed from the scene, hot reload | 1.2 #7; Phase 1 SDK; 6.3, where bindings come from the schema registry |
| Agent's work as one diff, kept or undone in one step | 1.2 #4; 2.4; Phase 3 chat UI inline diffs and "revert this turn" |
| Headless run returns frames, logs and checks | Phase 1 headless runner; App. A `play.run` |
| Export to desktop, mobile and web from the same project | 1.1; Phase 5 |
| Work by hand, by agent, or both, in one history | 1.1; 1.2 #1 |

Every other section is unchanged from revision 5. Its map is in `result-4-graphics.md`.

## Limitations

- **Unverified visually:** see the deferred checks above.
- **Inherited limitations:** these come from earlier results. They cover the platform-mark
  trademark review, the finished-product voice and the tablet crop of the hero scene.
