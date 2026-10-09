# Incant product landing page

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
