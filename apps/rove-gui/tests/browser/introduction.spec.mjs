import {test,expect} from '@playwright/test';
import {bridge} from './bridge.mjs';
for(const width of [320,360,390,844,1440])test(`shipping introduction fits ${width}px and list actions live in plus`,async({page},testInfo)=>{
  await page.setViewportSize({width,height:width===844?390:width===360?568:820});
  await bridge(page,{intro:true,os:width<900?'android':'linux'});await page.goto('/');
  await expect(page.getByRole('heading',{name:'你的设备，随你漫游'})).toBeVisible();
  if(width===360){const box=await page.getByRole('button',{name:'开始',exact:true}).boundingBox();expect(box.y+box.height).toBeLessThanOrEqual(568);}
  await page.screenshot({path:testInfo.outputPath('welcome.png'),fullPage:true});
  for(const name of ['添加模型','连接设备','开始对话']){
    await page.locator('.intro-progress').getByRole('button',{name:new RegExp(name)}).click();
    expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
    await page.screenshot({path:testInfo.outputPath(`${name}.png`),fullPage:true});
  }
  await page.getByRole('button',{name:'进入会话列表',exact:true}).click();
  await expect(page.locator('.introduction')).toHaveCount(0);
  await expect(page.locator('.conversation-list').getByRole('button',{name:'归档管理',exact:true})).toHaveCount(0);
  await expect(page.locator('.topbar .target-context')).toHaveCount(0);
  if(width<900)await expect(page.locator('.conversation-list')).toBeVisible();
  await page.getByRole('button',{name:'全局添加',exact:true}).click();
  await expect(page.locator('.global-menu').getByRole('button',{name:'归档管理',exact:true})).toBeVisible();
  await page.getByRole('button',{name:'新会话',exact:true}).click();
  await expect(page.getByRole('textbox',{name:'消息',exact:true})).toHaveValue('');
  expect(await page.evaluate(()=>window.testBridge.calls.filter(c=>c.operation_id==='submit_run'))).toHaveLength(0);
});
test('suggestion continues after real model save without sending or opening legacy network conversation',async({page})=>{
  await bridge(page,{intro:true,configured:false,empty:true});await page.goto('/');
  await page.locator('.intro-progress').getByRole('button',{name:/开始对话/}).click();
  await page.getByRole('button',{name:/建立自己的私人影院/}).click();
  const dialog=page.getByRole('dialog').last();
  await dialog.getByLabel('API 密钥',{exact:true}).fill('fixture-key');
  await expect(dialog.getByRole('button',{name:'保存模型',exact:true})).toBeDisabled();
  await dialog.getByRole('button',{name:'测试模型',exact:true}).click();
  await expect(dialog.locator('.model-test').getByRole('status')).toContainText('测试通过');
  await dialog.getByRole('button',{name:'保存模型',exact:true}).click();
  await expect(page.locator('.introduction')).toHaveCount(0);
  await expect(page.getByRole('textbox',{name:'消息',exact:true})).toHaveValue('帮我规划在这台设备上部署开源影音服务器，先不要执行安装。');
  const calls=await page.evaluate(()=>window.testBridge.calls);
  expect(calls.filter(c=>c.operation_id==='import_models')).toHaveLength(1);
  expect(calls.filter(c=>c.operation_id==='create_session')).toHaveLength(1);
  expect(calls.filter(c=>c.operation_id==='submit_run')).toHaveLength(0);
});
test('settings replay is deferred, keeps local draft and model configuration',async({page})=>{
  await bridge(page);await page.goto('/');
  await page.getByRole('textbox',{name:'消息',exact:true}).fill('保留这份草稿');
  await page.goto('/#/settings');await page.getByRole('button',{name:'重置导航页',exact:true}).click();
  await expect(page.locator('.introduction')).toHaveCount(0);
  await expect(page.getByRole('status')).toContainText('下次启动');
  await page.reload();await expect(page.locator('.introduction')).toBeVisible();
  await page.getByRole('button',{name:'开始',exact:true}).click();
  await expect(page.getByText('已添加 1 个型号，现有配置会保留。')).toBeVisible();
  await page.getByRole('button',{name:'跳过引导',exact:true}).click();
  await expect(page.getByRole('textbox',{name:'消息',exact:true})).toHaveValue('保留这份草稿');
  expect(await page.evaluate(()=>window.testBridge.calls.filter(c=>/delete|clear|submit|cancel/.test(c.operation_id)))).toHaveLength(0);
});
test('English reduced-motion guide and failed preference writes stay honest',async({page})=>{
  await page.emulateMedia({reducedMotion:'reduce'});await bridge(page,{intro:true});
  await page.addInitScript(()=>localStorage.setItem('rove-gui-locale','en'));await page.goto('/');
  await page.locator('.intro-progress').getByRole('button',{name:/Connect devices/}).click();
  await expect(page.locator('.network-walkthrough')).toHaveClass(/reduced-motion/);
  await expect(page.getByRole('button',{name:/Pause connection/})).toHaveCount(0);
  await page.evaluate(()=>{Storage.prototype.setItem=()=>{throw Error('test storage denied');};});
  await page.getByRole('button',{name:'Skip introduction',exact:true}).click();
  await expect(page.getByRole('status')).toContainText('Could not save');
  await expect(page.locator('.introduction')).toBeVisible();
});
