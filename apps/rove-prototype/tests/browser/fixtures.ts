import { test as base, expect } from '@playwright/test';
import { seed } from '../../src/mock';
// Returning-user tests explicitly load demo data; introduction tests use empty storage.
export const test = base.extend({
  storageState: async ({}, use) => {
    await use({ cookies: [], origins: [{ origin: 'http://127.0.0.1:4173', localStorage: [
      { name: 'rove-prototype-v1', value: JSON.stringify(seed('daily')) },
      { name: 'rove-prototype-introduction-v1', value: JSON.stringify({ version: 1, completed: true, step: 3, resetPending: false }) },
    ] }] });
  },
});
export { expect };
export type { Page } from '@playwright/test';
