import { test, expect, type Page } from '@playwright/test';
import { createMockApi, seed } from '../../src/mock';
import type { Snapshot } from '../../src/domain';

async function install(page: Page, state: Snapshot) {
  await page.addInitScript(snapshot => {
    if (!localStorage.getItem('rove-prototype-v1')) localStorage.setItem('rove-prototype-v1', JSON.stringify(snapshot));
  }, state);
}
async function saved(page: Page) { return page.evaluate(() => JSON.parse(localStorage.getItem('rove-prototype-v1')!)); }
async function noOverflow(page: Page) { expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1)).toBe(true); }

for (const width of [390, 1440]) {
  test(`archive via menu, read only, reload and restore original draft at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    const state = seed('daily'); state.sessions[0]!.draft = '保留这段草稿';
    await install(page, state); await page.goto('/#/sessions');
    const more = page.getByRole('button', { name: '更多操作：让设备为你协作', exact: true });
    if (width === 390) {
      await more.click(); await page.getByRole('menuitem', { name: '归档会话' }).click();
    } else {
      await more.focus(); await page.keyboard.press('Enter');
      await expect(page.getByRole('menu')).toBeFocused();
      await page.keyboard.press('ArrowDown'); await page.keyboard.press('Enter');
    }
    await expect(page.locator('.session-item')).toHaveCount(0);
    await expect(page.locator('.notice')).toContainText('归档不会取消');
    await page.getByRole('button', { name: '归档管理', exact: true }).click();
    await expect(page.getByRole('heading', { name: '归档管理', exact: true })).toBeVisible();
    await noOverflow(page); await page.reload();
    await page.getByRole('button', { name: '查看', exact: true }).click();
    await expect(page.getByText('会话已归档', { exact: true })).toBeVisible();
    await expect(page.getByRole('textbox', { name: '消息', exact: true })).toHaveCount(0);
    await expect(page.getByRole('combobox', { name: '会话模型' })).toHaveCount(0);
    await expect(page.getByRole('button', { name: '发送消息' })).toHaveCount(0);
    await page.getByRole('button', { name: '恢复会话', exact: true }).click();
    await expect(page.getByRole('textbox', { name: '消息', exact: true })).toHaveValue('保留这段草稿');
    await page.reload(); await expect(page.getByRole('textbox', { name: '消息', exact: true })).toHaveValue('保留这段草稿');
    expect((await saved(page)).sessions).toEqual(state.sessions);
    await page.getByRole('button', { name: '归档会话', exact: true }).click();
    await expect(page).toHaveURL(/#\/sessions$/);
    await page.goto('/#/settings'); await page.getByRole('button', { name: '归档管理', exact: true }).click();
    await page.getByRole('button', { name: '恢复', exact: true }).click();
    await expect(page.getByRole('heading', { name: '暂无归档会话' })).toBeVisible();
  });

  test(`confirm deletion, cancel safely, long title and deleted link at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    const state = seed('daily'); state.sessions[0]!.archived_at = 1000; state.sessions[0]!.title = '很长的历史会话'.repeat(18); state.sessions[0]!.draft = '不能误删';
    await install(page, state); await page.goto('/#/archives'); await noOverflow(page);
    await page.getByRole('button', { name: '切换明暗主题' }).click(); await noOverflow(page);
    await page.getByRole('button', { name: '删除', exact: true }).click();
    const dialog = page.getByRole('dialog', { name: '永久删除会话？' });
    await expect(dialog).toContainText(state.sessions[0]!.title);
    await expect(dialog).toContainText('删除后无法恢复');
    await expect(dialog.getByRole('button', { name: '取消', exact: true })).toBeFocused();
    await page.keyboard.press('Escape'); await expect(dialog).toBeHidden();
    expect((await saved(page)).sessions).toEqual(state.sessions);
    await page.getByRole('button', { name: '删除', exact: true }).click();
    await dialog.getByRole('button', { name: '取消', exact: true }).click();
    expect((await saved(page)).sessions).toEqual(state.sessions);
    await page.getByRole('button', { name: '删除', exact: true }).click();
    await dialog.getByRole('button', { name: '永久删除', exact: true }).click();
    await expect(page.getByRole('heading', { name: '暂无归档会话' })).toBeVisible();
    await page.reload(); const result = await saved(page); expect(result.sessions).toHaveLength(0); expect(result.services).toEqual(state.services);
    await page.goto('/#/sessions/welcome'); await expect(page.getByRole('heading', { name: '会话不存在或已删除' })).toBeVisible();
    await page.getByRole('button', { name: '返回会话列表', exact: true }).click(); await expect(page).toHaveURL(/#\/sessions$/);
  });
}

test('archived remote task progresses without resubmission and cannot be deleted until complete', async ({ page }) => {
  let snapshot = ''; const now = Date.now();
  const api = createMockApi({ getItem: () => null, setItem: (_, value) => { snapshot = value; } }, () => now);
  api.submit('welcome', '在书房电脑上安装音乐播放器', 'windows'); api.snapshot(); api.archiveSession('welcome');
  await page.clock.install({ time: now }); await install(page, JSON.parse(snapshot)); await page.goto('/#/archives');
  await expect(page.getByText('任务进行中 · 仍会继续更新')).toBeVisible();
  await page.getByRole('button', { name: '删除', exact: true }).click();
  await expect(page.getByRole('alert')).toContainText('请等待完成'); await expect(page.getByRole('dialog')).toHaveCount(0);
  await page.getByRole('button', { name: '查看', exact: true }).click();
  await expect(page.getByRole('button', { name: '取消任务' })).toHaveCount(0);
  await page.clock.runFor(14000); await expect(page.locator('.bubble-meta')).toContainText('已完成');
  await page.getByRole('region', { name: '会话详情' }).getByRole('button', { name: '归档管理', exact: true }).click();
  await page.getByRole('button', { name: '删除', exact: true }).click();
  await page.getByRole('button', { name: '永久删除', exact: true }).click();
  const result = await saved(page); expect(result.runs).toHaveLength(0); expect(result.sessions).toHaveLength(0); expect(result.services).toHaveLength(4);
});
