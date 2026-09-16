import { test, expect, type Page } from '@playwright/test';
async function openModels(page: Page) {
  await page.goto('/#/models'); await page.getByRole('button', { name: '添加模型', exact: true }).click();
}
async function choose(page: Page, query: string, label: string) {
  await page.getByRole('combobox', { name: '服务商', exact: true }).click();
  await page.getByRole('searchbox', { name: '搜索服务商', exact: true }).fill(query);
  await page.getByRole('option', { name: label, exact: true }).click();
}
test('search providers in Chinese and English, update defaults and clear old authentication/selection', async ({ page }) => {
  await openModels(page);
  await page.getByRole('radio', { name: '账号登录', exact: true }).check();
  await page.getByRole('button', { name: '模拟 Codex 账号登录' }).click();
  await choose(page, '智谱', 'Z.ai（GLM / 智谱国际站）');
  await expect(page.getByLabel('接口地址', { exact: true })).toHaveValue('https://api.z.ai/api/paas/v4');
  await expect(page.locator('.provider-note')).toContainText('国际站');
  await expect(page.getByRole('radio', { name: 'API 密钥', exact: true })).toBeChecked();
  await expect(page.getByRole('radio', { name: '账号登录', exact: true })).toBeDisabled();
  await expect(page.locator('.catalog-list input:checked')).toHaveCount(1);
  await expect(page.getByText('演示认证完成', { exact: false })).toHaveCount(0);
  await choose(page, 'MINIMAX', 'MiniMax');
  await expect(page.getByLabel('接口地址', { exact: true })).toHaveValue('https://api.minimax.io/v1');
  await page.getByRole('checkbox', { name: /MiniMax M2.7/ }).check();
  await expect(page.getByRole('textbox', { name: '模型名称 MiniMax-M2.7', exact: true })).toHaveValue('minimax-m2.7-01');
  await page.getByLabel('接口地址', { exact: true }).fill('https://proxy.example.invalid/v1');
  await expect(page.locator('.catalog-list input:checked')).toHaveCount(1);
  await page.getByRole('checkbox', { name: /MiniMax M2.7/ }).check();
  await page.getByRole('button', { name: '保存模型', exact: true }).click(); await page.reload();
  await expect(page.locator('.model-row').filter({ hasText: 'minimax-m2.7-01' })).toBeVisible();
  const stored = await page.evaluate(() => JSON.parse(localStorage.getItem('rove-prototype-v1')!));
  expect(stored.models.some((m: { model: string }) => m.model === 'MiniMax-M2.7')).toBe(true);
  expect(stored.connections.some((c: { base_url: string }) => c.base_url === 'https://proxy.example.invalid/v1')).toBe(true);
});

test('narrow-screen searchable list has empty state, local presets and zero external requests', async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 740 });
  const external: string[] = [], errors: string[] = [];
  page.on('request', r => { if (r.url().startsWith('http') && new URL(r.url()).origin !== 'http://127.0.0.1:4173') external.push(r.url()); });
  page.on('pageerror', e => errors.push(e.message));
  await openModels(page);
  await page.getByRole('combobox', { name: '服务商', exact: true }).click();
  await expect(page.getByRole('option')).toHaveCount(12);
  for (const label of ['阿里云百炼（Qwen）', '火山方舟（豆包 / Doubao）', '硅基流动（SiliconFlow）', '自定义 / 本地接口', '智谱（GLM）']) {
    await expect(page.getByRole('option', { name: label, exact: true })).toHaveCount(0);
  }
  await page.getByRole('searchbox', { name: '搜索服务商', exact: true }).fill('no-such-vendor');
  await expect(page.getByRole('option', { name: '没有匹配的 Rig 供应商，请更换关键词', exact: true })).toBeVisible();
  await page.getByRole('searchbox', { name: '搜索服务商', exact: true }).fill('月之暗面');
  await expect(page.getByRole('option', { name: 'Moonshot（月之暗面 / Kimi）', exact: true })).toBeVisible();
  await expect.poll(() => page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1)).toBe(true);
  await page.getByRole('option', { name: 'Moonshot（月之暗面 / Kimi）', exact: true }).click();
  await expect(page.locator('.provider-note')).toContainText('国内开放平台');
  await choose(page, 'ollama', 'Ollama（本地）');
  await expect(page.getByLabel('接口地址', { exact: true })).toHaveValue('http://localhost:11434');
  await expect(page.locator('.rig-provider-note')).toContainText('Rig 0.42.0 / ollama');
  await expect(page.locator('.provider-note')).toContainText('不代表已经安装');
  await page.getByLabel('API 密钥（仅 demo-key）', { exact: true }).fill('');
  await page.getByRole('button', { name: '模拟验证并获取型号' }).click();
  await expect(page.getByText('模拟获取结果', { exact: false })).toBeVisible();
  await page.getByRole('button', { name: '保存模型', exact: true }).click();
  await expect(page.locator('.model-row').filter({ hasText: 'ollama-qwen3-8b-01' })).toBeVisible();
  expect(external).toEqual([]); expect(errors).toEqual([]);
});

test('retired provider connections remain editable without changing identity or reappearing in Add Model', async ({ page }) => {
  await page.goto('/#/models');
  await page.evaluate(() => {
    const snapshot = JSON.parse(localStorage.getItem('rove-prototype-v1')!);
    snapshot.connections[0].provider = 'zhipu';
    snapshot.connections[0].base_url = 'https://open.bigmodel.cn/api/paas/v4/';
    localStorage.setItem('rove-prototype-v1', JSON.stringify(snapshot));
  });
  await page.reload();
  await page.getByRole('button', { name: '编辑连接', exact: true }).click();
  await expect(page.getByRole('combobox', { name: '服务商', exact: true })).toContainText('智谱（GLM）（旧配置）');
  await expect(page.getByText('此供应商已不在新增目录中', { exact: false })).toBeVisible();
  await expect(page.getByLabel('接口地址', { exact: true })).toHaveValue('https://open.bigmodel.cn/api/paas/v4/');
  await page.getByLabel('配置名称', { exact: true }).fill('旧模型连接');
  await page.getByRole('button', { name: '保存模型', exact: true }).click();
  await page.reload();
  const stored = await page.evaluate(() => JSON.parse(localStorage.getItem('rove-prototype-v1')!));
  expect(stored.connections[0]).toMatchObject({ provider: 'zhipu', name: '旧模型连接', base_url: 'https://open.bigmodel.cn/api/paas/v4/', credential_ref: 'demo-only' });
  expect(stored.models[0].name).toBe('custom-demo-chat-01');
  await page.getByRole('button', { name: '添加模型', exact: true }).click();
  await page.getByRole('combobox', { name: '服务商', exact: true }).click();
  await expect(page.getByRole('option')).toHaveCount(12);
  await expect(page.getByRole('option', { name: /旧配置/ })).toHaveCount(0);
});
