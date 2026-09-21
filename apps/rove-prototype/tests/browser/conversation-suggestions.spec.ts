import { test, expect } from './fixtures';

for (const width of [390, 1440]) {
  test(`conversation suggestions fill drafts without execution at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    await page.goto('/#/sessions');
    await page.getByRole('button', { name: '全局添加' }).click();
    await page.getByRole('menuitem', { name: '新会话', exact: true }).click();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1)).toBe(true);
    const suggestions = page.getByRole('group', { name: '会话建议' });
    await expect(suggestions.getByRole('button')).toHaveCount(5);
    await suggestions.getByRole('button', { name: /建立自己的私人影院/ }).click();
    const input = page.getByRole('textbox', { name: '消息', exact: true });
    await expect(input).toHaveValue('我想建立自己的私人影院，请帮我在自己的设备上安装和配置开源影音软件，并通过 Rove 访问。');
    await expect(input).toBeFocused();
    expect(await page.evaluate(() => JSON.parse(localStorage.getItem('rove-prototype-v1')!).runs.length)).toBe(0);
    await suggestions.getByRole('button', { name: /管理多台代码代理/ }).click();
    await expect(input).toHaveValue('我想建立自己的私人影院，请帮我在自己的设备上安装和配置开源影音软件，并通过 Rove 访问。\n我想管理多台设备上的代码代理，请帮我安装和配置第三方开源代码代理，并通过 Rove 访问这些服务。');
    await page.reload();
    await expect(input).toHaveValue(/私人影院[\s\S]*代码代理/);
    await expect(suggestions.getByRole('button')).toHaveCount(5);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1)).toBe(true);
    await page.getByRole('button', { name: '发送消息', exact: true }).click();
    await expect(suggestions).toHaveCount(0);
    expect(await page.evaluate(() => JSON.parse(localStorage.getItem('rove-prototype-v1')!).runs.length)).toBe(1);
    await page.getByRole('button', { name: '全局添加' }).click();
    await page.getByRole('menuitem', { name: '新会话', exact: true }).click();
    await expect(input).toHaveValue('');
    await expect(suggestions.getByRole('button', { name: /管理多台代码代理/ })).toBeVisible();
  });
}
