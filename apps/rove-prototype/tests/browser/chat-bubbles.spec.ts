import { test, expect } from '@playwright/test';
test('chat uses left/right bubbles and inline job states with details and cancellation, never progress bars', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto('/#/sessions/welcome');
  const composer = page.getByRole('textbox', { name: '消息', exact: true });
  await composer.fill('查询服务'); await page.getByRole('button', { name: '发送消息' }).click();
  const task = page.locator('.task-message'); await expect(task).toHaveCount(1);
  await expect(task.locator('.message-bubble')).toBeVisible();
  await expect(page.getByRole('progressbar')).toHaveCount(0); await expect(page.locator('.run-card')).toHaveCount(0);
  await expect(page.locator('.message.user')).toHaveCSS('flex-direction', 'row-reverse');
  await expect(task).toHaveCSS('flex-direction', 'row');
  const preceding = await task.evaluate(el => el.previousElementSibling?.classList.contains('user')); expect(preceding).toBe(true);
  await task.locator('summary').click(); await expect(task.locator('.execution-details')).toContainText('任务 ID');
  await task.getByRole('button', { name: '取消任务', exact: true }).click();
  await expect(task).toContainText('已取消'); await page.reload(); await expect(task).toHaveCount(1); await expect(task).toContainText('已取消');
  await expect.poll(() => page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1)).toBe(true);
});
