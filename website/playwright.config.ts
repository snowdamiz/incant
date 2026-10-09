import { defineConfig } from '@playwright/test'

const basePath = process.env.PAGES_BASE_PATH || '/'

export default defineConfig({
  testDir: './tests',
  fullyParallel: true,
  workers: 2,
  forbidOnly: Boolean(process.env.CI),
  retries: process.env.CI ? 1 : 0,
  reporter: 'list',
  use: {
    baseURL: `http://127.0.0.1:4175${basePath}`,
    browserName: 'chromium',
    launchOptions: {
      executablePath: process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH,
    },
    trace: 'retain-on-failure',
  },
  webServer: {
    command: 'npm run preview',
    env: { PAGES_BASE_PATH: basePath },
    url: `http://127.0.0.1:4175${basePath}`,
    reuseExistingServer: false,
    timeout: 30_000,
  },
})
