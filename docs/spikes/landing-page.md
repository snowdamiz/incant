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

## Hosting prerequisite

The authenticated GitHub Pages creation call for `snowdamiz/incant`, with
`build_type=workflow`, returned HTTP 422:

> Your current plan does not support GitHub Pages for this repository.

The repository is private. No Pages site was created, and no billing or visibility
setting was changed. A director decision to use a Pages-eligible GitHub plan or
make the repository public is required before Pages can be enabled. After that,
set the Pages source to GitHub Actions and merge the PR; the main push deploys it.
The expected URL is `https://snowdamiz.github.io/incant/`. It is not a verified live
site while this prerequisite remains unresolved.

References: [Vite GitHub Pages deployment](https://vite.dev/guide/static-deploy.html#github-pages),
[Tailwind Vite integration](https://tailwindcss.com/docs/installation/using-vite).
