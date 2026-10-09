# Incant website

The product landing page is a standalone Vue 3, Vite 8 and Tailwind CSS 4 project.
It has its own pinned npm lockfile and does not depend on the native editor or
the build-time agent accounts. Run commands from this directory.

```sh
npm ci
npm run dev
```

Production verification, including the GitHub Pages repository subpath:

```sh
PAGES_BASE_PATH=/incant/ npm run build
npx playwright install chromium
PAGES_BASE_PATH=/incant/ npm test
PAGES_BASE_PATH=/incant/ npm run preview
```

For installed Chrome, set `PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH` to its executable
instead of downloading the Playwright browser. `npm run build` includes strict
Vue/TypeScript checking. Browser checks exercise the production build.

## Deployment

`.github/workflows/website.yml` checks pull requests that touch the site. On every
push to `main`, it installs from the lockfile, builds, runs browser checks, uploads
only `website/dist`, and deploys via GitHub's official Pages actions. Manual runs
can publish only from `main`. Pull requests cannot deploy.

The repository's Pages source must be **GitHub Actions**. The expected project
address is https://snowdamiz.github.io/incant/. GitHub Pages availability for a
private repository depends on the repository owner's GitHub plan. Repository
visibility is not changed by this workflow. Only generated public marketing
assets are uploaded; source documents, handoffs and credentials are excluded.

The workflow derives the base path from the repository name. For local builds
or a future custom-domain deployment, use `PAGES_BASE_PATH=/` when building and
testing, and update the workflow's path calculation. All production assets must
continue to use Vite imports or `import.meta.env.BASE_URL`.

This site is independent of the engine phase gates; a deployed landing page is
not evidence of a signed engine release. Public copy follows the director's
requested finished-product presentation.

References: [Vite deployment](https://vite.dev/guide/static-deploy.html#github-pages),
[Tailwind's Vite integration](https://tailwindcss.com/docs/installation/using-vite).
