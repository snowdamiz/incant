# Product landing page and GitHub Pages delivery

Date: 2026-10-08 (America/New_York). This work is independent of engine phase gates.

## Scope and provenance

The director requested a modern landing page using Vue, Vite and Tailwind, with
GitHub Pages deployment on pushes to `main`, then explicitly requested a
finished-release presentation. Public marketing follows that direction. Internal
engine status and phase approvals are unchanged.

The implementation branch starts from `origin/main` at `11b569c`. The ongoing
editor implementation and its uncommitted changes remain in the original checkout.
The website lives in `website/`, with a separate lockfile, so it can build without
the editor, Rust toolchain or any build-time agent account. It adds no engine
project mutation path.

Visual implementation and rendered review are routed to Claude Opus 5.5 using
`python3 tools/handoff/main.py run 0005-landing-page --permission-mode acceptEdits`.
The runner from the current implementation checkout supplies the newer ACP
permission/model handling; the website branch does not change that tooling.
The design packet and result are in `handoffs/0005-landing-page/`.
Astra owns build integration, functional checks, CI and this evidence record.

## Deployment behavior

`.github/workflows/website.yml` checks site-related pull requests and builds on
every push to `main`. It derives the repository subpath, runs strict Vue/TypeScript
checking and production browser tests, then uploads only `website/dist`. Deployment
is restricted to `main`; PR runs receive no Pages or OIDC write permissions.
The deployment job uses GitHub's Pages environment and the official pinned
configure, upload and deploy actions. Branch concurrency serializes main runs.

Only public marketing assets ship. No build-time account, provider credential,
handoff reference, test output, project document, or Rust artifact is included.

## Verification

- All direct website dependency versions and transitive resolutions are pinned.
- `npm audit` in `website/`: zero vulnerabilities when installed.
- `actionlint` 1.7.12: workflow passed. Its release checksum was verified before use.
- Both convention files were regenerated and `generate_conventions.py --check`
  passed without changes.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` passed on the
  unchanged bootstrap baseline.
- `cargo test --workspace --release --locked` passed on that baseline, which has
  zero Rust tests. This is not engine behavior evidence.
- `cargo fmt --all --check` reports pre-existing blank-line formatting differences
  in the five empty library files on the bootstrap baseline. No Rust files were
  changed for the website.
- The bootstrap branch has no `tools/tests/`; the later implementation branch's
  tool and editor test suites are outside this independent landing-page branch.

### Integrated website results

The first version's GitHub Actions build also passed on Linux (run 37871614168).
The director then rejected its appearance as generic and requested substantial
refinement. The following results establish functional evidence for version 1;
they do not imply acceptance of its design. Revision 2 is routed to Claude in the
updated handoff packet.

- Claude visual commit `7557c10` was integrated as `dc9c11b`, preserving its
  `Built-by: claude` provenance. Its templates, styles and artwork were retained.
- The only cherry-pick conflict was the independently created Playwright config.
  The integrated config uses a portable browser executable override, two workers,
  matching preview base paths, isolated server startup and CI retry/trace support.
- Fresh `npm ci` and `PAGES_BASE_PATH=/incant/ npm run build`: passed, including
  strict checking of Vue, TypeScript, build config and tests.
- All **22 production browser checks passed in 9.4 seconds** using local Chrome.
  Coverage includes real asset loading, no third-party requests, no overflow at
  320/390/768/1440px, resolved anchors, skip navigation, mobile menu dismissal,
  workflow tab keyboard selection, FAQ disclosure and reduced motion. axe found
  zero violations, including WCAG A/AA scans at 390 and 1440px.
- Integrated bundle: JavaScript 119.81 kB raw / 43.00 kB gzip; CSS 52.94 kB raw /
  9.57 kB gzip; local fonts 48.25 and 40.40 kB. Total deployed size: 268 kB on disk.
- Claude reviewed actual desktop, tablet, mobile, menu and workflow-state renders
  and fixed clipping, wrapping, overlap and menu-background issues. See the full
  findings and browser limitations in `handoffs/0005-landing-page/result.md`.
- Normal test captures go to ignored `website/test-results/visual/`; the reviewed
  screenshots are retained unchanged in the handoff. An explicit
  `INCANT_EVIDENCE_DIR` override can regenerate evidence intentionally.

These are website checks, not engine or live-provider release evidence. Firefox,
Safari and physical-device visual checks were not run. Public CTA destinations
are the real repository and its product plan; they require repository access.

### Revision 2 integration

Following the director's rejection of the first visual design, Claude produced
`cbfbfab`, integrated as `373a2bf`. The new design uses a warm paper/ink palette,
Fraunces display typography, fewer sections and a single interactive editor stage.
Ask, Review, Play-test and Undo change the illustrated scene, outline, inspector
and status together. The critique and design rationale are in
`handoffs/0005-landing-page/revision-2.md`; reviewed screenshots are preserved in
`handoffs/0005-landing-page/screenshots/revision-2/` alongside the original evidence.

Fresh `npm ci` reports zero vulnerabilities; the strict production build succeeds.
All 22 integrated browser checks pass in 9.9 seconds, including zero axe
violations and WCAG A/AA checks. Independent behavioral tests exercise the revised
controls without hard-coding marketing copy. Actionlint and generated-convention
checks remain clean. Both reviewed screenshot sets remain unchanged by tests.

The integrated revision ships 95.86 kB JS / 35.54 kB gzip and 34.74 kB CSS /
7.43 kB gzip. Four local font files total 228.85 kB; their OFL licenses ship in
`public/licenses/`. The complete deployed directory is 392 kB on disk including
licenses. The CI workflow and the unresolved private-repository Pages prerequisite
remain as described below. Director visual acceptance and merge remain pending.

### Revision 3: director-requested neutral colors

The director asked to reduce purple and match the previous editor colors. Claude
Opus 5.5 completed packet 0007 in commit 6090702, integrated as 5e677a7. The landing
page keeps the revision-2 layout and interactions, with neutral graphite editor
panels, slate scene art, an ink statement band and restrained indigo accents.
The approved wisp shape stays unchanged. Palette notes and before/after captures
are retained in `handoffs/0007-landing-neutral/`.

The integrated `/incant/` production build and all 22 browser checks passed in
10.6 seconds, including both accessibility suites. The local preview serves the
rebuilt assets. GitHub CI is running on the updated PR. Publishing prerequisites
remain unchanged; this work did not alter billing, repository visibility or Pages.

### Revision 4: modeling and the complete product workflow

The director requested more of PLAN.md's product scope, specifically the missing
3D modeling capabilities, and reaffirmed the finished-product presentation.
Claude Opus 5.5 resumed the same ACP session and produced `317cf6b`, integrated as
`11abac8`. Public copy uses the present tense without future-feature labels.

The page now includes an interactive geometry graph that builds a lighthouse
from a cylinder through taper, extrusion, array, boolean and scatter steps to a
wireframe output. Six editorial groups explain modeling, mesh cleanup, materials,
terrain/light, animation/effects and generated assets. A second section connects
Model, Surface, Script, Review, Play-test and Ship, with all six export targets.
The hero identifies Incant as a 3D/2D modeling tool, engine and editor. The neutral
palette, connected panels, wisp artwork and prior interactions remain intact.
The handoff result maps product claims to PLAN.md sections and retains the earlier
palette report separately as `result-1-palette.md`.

The director's computer-use and screen-capture restriction remained in force for
the entire revision. Neither Claude nor Astra launched a browser, used native
UI automation, or captured local pixels. Claude ran strict type checking, both
base-path builds and a text-only Vue server-rendered structural check. Astra's
integrated `/incant/` production build also passed: 113.67 kB JavaScript raw /
41.39 kB gzip and 38.13 kB CSS raw / 8.02 kB gzip. No dependencies were added.
A direct HTTP check confirms the existing local preview serves that exact build
and its assets; this is not a visual or interaction check.

Hosted browser verification for source revision `11abac8` passed in
[run 37879493383](https://github.com/snowdamiz/incant/actions/runs/37879493383),
including all 23 tests; deployment was skipped for the PR. The director then
reported that the added sections were too text-heavy and too close together.
Claude is revising their density and overall page spacing; passing browser tests
is not acceptance of the visual design. Agent visual review remains deferred. The screenshots in
`screenshots/after/` establish only the earlier palette revision, not the new
modeling and workflow sections. The public copy follows the director's requested
presentation; the engine remains in Phase 0 and no phase gate is approved.

### Revision 5: less text and more space

The director found the new sections too text-heavy and crowded. Claude's
`80e3649`, integrated as `1acb2d5`, replaces six tool paragraphs with a short
two-column index and reduces the workflow to six actions and their outputs.
The modeling and workflow openings now each have one heading and one sentence.
Larger inner gaps and section padding separate the illustrations, index, workflow
and export platforms. The principles, hero and FAQ are shorter as well. On small
screens, graph nodes use a compact horizontal strip below the viewport.

Claude's text-only before/after measurement records modeling content falling from
324 to 148 words and workflow content from 193 to 57; the two sections drop from
fifteen headings to two. The result preserves the full product presentation,
neutral palette and existing interactions. This is a response to the director's
design feedback, not a claim of visual acceptance.

The integrated strict production build passes with `/incant/`: JavaScript is
110.76 kB raw / 40.37 kB gzip and CSS is 38.45 kB raw / 8.06 kB gzip. An HTTP check
confirms the existing preview serves this build. Hosted verification passed all 23 browser tests, including both accessibility
suites, in [run 37880016814](https://github.com/snowdamiz/incant/actions/runs/37880016814).
Deployment was skipped for the PR.
Local computer use and capture stayed stopped. The latest result packet records
remaining visual checks and preserves the expansion report separately as
`result-2-expansion.md`.

### Revision 6: graphical explanations and platform SVGs

The director asked for another redesign with more graphics, specifically actual
SVG platform icons. Claude's `81a7f9b`, integrated as `f436bf7`, replaces the
platform-name row with Font Awesome marks for Windows, Apple/macOS, Linux,
Apple/iOS, Android and the web. Small accessible labels distinguish the targets.
The source and license records ship in the site, with Lucide icons used for the
workflow. Astra's `4fd741e` links the visible credits to the license pages and
updates the existing outbound-link check to allow genuine HTTPS credit links.

Six original SVG tool illustrations now show topology/UVs, LODs, shader graphs,
terrain, rigging/timelines and image-to-3D in connected charcoal viewports. The
full tool descriptions are available in a closed disclosure. The workflow uses
icons connected by a wire, with reduced-motion handling, and the three principles
use small explanatory diagrams. The finished-product voice and neutral palette
remain intact.

The integrated production build passes at `/incant/`: JavaScript is 132.05 kB raw /
48.18 kB gzip and CSS is 40.02 kB raw / 8.19 kB gzip. The local preview serves that
build, verified by HTTP. The first hosted run for `4fd741e` passed 22 checks but
failed the geometry keyboard test because its generic image selector also
matched the six new illustrations. `2f04e7d` scopes that locator to the geometry
editor's accessible name; the interaction assertions remain intact. All 23 tests
passed in [run 37881244624](https://github.com/snowdamiz/incant/actions/runs/37881244624).
No local browser, computer use or capture ran. Agent visual review is still
deferred; the packet records the unreviewed icon proportions, drawings and layout.
Earlier reports remain as `result-1-palette.md`, `result-2-expansion.md` and
`result-3-density.md`.

### Revision 7: restore a useful middle ground in the workflow

The director's screenshot feedback said the workflow had removed too much text.
Claude's `76ce75c`, integrated as `bf99437`, restores a short introduction and a
single visible explanation beneath each of the six steps. The icons and platform
SVGs remain. The section's visible copy is now 131 words, between the original
193 and the graphical pass's 32. This is a description of the change, not a word
count acceptance criterion.

The heading is smaller, section padding and the gap before the platform row are
reduced, and a vertical rail below 1280 px keeps each explanation next to its
icon. On wider screens the six columns retain the connected horizontal diagram.
The correction is scoped to the workflow; the other graphical sections remain.

The integrated strict production build passes with `/incant/`: JavaScript is
132.77 kB raw / 48.46 kB gzip; CSS is 40.47 kB raw / 8.31 kB gzip. HTTP confirms the
existing preview serves this build. All 23 hosted browser tests passed in 16.6s
in [run 37881478211](https://github.com/snowdamiz/incant/actions/runs/37881478211).
PR deployment was skipped. No local browser or capture ran; the director's
attached screenshot is feedback, not permission to resume capture. Agent visual
review remains deferred. The earlier graphical result is preserved as
`result-4-graphics.md`.

## Hosting prerequisite

The authenticated GitHub Pages creation call for `snowdamiz/incant`, with
`build_type=workflow`, returned HTTP 422:

> Your current plan does not support GitHub Pages for this repository.

At that check the repository was private. No Pages site was created, and this
implementation did not change billing or repository visibility. A later read-only
GitHub check on 2026-10-08 reports the repository as public; the Pages endpoint
returns HTTP 404, so a configured site is not yet confirmed. The previous
private-repository eligibility failure is historical, not the current blocker.
On 2026-10-09 the director authorized agents to merge completed PRs after review
and passing checks. The Pages creation call now succeeds with `build_type=workflow`,
HTTPS enforced, and the expected URL `https://snowdamiz.github.io/incant/`. Repository
visibility and billing were not changed. PR #2 merged into main as `6a0841b` after its website and source checks passed.
[The main-branch workflow](https://github.com/snowdamiz/incant/actions/runs/37885030479)
successfully built, tested and deployed the site. A direct HTTPS read returned 200
with the expected Incant document and `/incant/assets/` references. This verifies
hosting, not a new visual review. [Deployment evidence](evidence/landing-deploy-2026-10-09.json).

References: [Vite GitHub Pages deployment](https://vite.dev/guide/static-deploy.html#github-pages),
[Tailwind Vite integration](https://tailwindcss.com/docs/installation/using-vite).

### Revision 8: rendered review after capture permission returned

The director restored capture permission. Claude reviewed the current page at
320, 390, 768, 1024, 1280 and 1440 px in handoff 0008. The workflow now uses a
three-by-two grid on tablets to avoid a half-empty section, smaller phone icons
leave room for the explanations, platform marks are centered on their visible
shapes with balanced sizes, and the narrow modeling window retains its project
name. Each step keeps an icon, title and sentence. Source artwork and licenses
remain unchanged.

Claude’s 24 browser tests pass, including a width regression that fails against
the old tablet layout. The integrated strict build also passes: JavaScript is
48.65 kB gzip and CSS is 8.40 kB gzip. The new changes are separate from deployed
PR #2 and await their own hosted integration checks. Before/after review evidence
and exact commits are in the [result packet](../../handoffs/0008-native-and-site-review/result.md).
