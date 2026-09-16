import { defineConfig } from '@playwright/test';
export default defineConfig({
  testDir: './tests/browser', timeout: 30000, fullyParallel: true, workers: 2,
  use: { baseURL: 'http://127.0.0.1:4173', headless: true, trace: 'retain-on-failure', screenshot: 'only-on-failure', launchOptions: process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH ? { executablePath: process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH } : {} },
  webServer: { command: 'npm run dev', url: 'http://127.0.0.1:4173', reuseExistingServer: process.env.PLAYWRIGHT_REUSE_SERVER === '1', timeout: 30000 },
});
