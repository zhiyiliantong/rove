import { test, expect, type Page } from '@playwright/test';
import { seed } from '../../src/mock';
async function step(page: Page, name: string) { await page.getByRole('navigation', {name:'引导步骤'}).getByRole('button', {name}).click(); }
async function snapshot(page: Page) { return page.evaluate(() => JSON.parse(localStorage.getItem('rove-prototype-v1')!)); }
async function noOverflow(page: Page) { expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1)).toBe(true); }
async function installExisting(page: Page) {
  await page.addInitScript(state => {
    if (!sessionStorage.getItem('existing-data')) { localStorage.setItem('rove-prototype-v1', JSON.stringify(state)); sessionStorage.setItem('existing-data','1'); }
  }, seed('daily'));
}

for (const width of [320, 390, 768, 1440]) {
  test(`four pages fit ${width}px and only plus offers list actions`, async ({page}) => {
    const errors: string[] = [], external: string[] = [];
    page.on('pageerror', e => errors.push(e.message));
    page.on('request', r => { if (r.url().startsWith('http') && new URL(r.url()).origin !== 'http://127.0.0.1:4173') external.push(r.url()); });
    await page.setViewportSize({width,height:844}); await page.goto('/');
    await expect(page.getByRole('heading', {name:/你的设备，\s*随你漫游/})).toBeVisible();
    await expect(page.getByText('数据自己掌握，跳出平台控制')).toBeVisible();
    await noOverflow(page); await page.getByRole('button', {name:'开始',exact:true}).click();
    await expect(page.getByRole('heading', {name:'先认识你的 AI'})).toBeVisible(); await noOverflow(page);
    await page.getByRole('button', {name:'稍后添加',exact:true}).click();
    await expect(page.getByRole('heading', {name:'让设备，彼此相连'})).toBeVisible(); await noOverflow(page);
    await page.getByRole('button', {name:'暂停连接动画'}).click();
    const phase=await page.locator('.network-film').getAttribute('data-phase'); await page.waitForTimeout(2800);
    await expect(page.locator('.network-film')).toHaveAttribute('data-phase',phase!);
    await page.getByRole('button', {name:'稍后设置',exact:true}).click();
    await expect(page.getByRole('group', {name:'会话建议'}).getByRole('button')).toHaveCount(5); await noOverflow(page);
    await page.screenshot({path:`test-results/introduction-ready-${width}.png`,fullPage:true});
    await page.getByRole('button', {name:'进入会话列表'}).click();
    await expect(page.getByRole('region', {name:'会话列表'})).toBeVisible();
    await expect(page.locator('.list-bottom,.list-intro,.archive-entry,.demo-banner')).toHaveCount(0);
    await expect(page.getByRole('button',{name:'新会话',exact:true})).toHaveCount(0);
    if(width<=850) await expect(page.locator('.conversation-placeholder')).toBeHidden();
    await page.getByRole('button',{name:'全局添加'}).click();
    await expect(page.getByRole('menuitem',{name:'新会话',exact:true})).toBeVisible();
    await page.getByRole('menuitem',{name:'归档管理',exact:true}).click();
    await expect(page.getByRole('heading',{name:'归档管理',exact:true})).toBeVisible();
    await page.reload(); await expect(page.getByRole('region',{name:'使用引导'})).toHaveCount(0);
    expect(errors).toEqual([]); expect(external).toEqual([]);
  });
}

test('fourth page retains chosen prompt through model setup without submitting',async({page})=>{
  await page.goto('/'); await step(page,'4 开始对话');
  await page.getByRole('button',{name:/建立自己的私人影院/}).click();
  await expect(page.getByRole('dialog',{name:'添加模型'})).toBeVisible();
  await page.getByRole('button',{name:'取消',exact:true}).click();
  await expect(page.getByRole('heading',{name:'想做什么，聊聊就好'})).toBeVisible();
  await page.getByRole('button',{name:/建立自己的私人影院/}).click();
  await page.getByRole('button',{name:'模拟验证并获取型号'}).click();
  await page.getByRole('button',{name:'保存模型',exact:true}).click();
  await expect(page.getByRole('textbox',{name:'消息',exact:true})).toHaveValue(/我想建立自己的私人影院/);
  const saved=await snapshot(page); expect(saved.runs).toHaveLength(0); expect(saved.sessions.filter((s:any)=>s.onboarding)).toHaveLength(1);
  await page.reload(); await expect(page.getByRole('textbox',{name:'消息',exact:true})).toHaveValue(/私人影院/);
});

test('new conversation after existing model starts empty and keeps old conversations',async({page})=>{
  await installExisting(page); await page.goto('/'); await step(page,'2 添加模型');
  await expect(page.getByText('模型已准备好',{exact:true})).toBeVisible();
  await step(page,'4 开始对话'); await page.getByRole('button',{name:'新会话',exact:true}).click();
  await expect(page.getByRole('textbox',{name:'消息',exact:true})).toHaveValue('');
  const state=await snapshot(page); expect(state.sessions).toHaveLength(2); expect(state.runs).toHaveLength(0);
});

test('reset is deferred until reload, keeps drafts, models, networks, archives and tasks',async({page})=>{
  await installExisting(page); await page.goto('/'); await page.getByRole('button',{name:'跳过引导'}).click();
  await page.goto('/#/sessions/welcome'); await page.getByRole('textbox',{name:'消息',exact:true}).fill('验收草稿，不可丢失');
  await page.getByRole('button',{name:'归档会话',exact:true}).click();
  const before=await snapshot(page);
  await page.goto('/#/settings'); await page.getByRole('button',{name:'重置使用引导',exact:true}).click();
  await expect(page.getByRole('heading',{name:'设置',exact:true})).toBeVisible();
  await expect(page.getByRole('button',{name:'重置使用引导',exact:true})).toBeDisabled();
  await expect(page.getByRole('status')).toContainText('下次启动');
  await page.getByRole('button',{name:'全局添加'}).click(); await page.getByRole('menuitem',{name:'归档管理',exact:true}).click();
  await expect(page.getByRole('region',{name:'使用引导'})).toHaveCount(0);
  expect(await snapshot(page)).toEqual(before);
  await page.reload(); await expect(page.getByRole('heading',{name:/你的设备，\s*随你漫游/})).toBeVisible();
  expect(await snapshot(page)).toEqual(before);
  await page.getByRole('button',{name:'跳过引导'}).click(); await page.reload();
  await expect(page.getByRole('region',{name:'使用引导'})).toHaveCount(0);
  expect(await snapshot(page)).toEqual(before);
});

test('reset does not claim success when local storage rejects writes',async({page})=>{
  await page.goto('/'); await page.getByRole('button',{name:'跳过引导'}).click(); await page.goto('/#/settings');
  await page.evaluate(()=>{Storage.prototype.setItem=()=>{throw new Error('blocked');};});
  await page.getByRole('button',{name:'重置使用引导',exact:true}).click();
  await expect(page.getByRole('alert')).toContainText('未能安排下次重置');
  await expect(page.getByRole('button',{name:'重置使用引导',exact:true})).toBeEnabled();
});

test('reduced motion, dark theme and landscape preserve accessible steps',async({page})=>{
  await page.emulateMedia({reducedMotion:'reduce'}); await page.setViewportSize({width:844,height:390});
  await page.goto('/'); await page.evaluate(()=>document.documentElement.classList.add('dark')); await step(page,'3 连接设备');
  await expect(page.locator('.network-walkthrough')).toHaveClass(/reduced-motion/);
  await expect(page.getByRole('button',{name:'暂停连接动画'})).toHaveCount(0);
  await page.getByRole('list',{name:'连接设备步骤'}).getByRole('button',{name:'4 设备，从此相连'}).click();
  await expect(page.locator('.network-film')).toHaveAttribute('data-phase','3'); await noOverflow(page);
  await page.getByRole('button',{name:'加入网络',exact:true}).click(); await expect(page.getByRole('dialog',{name:'加入网络'})).toBeVisible();
});
