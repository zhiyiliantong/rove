import { test, expect } from './fixtures';
test.beforeEach(async ({ page }) => { page.on('dialog', dialog => dialog.accept()); await page.goto('/'); });
test('first model batch starts onboarding only once and exports a demo card', async ({ page }) => {
  await page.goto('/#/settings');
  await page.getByRole('combobox', { name: '演示场景' }).click();
  await page.getByRole('option', { name: '首次使用（空状态）' }).click();
  await page.getByRole('button', { name: '开始', exact: true }).click();
  await page.getByRole('button', { name: '添加模型', exact: true }).click();
  await page.getByRole('button', { name: '模拟验证并获取型号' }).click();
  await page.getByRole('button', { name: '保存模型' }).click();
  await expect(page.getByRole('heading', { name: '让设备，彼此相连' })).toBeVisible();
  await page.getByRole('button', { name: '创建网络', exact: true }).click();
  await page.getByLabel('网络名称', { exact: true }).fill('我们的网络');
  await page.getByRole('button', { name: '保存网络' }).click();
  await expect(page.getByRole('dialog', { name: '网络名片' })).toBeVisible();
  await expect(page.getByAltText('演示网络名片二维码')).toBeVisible();
  const url = await page.getByLabel('加入网络 URL', { exact: true }).inputValue();
  expect(url).toContain('rove.example.invalid');
  expect(url).not.toContain('credential_ref');
  await page.keyboard.press('Escape');
  await page.reload();
  await expect(page.getByRole('heading', { name: '让设备，彼此相连' })).toBeVisible();
  await expect(page.locator('.intro-network-status')).toContainText('已保存 1 个演示网络');
  expect(await page.evaluate(() => JSON.parse(localStorage.getItem('rove-prototype-v1')!).sessions.filter((s: any) => s.onboarding).length)).toBe(1);
});
test('remote task persists across reload; offline state does not duplicate submission', async ({ page }) => {
  await page.goto('/#/sessions/welcome');
  await page.getByRole('textbox', { name: '消息', exact: true }).fill('在书房电脑上安装音乐播放器');
  await page.getByRole('button', { name: '发送消息' }).click();
  await expect(page.locator('.task-message')).toHaveCount(1);
  await page.reload();
  await expect(page.locator('.task-message')).toHaveCount(1);
  await expect(page.locator('.task-message')).toContainText('已完成', { timeout: 20000 });
  await page.goto('/#/services');
  await expect(page.getByRole('heading', { name: '新音乐服务', exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: /删除|编辑|部署|启停/ })).toHaveCount(0);
});
test('default model radio, connection edit, theme, and network switching', async ({ page }) => {
  await page.goto('/#/models');
  await page.locator('input[type=radio]').nth(1).check();
  await expect(page.locator('input[type=radio]').nth(1)).toBeChecked();
  await page.getByRole('button', { name: '编辑连接' }).click();
  await page.getByLabel('配置名称').fill('我的连接');
  await page.getByRole('button', { name: '保存模型' }).click();
  await expect(page.getByRole('heading', { name: '我的连接', exact: true })).toBeVisible();
  await page.getByRole('button', { name: '切换明暗主题' }).click();
  await expect(page.locator('html')).toHaveClass('dark');
  await page.goto('/#/networks/studio');
  await expect(page.locator('.network-context')).toHaveText('我的漫游空间');
  await page.goto('/#/networks');
  await page.getByRole('button', { name: '连接', exact: true }).click();
  await expect(page.locator('.network-context')).toHaveText('2 个网络已连接');
});
test('real API key is rejected and never persisted', async ({ page }) => {
  await page.getByRole('button', { name: '全局添加' }).click();
  await page.getByRole('menuitem', { name: '添加模型' }).click();
  await page.getByLabel('API 密钥（仅 demo-key）').fill('secret-must-not-persist');
  await page.getByRole('button', { name: '模拟验证并获取型号' }).click();
  await expect(page.getByRole('alert')).toContainText('demo-key');
  expect(await page.evaluate(() => JSON.stringify(localStorage))).not.toContain('secret-must-not-persist');
  await page.keyboard.press('Escape');
  await expect(page.getByRole('dialog')).toHaveCount(0);
});
test('offline device preserves draft and network import reports errors', async ({ page }) => {
  await page.goto('/#/settings');
  await page.getByRole('combobox', { name: '演示场景' }).click();
  await page.getByRole('option', { name: '设备离线', exact: true }).click();
  await page.goto('/#/sessions/welcome');
  await page.getByRole('textbox', { name: '消息', exact: true }).fill('在书房电脑上安装音乐播放器');
  await page.getByRole('button', { name: '发送消息' }).click();
  await expect(page.getByRole('alert')).toContainText('任务尚未提交');
  await expect(page.getByRole('textbox', { name: '消息', exact: true })).toHaveValue('在书房电脑上安装音乐播放器');
  await expect(page.locator('.task-message')).toHaveCount(0);
  await page.getByRole('button', { name: '全局添加' }).click();
  await page.getByRole('menuitem', { name: '加入网络' }).click();
  await page.getByLabel('演示网络 URL / JSON 配置').fill('invalid');
  await page.getByRole('button', { name: '导入并连接' }).click();
  await expect(page.locator('.form-error')).toContainText('仅支持本原型');
});
for (const width of [320, 360, 390, 640, 768, 1024, 1440]) {
  test(`responsive navigation and no document overflow at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    for (const route of ['/sessions', '/sessions/welcome', '/services', '/networks', '/networks/home', '/models', '/settings']) {
      await page.goto(`/#${route}`);
      await expect(page.locator('#main-content')).toBeVisible();
      await page.waitForTimeout(150);
      const overflow = await page.evaluate(() => document.documentElement.scrollWidth > window.innerWidth + 1);
      expect(overflow, route).toBe(false);
    }
    await page.goto('/#/services');
    await page.screenshot({ path: `test-results/services-${width}.png`, fullPage: true });
  });
}
test('resize keeps draft, target, and platform independent; no external requests', async ({ page }) => {
  const external: string[] = [];
  const errors: string[] = [];
  page.on('request', req => { if (req.url().startsWith('http') && !req.url().startsWith('http://127.0.0.1:4173')) external.push(req.url()); });
  page.on('pageerror', error => errors.push(error.message));
  await page.goto('/#/sessions/welcome');
  await page.getByRole('textbox', { name: '消息', exact: true }).fill('保留草稿和执行目标');
  await expect(page.getByRole('combobox', { name: '执行设备', exact: true })).toHaveCount(0);
  for (const width of [1440, 900, 850, 700, 641, 640, 390, 320, 768]) {
    await page.setViewportSize({ width, height: width === 768 ? 390 : 900 });
    await expect(page.getByRole('textbox', { name: '消息', exact: true })).toHaveValue('保留草稿和执行目标');
    await expect(page.getByRole('combobox', { name: '会话模型', exact: true })).toContainText('demo-chat');
    await expect.poll(() => page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1), { message: `overflow at ${width}px` }).toBe(true);
  }
  await page.getByRole('combobox', { name: '外观预览' }).click();
  await page.getByRole('option', { name: '桌面外观', exact: true }).click();
  await page.setViewportSize({ width: 390, height: 844 });
  await expect(page.getByRole('button', { name: '模拟关闭' })).toBeVisible();
  await page.addStyleTag({ content: 'html { font-size: 20px; }' });
  await expect.poll(() => page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1)).toBe(true);
  expect(external).toEqual([]); expect(errors).toEqual([]);
});
test('keyboard skip link, modal escape, and long labels remain usable on mobile', async ({ page }) => {
  await page.keyboard.press('Tab');
  await expect(page.getByRole('link', { name: '跳到主要内容' })).toBeFocused();
  await page.keyboard.press('Enter');
  await expect(page.locator('#main-content')).toBeFocused();
  expect(page.url()).not.toContain('#main-content');
  await page.getByRole('button', { name: '全局添加' }).click();
  await page.getByRole('menuitem', { name: '添加模型' }).click();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await page.evaluate(() => { const data = JSON.parse(localStorage.getItem('rove-prototype-v1')!); const long = 'averylongname'.repeat(18); data.connections[0].name = long; data.connections[0].base_url = `https://example.invalid/${long}`; data.models[0].model = long; data.networks[0].name = long; data.services[0].name = long; data.services[0].address = `https://example.invalid/${long}`; data.devices[0].name = long; localStorage.setItem('rove-prototype-v1', JSON.stringify(data)); });
  await page.setViewportSize({ width: 320, height: 844 });
  await page.reload();
  for (const route of ['/models', '/networks', '/networks/home', '/services']) { await page.goto(`/#${route}`); await page.waitForTimeout(200); await expect.poll(() => page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1), { message: route }).toBe(true); }
});
test('touch navigation and form controls on mobile browser', async ({ browser, storageState }) => {
  const context = await browser.newContext({ storageState, viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true, baseURL: 'http://127.0.0.1:4173' });
  try {
    const page = await context.newPage();
    await page.goto('/');
    await page.locator('.bottom-nav').getByRole('link', { name: '服务', exact: true }).tap();
    await expect(page.getByRole('heading', { name: '你的服务，随处可达。' })).toBeVisible();
    await page.getByRole('button', { name: '打开服务', exact: true }).first().tap();
    await expect(page.getByRole('dialog', { name: '服务访问演示' })).toBeVisible();
    await page.getByRole('button', { name: '返回 Rove' }).tap();
    await page.getByRole('button', { name: '全局添加' }).tap();
    await page.getByRole('menuitem', { name: '添加模型' }).tap();
    await page.getByRole('button', { name: '模拟验证并获取型号' }).tap();
    await page.getByRole('checkbox', { name: /GPT-5.6 Luna/ }).uncheck();
    await page.getByRole('button', { name: '保存模型' }).tap();
    await expect(page.getByRole('heading', { name: '选一个合拍的模型。' })).toBeVisible();
  } finally { await context.close(); }
});
