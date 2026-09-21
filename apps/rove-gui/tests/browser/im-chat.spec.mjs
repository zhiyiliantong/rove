import {test,expect} from '@playwright/test';
import {bridge} from './bridge.mjs';

for(const [width,height,os] of [[360,568,'android'],[844,390,'android'],[1440,900,'linux']])test(`one pinned chat shell and deliberate stream following at ${width}x${height}`,async({page})=>{
  await page.setViewportSize({width,height});await bridge(page,{live:true,os});
  await page.addInitScript(()=>{window.testBridge.snapshot.output_tail=Array.from({length:60},(_,i)=>`段落 ${i}：连续消息，不要移动顶部栏。`).join('\n\n');});
  await page.goto('/');await page.getByRole('button',{name:'音乐部署',exact:true}).click();
  await expect(page.locator('.history-details,.conversation-header')).toHaveCount(0);
  const log=page.getByRole('log'),header=page.locator('.topbar'),composer=page.locator('.composer');
  await expect(log).toContainText('段落 59');
  await expect.poll(()=>log.evaluate(el=>el.scrollHeight-el.scrollTop-el.clientHeight)).toBeLessThan(80);
  const top=await header.boundingBox(),bottom=await composer.boundingBox();
  expect(top.y).toBeGreaterThanOrEqual(0);expect(bottom.y+bottom.height).toBeLessThan(height);
  await log.evaluate(el=>{el.scrollTop=100;el.dispatchEvent(new Event('scroll'));});
  await page.evaluate(()=>window.testBridge.events.push({run_id:'r1',seq:2,kind:'assistant_delta',data:{text:'\n\n新的流式应答'}}));
  await expect(log).toContainText('新的流式应答');
  expect(await log.evaluate(el=>el.scrollTop)).toBeCloseTo(100,0);
  expect((await header.boundingBox()).y).toBeCloseTo(top.y,0);
  expect((await composer.boundingBox()).y).toBeCloseTo(bottom.y,0);
  expect(await page.evaluate(()=>document.documentElement.scrollHeight<=innerHeight+1)).toBe(true);
  await page.getByRole('button',{name:'回到最新消息',exact:true}).click();
  await expect.poll(()=>log.evaluate(el=>el.scrollHeight-el.scrollTop-el.clientHeight)).toBeLessThan(5);
  await page.getByRole('button',{name:'重命名会话',exact:true}).click();
  await expect(page.getByRole('dialog')).toHaveCount(0);
  const rename=page.getByRole('textbox',{name:'会话名称',exact:true});await rename.fill('我的即时会话');
  await rename.press('Enter');await expect(page.locator('.chat-title')).toHaveText('我的即时会话');
  if(os==='android'){
    await page.getByRole('button',{name:'返回会话列表',exact:true}).click();
    await expect(page.getByRole('button',{name:'我的即时会话',exact:true})).toBeVisible();
    await expect(log).toBeHidden();
  }
});

test('suggestions default on, persist off, and never change agent configuration',async({page})=>{
  await bridge(page);await page.goto('/');await page.getByRole('button',{name:'代码代理',exact:true}).click();
  await expect(page.getByRole('group',{name:'会话建议'})).toBeVisible();
  await page.evaluate(()=>location.hash='#/settings');await page.getByLabel('显示对话建议',{exact:true}).uncheck();
  await page.reload();await expect(page.getByLabel('显示对话建议',{exact:true})).not.toBeChecked();
  await page.evaluate(()=>location.hash='#/sessions');await page.getByRole('button',{name:'代码代理',exact:true}).click();
  await expect(page.getByRole('group',{name:'会话建议'})).toHaveCount(0);
  expect(await page.evaluate(()=>window.testBridge.calls.filter(c=>['update_settings','submit_run'].includes(c.operation_id)))).toHaveLength(0);
});

test('automatic model listing uses one editable row, retains names and does not infer',async({page})=>{
  await bridge(page);await page.goto('/#/models');await page.getByRole('button',{name:'添加模型',exact:true}).click();
  const dialog=page.getByRole('dialog');await dialog.getByLabel('提供商',{exact:true}).selectOption('deepseek');
  await dialog.getByLabel('API 密钥',{exact:true}).fill('fixture-only');await dialog.getByLabel('API 密钥',{exact:true}).press('Tab');
  await expect(dialog.locator('.model-choice')).toHaveCount(2);
  await expect(dialog.getByLabel('real-chat',{exact:true})).toBeChecked();
  const name=dialog.getByLabel('real-chat · 配置名称');await name.fill('私人助手');
  await dialog.getByRole('button',{name:'获取可用型号',exact:true}).click();await expect(name).toHaveValue('私人助手');
  expect(await page.evaluate(()=>window.testBridge.calls.filter(c=>['test_model','submit_run','import_models'].includes(c.operation_id)))).toHaveLength(0);
  expect(await page.evaluate(()=>JSON.stringify(localStorage))).not.toContain('fixture-only');
});

test('stale model discovery cannot replace a different provider or its key',async({page})=>{
  await bridge(page);await page.goto('/#/models');
  await page.evaluate(()=>{const original=window.__TAURI_INTERNALS__.invoke;window.pendingListings=[];window.__TAURI_INTERNALS__.invoke=(cmd,args)=>args?.request?.operation_id==='discover_models'?new Promise(resolve=>window.pendingListings.push(resolve)):original(cmd,args);});
  await page.getByRole('button',{name:'添加模型',exact:true}).click();const dialog=page.getByRole('dialog');
  await dialog.getByLabel('API 密钥',{exact:true}).fill('first-fixture');await dialog.getByLabel('API 密钥',{exact:true}).press('Tab');
  await expect.poll(()=>page.evaluate(()=>window.pendingListings.length)).toBe(1);
  await dialog.getByLabel('提供商',{exact:true}).selectOption('deepseek');
  await page.evaluate(()=>window.pendingListings.shift()({status_code:200,body:{models:['obsolete-model'],truncated:false}}));
  await expect(dialog.getByLabel('obsolete-model',{exact:true})).toHaveCount(0);
  await expect(dialog.getByLabel('API 密钥',{exact:true})).toHaveValue('');
  await dialog.getByLabel('API 密钥',{exact:true}).fill('second-fixture');await dialog.getByLabel('API 密钥',{exact:true}).press('Tab');
  await expect.poll(()=>page.evaluate(()=>window.pendingListings.length)).toBe(1);
  await page.evaluate(()=>window.pendingListings.shift()({status_code:200,body:{models:['current-model'],truncated:false}}));
  await expect(dialog.getByLabel('current-model',{exact:true})).toBeChecked();
});

test('failed automatic listing retains editable reference models without background retries',async({page})=>{
  await bridge(page);await page.goto('/#/models');
  await page.evaluate(()=>{const original=window.__TAURI_INTERNALS__.invoke;window.listAttempts=0;window.__TAURI_INTERNALS__.invoke=(cmd,args)=>{if(args?.request?.operation_id==='discover_models'){window.listAttempts++;return Promise.resolve({status_code:502,body:{error:{code:'model_discovery_failed',message:'fixture-provider-error'}}});}return original(cmd,args);};});
  await page.getByRole('button',{name:'添加模型',exact:true}).click();const dialog=page.getByRole('dialog');
  await dialog.getByLabel('提供商',{exact:true}).selectOption('deepseek');
  await dialog.getByLabel('API 密钥',{exact:true}).fill('failure-fixture');await dialog.getByLabel('API 密钥',{exact:true}).press('Tab');
  await expect(dialog.getByRole('status')).toContainText('保留现有列表');
  await expect(dialog.getByLabel('deepseek-flash',{exact:true})).toBeChecked();
  const name=dialog.getByLabel('deepseek-flash · 配置名称');await name.fill('可手动补充');await name.press('Tab');
  expect(await page.evaluate(()=>window.listAttempts)).toBe(1);
  await dialog.getByRole('button',{name:'获取可用型号',exact:true}).click();
  await expect.poll(()=>page.evaluate(()=>window.listAttempts)).toBe(2);await expect(name).toHaveValue('可手动补充');
  expect(await page.evaluate(()=>window.testBridge.calls.filter(c=>c.operation_id==='test_model'))).toHaveLength(0);
});
