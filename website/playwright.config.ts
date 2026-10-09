import { defineConfig } from '@playwright/test'

const base = process.env.PAGES_BASE_PATH ?? '/incant/'
const chrome = process.env.CHROME_PATH ?? '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'

export default defineConfig({
  testDir: './tests',
  fullyParallel: false,
  workers: 1,
  reporter: 'list',
  use: {
    baseURL: `http://127.0.0.1:4175${base}`,
    launchOptions: { executablePath: chrome },
  },
  webServer: {
    command: 'npm run preview',
    env: { PAGES_BASE_PATH: base },
    url: `http://127.0.0.1:4175${base}`,
    reuseExistingServer: false,
    timeout: 30_000,
  },
})
