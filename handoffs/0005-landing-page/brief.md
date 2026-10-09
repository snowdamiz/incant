# Incant product landing page

## Revision 2 — director rejected the first design

Latest feedback, verbatim:

> the design needs to be heavily refined and improved. Right now it looks vibe coded and a bit generic

This feedback supersedes the previous visual completion claim. The first version
is committed as `7557c10` in this worktree and integrated in PR #2. Its functional
checks pass, including Linux CI, but that does not satisfy the director's visual
quality requirement. Do not defend the first result with test scores.

Treat this as a substantial redesign, not another spacing adjustment. First
critique your own original desktop and mobile renders: identify the specific
composition, typography, content, illustration and repeated UI patterns that
make it feel generic. Then choose and implement a materially stronger, coherent
visual direction appropriate to a professional creative tool. You own that
direction; do not ask Astra to make appearance decisions. The first screen and
overall page structure should make the improvement unmistakable.

Reconsider the first version's heavy purple glow, pill treatments, many rounded
feature cards, decorative micro-labels, technical copy density, and conceptual
editor's level of craft. Keep only elements justified by your new direction.
Aim for restraint, distinctive typography, confident hierarchy, intentional
spacing, a compelling product focal point and a considered reading rhythm.
Avoid merely swapping colors or adding more decoration. Reduce copy and sections
where they weaken the story. Preserve the wisp identity and the director's
finished-product voice. You may replace the current artwork, change the theme,
use a better licensed self-hosted font (pin dependencies and lockfile), and
restructure components when that improves the result. Do not use another model.

Write a short revision rationale in `handoffs/0005-landing-page/revision-2.md`,
including your critique and what changed. Implement first, then review real
desktop/mobile renders and interaction states. Preserve the original screenshots;
save the new evidence under `screenshots/revision-2/`. Update `result.md` for the
final result. Work efficiently: one substantial redesign pass followed by focused
corrections to observed defects. Browser metrics alone are not visual acceptance.

Integration notes: Astra already supplied the production workflow and portable
Playwright config in the integration branch. Do not edit CI, Vite plumbing,
`package.json` scripts, or `playwright.config.ts` in this revision. You may update
your `tests/landing.spec.ts` for new interactions and the revision screenshot
destination. Astra will adapt the independent behavioral tests if the design
changes labels or control structure. Keep semantic navigation, a functional
interaction, reduced motion and accessible mobile behavior. Use your existing
worktree's `npx playwright test` for local visual evidence. Port 4176 is the user's
existing integration preview; do not stop it. Manage only your own server on
4175/5175 by its exact PID. No broad pkill commands.

## Director request and latest priority

“Design a beautiful, professional, and modern landing page for this app, use vue,
vite, tailwind. Should be deployed to github pages via CI when pushed to main.”
The director then explicitly corrected the proposed early-stage messaging:
“imply finished release”. Give the public page a confident finished-product tone.
Keep Phase 0 and internal implementation tracking out of public marketing copy.
Do not invent customer counts, quotes, awards, pricing, version numbers, downloadable
installers, signup backends, or working demos. Use genuine repository destinations
and working in-page interactions. Internal evidence remains accurate.

## Ownership and scope

Claude Opus 5.5 through ACP owns all appearance, copy composition, Vue templates,
CSS/Tailwind, original SVG artwork, interaction design, screenshots, and rendered
review. Astra owns Vite plumbing, CI, behavioral verification and integration.
Work only in this isolated handoff worktree, not the director's active checkout.
Allowed paths: website/ and handoffs/0005-landing-page/. Do not modify existing
engine/editor code, root manifests, workflow files, credentials, or other handoffs.

Read PLAN.md for product context. Incant is a text-native game engine and editor:
human and AI work on the same scene, every edit is inspectable and undoable, Rust
core, TypeScript gameplay, own model connection, editor works offline. See the
vision/principles in section 1. Present the product coherently and aspirationally,
with no misleading endorsement or performance numbers. The actual private repo
is https://github.com/snowdamiz/incant (links can require GitHub access); PLAN.md
is a real destination for product details. Avoid announcing source is public/open
source or claiming shipped platforms. A nonfunctional Download button is unacceptable.

## Design task

Create a distinctive, professional, memorable product landing page with strong
typography, careful whitespace, responsive composition, and an art-directed hero
that meaningfully expresses game creation. You own visual direction; do not stop
at a generic stack of text cards. Deliver a complete polished page including
navigation, hero, compelling product/editor illustration, feature storytelling,
workflow, grounded FAQ and closing CTA/footer as appropriate to your design.
Any illustrated editor is a conceptual product illustration, not a fabricated
live screenshot. Make that clear in its accessible description.

The supplied brand-reference.png is the director's new purple wisp mascot with
two eyes. Use just the mascot as the brand mark; faithfully reproduce its shape
in local SVG if necessary, not the other wordmarks/reference composition. Another
handoff is finalizing the app mascot concurrently. No image-generation services
or substitute model. All visual work and review must be yours.

Vue 3 + Vite 8 + Tailwind 4 are scaffolded and pinned in website/. Use Composition
API and strict TypeScript, focused components, and actual Tailwind utilities with
the Vite Tailwind plugin. Implement index.html, src/App.vue, src/style.css and any
components/assets you need. main.ts already imports these. Local Inter variable
and JetBrains Mono are installed; use latin-only fonts to control payload, or
request a specific additional dependency in result.md if essential.

## Acceptance

- Desktop 1440px, tablet 768px, mobile 390px and narrow 320px all compose properly
  with no clipped content, horizontal overflow or illegible text.
- All buttons/links work; mobile menu works with keyboard and Escape. Use semantic
  links/buttons, clear focus, a skip link, descriptive image alternatives, correct
  heading order and accessible color contrast. Honor prefers-reduced-motion.
- Functional interaction beyond scrolling (for example keyboard-accessible
  workflow tabs that change an illustration, FAQ disclosure, or similar) when it
  improves the product story. No fake forms, dead video controls or pretend backend.
- Assets/fonts are local, no tracking or external runtime requests. No secrets.
- Relative/base-aware asset URLs must work under /incant/ GitHub Pages subpath.
- Keep initial JS under 100 kB gzip; avoid 3D/webgl dependency bundles. Prefer
  original lightweight SVG/CSS artwork. Keep total deployed static payload sane
  (target below 2 MB), avoid shipping the full reference image or evidence.
- npm run build succeeds with strict TS. Inspect actual rendered browser pixels,
  fix visual issues, record screenshots desktop/mobile and review findings.

## Commands and evidence

Run from website/: npm run build; npm run dev -- --port 5175 --strictPort;
PAGES_BASE_PATH=/incant/ npm run build; npm run preview.
website/node_modules will be available. Chrome is at
/Applications/Google Chrome.app/Contents/MacOS/Google Chrome.
Use @playwright/test (installed) to capture actual screenshots and review them.
Use axe-core (installed) to check accessibility and fix any serious/critical issues.
Do not use snapshot evidence for an engine phase gate. Do not touch desktop apps.

Write handoffs/0005-landing-page/result.md with exact model, changed paths,
commands/results, screenshot paths, visual decisions and limitations. Commit only
scoped files with trailer Built-by: claude. Do not push, publish, merge or approve
phase gates. Astra will integrate and open a director-mergeable PR to main.
