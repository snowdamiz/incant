# Result: 0005 landing page

## Status

Complete and ready for Astra integration. Not pushed, published or merged. No phase gate is
claimed or approved by this handoff.

## Model

Claude Opus 5.5 (`claude-opus-5-5`), running in Claude Code with the director's Claude
account. No image-generation service or substitute model was used. All artwork is hand-written SVG/CSS.

## Director feedback applied

- **"Imply finished release":** the public copy uses a confident, present-tense product voice. It contains no Phase 0, gate, roadmap, beta, "coming soon" or internal-tracking language. A grep audit of `website/src` and `index.html` confirms this.
- **No fabrication:** there are no customer counts, quotes, awards, pricing, version numbers, installers, signup forms, video or demo.
- **Genuine destinations only:** the CTAs point to the private repository or its PLAN.md ("product guide"). Every outbound link carries screen-reader text saying it is on GitHub and may require access. The page has no Download button.
- **Hedged claims:** the page does not claim public or open source or any shipped platform list. It states no performance numbers.

## Changed paths

All changes are inside the allowed scope.

- `website/index.html`: meta tags, a favicon with a base-aware URL and a noscript fallback.
- `website/public/favicon.svg`: the wisp mascot.
- `website/src/style.css`: Tailwind 4 `@theme` design tokens, latin-only local fonts, focus ring and reduced motion.
- `website/src/App.vue`: skip link and section composition.
- `website/src/content.ts`: links, nav, workflow steps and FAQ copy.
- `website/src/components/`:
  - `WispMark.vue`
  - `SiteHeader.vue`
  - `HeroSection.vue`
  - `EditorIllustration.vue`
  - `GameScene.vue`
  - `FeatureCard.vue`
  - `FeatureSection.vue`
  - `WorkflowSection.vue`
  - `WorkflowVisual.vue`
  - `TrustSection.vue`
  - `FaqSection.vue`
  - `ClosingSection.vue`
- `website/playwright.config.ts` and `website/tests/landing.spec.ts`: evidence capture and behavioural checks.
- `handoffs/0005-landing-page/result.md` and `handoffs/0005-landing-page/screenshots/*.png`.

`website/.gitignore` shows a one-character change (`node_modules/` became `node_modules`) that this session did not make. It was left uncommitted.

No CI workflow was added. Pages deployment on push to main is Astra's integration task, and workflow files are outside this handoff's allowed paths.

## Commands and results

These were run from `website/` on 2026-10-08.

| Command | Result |
|---|---|
| `npm run build` | Pass. `vue-tsc --noEmit` is strict and clean, and the Vite build succeeds. |
| `PAGES_BASE_PATH=/incant/ npm run build` | Pass. `dist/index.html` references `/incant/assets/...` and `/incant/favicon.svg`. |
| `npm run dev -- --port 5175 --strictPort` | HTTP 200 on `http://127.0.0.1:5175/`. |
| `npx playwright test` | **10 passed.** It runs `npm run preview` with `/incant/` in system Chrome. |

Payload of the `/incant/` build:

| Asset | Raw | Gzip |
|---|---|---|
| JS (Vue + app, the only JS) | 119.8 kB | **43.0 kB** (budget 100 kB) |
| CSS | 52.7 kB | 9.5 kB |
| Inter latin variable woff2 | 48.3 kB | n/a |
| JetBrains Mono latin variable woff2 | 40.4 kB | n/a |
| Total `dist/` | **268 kB** on disk | (target below 2 MB) |

The brand reference PNG and the screenshots are not shipped in `dist/`.

### What the Playwright suite verifies

These are real-browser checks against the built preview, not mocks.

1. At 1440, 768, 390 and 320 the page has no horizontal overflow, and every request stays on `127.0.0.1`.
2. axe-core 4.14 at 1440 and 390 reports **zero violations of any impact**.
3. Tab reveals the skip link and Enter moves focus to `<main>`. There is exactly one h1 and no skipped heading levels. Every `#hash` link resolves, and every external href is the repo or PLAN.md.
4. The mobile menu opens from the keyboard, focuses its first link and sets `aria-expanded`. Escape closes it and returns focus to the toggle, and choosing a link closes it and navigates.
5. Workflow tabs follow the ARIA tabs pattern. Arrow keys move both focus and selection, Home and End work, and the panel and illustration swap.
6. FAQ `<details>` disclosures toggle from the keyboard.
7. With `prefers-reduced-motion: reduce`, every animation runs only once at near-zero duration.

## Screenshots

These are real Chrome captures in `handoffs/0005-landing-page/screenshots/`.

- `desktop-1440-full.png` and `desktop-1440-fold.png`
- `tablet-768-full.png` and `tablet-768-fold.png`
- `mobile-390-full.png`, `mobile-390-fold.png` and `mobile-390-menu-open.png`
- `narrow-320-full.png` and `narrow-320-fold.png`
- `workflow-describe-1440.png`, `workflow-inspect-1440.png`, `workflow-playtest-1440.png` and `workflow-rewind-1440.png`

## Visual direction and design system

- **Mood:** "spellbook at dusk." The page sits on ink-violet night neutrals, lit by the wisp's violet. Warm ember marks the game world and human edits, and mint "spell" marks validated or applied state. Each colour has one meaning, and the editor illustration, history chips and diffs reuse it.
- **Brand mark:** `WispMark.vue` is a single-path SVG redraw of the director's mascot, used alone without the wordmark. It was checked by overlaying it on `brand-reference.png` and matches closely. It appears in the nav, footer, favicon, the floating hero mascot that blinks unless motion is reduced, the agent avatar and the closing CTA.
- **Typography:** Inter Variable at tight tracking for display, set from 2.6rem at 320px up to 5.4rem at desktop. JetBrains Mono carries the "engine voice": eyebrows, commands and code.
- **Hero:** the headline is "Say the word. Build the world." Below it is an art-directed conceptual editor window. It has a hierarchy, a dusk viewport with a grapple line to a crystal ledge, and an agent panel with a four-command transaction. Its accessible label and visible caption both state it is a conceptual illustration, not a screenshot.
- **Story order:**
  1. A features bento covering the command bus, text-native documents, shared senses, history and undo, your model and keys, and Rust with TypeScript.
  2. An interactive four-step workflow: Describe, Inspect, Play-test, Rewind.
  3. Trust rules drawn from PLAN.md: offline by default, project content treated as data, no agent shell, and capability grants.
  4. A plain FAQ.
  5. The closing CTA and footer.
- **States and accessibility:** a 2px violet focus-visible ring is used everywhere, and tap targets are at least 44px. Muted text is `#b4afcc` on `#08070f`, about 9:1 contrast. Long code scrolls inside a focusable region instead of overflowing the page. A noscript fallback is included. Loading and error states do not apply because there is no data fetching.

## Rendered review findings fixed

- **Mascot overlap:** the hero mascot covered the window chrome's Edit and Play chips. It was repositioned per breakpoint, beside the window on xl and sitting on its edge below that.
- **Clipped card at 320px:** the scripting card's implicit grid column grew to the code width, so the card's overflow clipping cut off its text. It now uses `grid-cols-1` with `minmax(0,…)` columns and min-w-0 children, and the code scrolls internally.
- **Hero pill at 320px:** it wrapped into two awkward lines, so the secondary text is now hidden below 380px.
- **Tablet bento hole:** the 768px grid left an empty slot, which is now fixed.
- **Small fixes:**
  - The closing CTA button wrapped at 320px.
  - The workflow height jumped between tabs.
  - The open mobile menu was translucent over the hero headline.
  - The gap between the hero and the features section was oversized.

## Limitations and open questions

- **Unverified artifact:** at 320px, Playwright's clipped full-page captures showed a dark rectangle over the illustration's left edge. A normal scrolled viewport capture at the same spot is clean, and hit-testing finds no element there. It is treated as a capture artifact, but not proven to be one.
- **Not tested:** Firefox, Safari and real devices. Lighthouse and visual-regression baselines were not run.
- **Mascot sync:** another handoff is finalizing the app mascot. If its final geometry differs, replace the path in `WispMark.vue` and `public/favicon.svg`.
- **Private destinations:** both CTAs lead to a private repo. Visitors without access will get a GitHub 404 or a sign-in page. The director may want a public destination later.
- **Illustrative content:** the editor UI, the grapple example, the TypeScript API shown (`behavior`, `ctx.pull`) and the scene document syntax are illustrative, not the shipped API. They should be aligned once the real APIs exist.
- **Out of scope here:** no Open Graph image was added because it would need an absolute deploy URL. The GitHub Pages workflow is for Astra to add.
