import { test, expect } from '@playwright/test';
import { seed } from '../../src/mock';
import { manageRovePrompt } from '../../src/conversation-prompts';

for (const width of [320, 1440]) {
  test(`management entries prepare a new local draft without disturbing existing work at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    const state = seed('daily');
    state.sessions[0]!.draft = '保留我的旧草稿';
    state.sessions[0]!.target_device_id = 'windows';
    await page.addInitScript(snapshot => {
      if (!localStorage.getItem('rove-prototype-v1')) localStorage.setItem('rove-prototype-v1', JSON.stringify(snapshot));
    }, state);
    await page.goto('/#/sessions/welcome');
    await page.getByRole('button', { name: /^管理 Rove/ }).click();
    const input = page.getByRole('textbox', { name: '消息', exact: true });
    await expect(input).toHaveValue(manageRovePrompt);
    await expect(input).toBeFocused();
    await expect(page.locator('.conversation-header')).toContainText('执行设备：此设备');
    await expect(page.locator('.notice')).toContainText('不会扫描磁盘');
    let saved = await page.evaluate(() => JSON.parse(localStorage.getItem('rove-prototype-v1')!));
    expect(saved.sessions.find((s: any) => s.id === 'welcome').draft).toBe('保留我的旧草稿');
    expect(saved.sessions.find((s: any) => s.id === 'welcome').target_device_id).toBe('windows');
    expect(saved.runs).toHaveLength(0);
    expect(saved.services).toEqual(state.services);
    const firstUrl = page.url();
    await page.goto('/#/settings');
    const banner = page.locator('.manage-rove-card');
    await expect(banner).toContainText('设置不必自己找');
    await expect(banner).toContainText('再由你确认');
    await expect(page.getByRole('button', { name: '管理模型', exact: true })).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1)).toBe(true);
    await page.getByRole('button', { name: '通过对话管理', exact: true }).click();
    expect(page.url()).not.toBe(firstUrl);
    await expect(input).toHaveValue(manageRovePrompt);
    await expect(page.locator('.conversation-header')).toContainText('执行设备：此设备');
    await page.reload();
    await expect(input).toHaveValue(manageRovePrompt);
    saved = await page.evaluate(() => JSON.parse(localStorage.getItem('rove-prototype-v1')!));
    expect(saved.sessions).toHaveLength(3);
    expect(saved.runs).toHaveLength(0);
    expect(saved.services).toEqual(state.services);
    expect(saved.max_active_runs).toBe(state.max_active_runs);
  });
}

test('management without models explains setup and leaves manual settings available', async ({ page }) => {
  await page.addInitScript(snapshot => localStorage.setItem('rove-prototype-v1', JSON.stringify(snapshot)), seed('empty'));
  await page.goto('/#/settings');
  await page.getByRole('button', { name: '通过对话管理', exact: true }).click();
  await expect(page.getByRole('dialog', { name: '添加模型' })).toBeVisible();
  await expect(page.locator('.notice')).toContainText('请先添加模型');
  expect(await page.evaluate(() => JSON.parse(localStorage.getItem('rove-prototype-v1')!).sessions.length)).toBe(0);
  await page.keyboard.press('Escape');
  await expect(page.getByRole('button', { name: '管理模型', exact: true })).toBeVisible();
  await expect(page.getByRole('heading', { name: '外观', exact: true })).toBeVisible();
});
