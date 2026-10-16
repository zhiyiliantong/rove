// Read-only acceptance of an already installed Windows WebView via a local SSH
// CDP forward. This is not a browser bridge fixture and never calls a model.
import { chromium } from 'playwright';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const expectedScript = readFileSync(new URL('../dist/index.html', import.meta.url), 'utf8')
  .match(/src="(\/assets\/index-[^"]+\.js)"/)?.[1];
assert.ok(expectedScript, 'Build the frontend before checking its installed version');
const browser = await chromium.connectOverCDP(process.env.ROVE_CDP_URL || 'http://127.0.0.1:19223');
try {
  const pages = browser.contexts().flatMap(context => context.pages());
  const page = pages.find(page => page.url().startsWith('http://tauri.localhost'));
  assert.ok(page, 'Installed Tauri window must exist');
  const assets = [];
  page.on('response', response => {
    if (response.url().includes('/assets/')) {
      assets.push({ url: response.url(), status: response.status() });
    }
  });
  // Observe real script loads instead of fetch(): production CSP intentionally
  // restricts connect-src to IPC, so an in-page asset fetch is not permitted.
  await page.reload();
  await page.locator('#app button').first().waitFor({ timeout: 20000 });
  const result = await page.evaluate(async () => {
    const response = await window.__TAURI_INTERNALS__.invoke('agent_call', { request: {
      kind: 'request', correlation_id: crypto.randomUUID(), operation_id: 'get_device',
    } });
    return { url: location.href, title: document.title,
      appChildren: document.querySelector('#app').children.length,
      assetError: document.body.innerText.includes('asset not found:'),
      device: response };
  });
  result.assets = assets;
  assert.equal(result.assetError, false);
  assert.ok(result.appChildren > 0);
  assert.ok(result.assets.some(asset => new URL(asset.url).pathname === expectedScript));
  assert.ok(result.assets.every(asset => asset.status === 200));
  assert.equal(result.device.status_code, 200);
  await page.screenshot({ path: process.env.ROVE_UI_SCREENSHOT || '/tmp/rove-installed-windows-ui.png' });
  console.log(JSON.stringify(result));
} finally {
  await browser.close();
}
