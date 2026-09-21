import { test, expect } from './fixtures';
import { seed } from '../../src/mock';

for (const width of [320, 390, 1440]) {
  test(`reference-style reply has accessible icon actions and contained blocks at ${width}px`, async ({ browser }) => {
    const context = await browser.newContext({ viewport: { width, height: 900 }, hasTouch: width < 640 });
    const state = seed('daily');
    state.sessions[0]!.messages[0]!.text = '而换成复数：\n\n```text\ne^(inx)\n```\n\n平移就变成单纯乘：\n\n$$\ne^{in(x+a)} = e^{ina}e^{inx}\n$$\n\n所以复指数形式恰好把 **一个频率** 真正变成了一维群表示。\n\n```text\n' + 'long-code-'.repeat(50) + '\n```';
    await context.addInitScript(snapshot => {
      localStorage.setItem('rove-prototype-v1', JSON.stringify(snapshot));
      localStorage.setItem('rove-prototype-introduction-v1', JSON.stringify({version:1,completed:true,step:3,resetPending:false}));
      Object.defineProperty(navigator, 'clipboard', { configurable: true, value: undefined });
    }, state);
    const page = await context.newPage();
    try {
      await page.goto('/#/sessions/welcome');
      const reply = page.locator('.assistant-reply').first();
      await expect(reply.locator('.formula-block .katex')).toHaveCount(1);
      await expect(page.locator('.message.assistant .message-bubble').first()).toHaveCSS('background-color', 'rgba(0, 0, 0, 0)');
      for (const dark of [false, true]) {
        await page.evaluate(value => document.documentElement.classList.toggle('dark', value), dark);
        for (const block of await reply.locator('.code-block, .formula-block').all()) {
          await expect(block).toHaveCSS('border-radius', '22px');
          const box = (await block.boundingBox())!;
          const button = block.getByRole('button');
          const bounds = (await button.boundingBox())!;
          expect(bounds.x + bounds.width).toBeLessThanOrEqual(box.x + box.width);
          expect(bounds.y - box.y).toBeLessThan(15);
          expect(bounds.width).toBeGreaterThanOrEqual(width < 640 ? 44 : 40);
          await expect(button).toHaveText('');
          await expect(button).toHaveAttribute('title', /复制/);
        }
        expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1)).toBe(true);
      }
      const paragraphCopy = reply.locator('.explanation-block > button').first();
      if (width < 640) await expect(paragraphCopy).toHaveCSS('opacity', '1');
      await paragraphCopy.focus();
      await expect(paragraphCopy).toHaveCSS('opacity', '1');
      await page.keyboard.press('Enter');
      await expect(reply.getByRole('textbox', { name: '手动复制文本' })).toHaveValue('而换成复数：');
      await reply.getByRole('button', { name: '关闭手动复制' }).click();
      await expect(reply.getByRole('textbox', { name: '手动复制文本' })).toHaveCount(0);
      await expect(reply).not.toContainText('剪贴板不可用');
      const actions = reply.getByRole('group', { name: '回复操作' });
      await expect(actions.getByRole('button')).toHaveCount(2);
      await expect(actions).toHaveText('');
      await expect(actions.getByRole('button', { name: '朗读回复' })).toHaveAttribute('aria-pressed', 'false');
    } finally { await context.close(); }
  });
}

test('reply streams into Markdown/math, supports granular copies and explicit local speech', async ({ page }) => {
  const external: string[] = [];
  page.on('request', r => { if (r.url().startsWith('http') && new URL(r.url()).origin !== 'http://127.0.0.1:4173') external.push(r.url()); });
  await page.addInitScript(() => {
    const state = { copied: '', spoken: '', stopped: 0 }; (window as any).__replyTest = state;
    Object.defineProperty(navigator, 'clipboard', { configurable: true, value: { writeText: async (s: string) => { state.copied = s; } } });
    Object.defineProperty(window, 'speechSynthesis', { configurable: true, value: { getVoices: () => [{ localService: true, lang: 'zh-CN' }], speak: (u: any) => { state.spoken = u.text; }, cancel: () => { state.stopped++; } } });
    Object.defineProperty(window, 'SpeechSynthesisUtterance', { configurable: true, value: class { text: string; constructor(s: string) { this.text = s; } } });
  });
  await page.setViewportSize({ width: 390, height: 844 }); await page.goto('/#/sessions/welcome');
  await page.getByRole('textbox', { name: '消息', exact: true }).fill('演示富文本和公式'); await page.getByRole('button', { name: '发送消息' }).click();
  const bubble = page.locator('.task-message'); await expect(bubble.locator('.streaming-label')).toBeVisible();
  await expect.poll(async () => (await bubble.locator('.rich-text').textContent())?.length ?? 0).toBeGreaterThan(15);
  const prefix = await page.evaluate(() => JSON.parse(localStorage.getItem('rove-prototype-v1')!).runs[0].output_text.length);
  await expect.poll(async () => page.evaluate(() => JSON.parse(localStorage.getItem('rove-prototype-v1')!).runs[0].output_text.length)).toBeGreaterThan(prefix);
  await expect(bubble).toContainText('已完成', { timeout: 20000 });
  await expect(bubble.getByRole('heading', { name: '富文本演示' })).toBeVisible(); await expect(bubble.locator('.formula-block .katex')).toHaveCount(1);
  await bubble.locator('.formula-block').getByRole('button', { name: /复制公式/ }).click(); expect(await page.evaluate(() => (window as any).__replyTest.copied)).toBe('A = \\pi r^2');
  await bubble.locator('.code-block').getByRole('button', { name: /复制代码/ }).click(); expect(await page.evaluate(() => (window as any).__replyTest.copied)).toContain('let area');
  await bubble.locator('.explanation-block').first().getByRole('button', { name: /复制说明/ }).click(); expect(await page.evaluate(() => (window as any).__replyTest.copied)).toContain('Markdown');
  await bubble.getByRole('button', { name: '复制回复', exact: true }).click(); expect(await page.evaluate(() => (window as any).__replyTest.copied)).toContain('$$');
  expect(await page.evaluate(() => (window as any).__replyTest.spoken)).toBe('');
  await bubble.getByRole('button', { name: '朗读回复' }).click(); expect(await page.evaluate(() => (window as any).__replyTest.spoken)).toContain('富文本演示');
  await bubble.getByRole('button', { name: '停止朗读' }).click(); expect(await page.evaluate(() => (window as any).__replyTest.stopped)).toBeGreaterThan(1);
  await expect.poll(() => page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1)).toBe(true); expect(external).toEqual([]);
});
test('unavailable clipboard and speech offer truthful fallbacks', async ({ page }) => {
  await page.addInitScript(() => { Object.defineProperty(navigator, 'clipboard', { configurable: true, value: undefined }); Object.defineProperty(window, 'speechSynthesis', { configurable: true, value: { getVoices: () => [] } }); });
  await page.goto('/#/sessions/welcome');
  const reply = page.locator('.assistant-reply').first();
  await reply.getByRole('button', { name: '复制回复' }).click(); await expect(reply.getByRole('textbox', { name: '手动复制文本' })).toBeVisible();
  await reply.getByRole('button', { name: '朗读回复' }).click(); await expect(reply).toContainText('没有可用的本地朗读语音');
});
