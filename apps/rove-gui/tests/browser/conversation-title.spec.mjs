import {test,expect} from '@playwright/test';
import {bridge} from './bridge.mjs';

for(const [width,height,os] of [[320,568,'android'],[844,390,'android'],[390,844,'linux']])test(`icon navigation stays compact and accessible at ${width}px ${os}`,async({page})=>{
  await page.setViewportSize({width,height});await bridge(page,{os});await page.goto('/');
  const nav=page.locator('.bottom-nav');await expect(nav).toBeVisible();await expect(nav.locator('a')).toHaveCount(3);
  expect((await nav.innerText()).trim()).toBe('');expect((await nav.boundingBox()).width).toBe(width);expect((await nav.boundingBox()).height).toBeLessThanOrEqual(54);
  for(const name of ['会话','服务','网络']){const link=nav.getByRole('link',{name,exact:true});await expect(link).toBeVisible();const box=await link.boundingBox();expect(box.width).toBeGreaterThanOrEqual(44);expect(box.height).toBeGreaterThanOrEqual(44);}
  await nav.getByRole('link',{name:'服务',exact:true}).click();await expect(page).toHaveURL(/services/);await expect(nav).toBeVisible();
  expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
});

test('inline title supports blur, Escape, blank validation and IME without a dialog',async({page})=>{
  await page.setViewportSize({width:320,height:568});await bridge(page,{os:'android'});await page.goto('/');
  await page.getByRole('button',{name:'音乐部署',exact:true}).click();
  const open=()=>page.getByRole('button',{name:'重命名会话',exact:true}).click();const input=page.getByRole('textbox',{name:'会话名称',exact:true});
  await open();await expect(page.getByRole('dialog')).toHaveCount(0);await input.fill('不要保存');await input.press('Escape');
  await expect(page.locator('.chat-title')).toHaveText('音乐部署');expect(await page.evaluate(()=>window.testBridge.calls.filter(c=>c.operation_id==='update_session'))).toHaveLength(0);
  await open();await input.fill('   ');await input.press('Enter');await expect(page.getByRole('alert')).toContainText('不能为空');
  await input.fill('中文输入');await input.dispatchEvent('keydown',{key:'Enter',code:'Enter',isComposing:true});await expect(input).toBeVisible();
  expect(await page.evaluate(()=>window.testBridge.calls.filter(c=>c.operation_id==='update_session'))).toHaveLength(0);
  await input.fill('我的音乐');await page.getByRole('textbox',{name:'消息',exact:true}).click();await expect(page.locator('.chat-title')).toHaveText('我的音乐');
  expect(await page.evaluate(()=>window.testBridge.calls.filter(c=>c.operation_id==='update_session'))).toHaveLength(1);
  expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
});

test('failed inline save keeps draft and retries only when requested',async({page})=>{
  await bridge(page);await page.goto('/');
  await page.evaluate(()=>{const invoke=window.__TAURI_INTERNALS__.invoke;window.rejectTitle=true;window.__TAURI_INTERNALS__.invoke=(cmd,args)=>args?.request?.operation_id==='update_session'&&window.rejectTitle?Promise.reject(Error('fixture-save-failed')):invoke(cmd,args);});
  await page.getByRole('button',{name:'重命名会话',exact:true}).click();const input=page.getByRole('textbox',{name:'会话名称',exact:true});
  await input.fill('重试名称');await input.press('Enter');await expect(page.getByRole('alert')).toContainText('fixture-save-failed');await expect(input).toHaveValue('重试名称');
  await page.evaluate(()=>window.rejectTitle=false);await input.press('Enter');await expect(page.locator('.chat-title')).toHaveText('重试名称');
});

test('new conversation takes a message title then keeps manual naming',async({page})=>{
  await bridge(page);await page.goto('/');await page.getByRole('button',{name:'全局添加',exact:true}).click();await page.getByRole('button',{name:'新会话',exact:true}).click();
  const message=page.getByRole('textbox',{name:'消息',exact:true});await message.fill('帮我建立私人影院');await page.getByRole('button',{name:'发送消息',exact:true}).click();
  await expect(page.locator('.chat-title')).toHaveText('帮我建立私人影院');await expect(page.locator('.session-item').first()).toHaveText('帮我建立私人影院');
  await page.getByRole('button',{name:'重命名会话',exact:true}).click();const input=page.getByRole('textbox',{name:'会话名称',exact:true});await input.fill('我的影院');await input.press('Enter');
  await expect(page.locator('.chat-title')).toHaveText('我的影院');await message.fill('再介绍其他软件');await page.getByRole('button',{name:'发送消息',exact:true}).click();
  await expect(page.locator('.chat-title')).toHaveText('我的影院');
  const calls=await page.evaluate(()=>window.testBridge.calls);expect(calls.filter(c=>c.operation_id==='submit_run')).toHaveLength(2);expect(calls.find(c=>c.operation_id==='create_session').body.auto_title).toBe(true);
});

test('slow rename applies to its original session after switching',async({page})=>{
  await bridge(page);await page.goto('/');
  await page.evaluate(()=>{const invoke=window.__TAURI_INTERNALS__.invoke;window.__TAURI_INTERNALS__.invoke=(cmd,args)=>args?.request?.operation_id==='update_session'?new Promise(resolve=>{window.finishTitle=async()=>resolve(await invoke(cmd,args));}):invoke(cmd,args);});
  await page.getByRole('button',{name:'重命名会话',exact:true}).click();const input=page.getByRole('textbox',{name:'会话名称',exact:true});await input.fill('音乐的新标题');await input.press('Enter');
  await page.getByRole('button',{name:'代码代理',exact:true}).click();await page.evaluate(()=>window.finishTitle());
  await expect(page.locator('.chat-title')).toHaveText('代码代理');await expect(page.getByRole('button',{name:'音乐的新标题',exact:true})).toBeVisible();
});

test('archive row delete checks that row, blocks live jobs and never calls deletion',async({page})=>{
  await bridge(page,{live:true});await page.addInitScript(()=>window.testBridge.sessions[0].archived_at='2026-09-20T00:00:00Z');await page.goto('/#/archives');
  await page.getByRole('button',{name:'删除 音乐部署',exact:true}).click();const dialog=page.getByRole('dialog');
  await expect(dialog.getByRole('alert')).toBeVisible();await expect(dialog.getByRole('button',{name:'确认永久删除'})).toBeDisabled();
  expect(await page.evaluate(()=>window.testBridge.calls.filter(c=>c.operation_id==='delete_session'))).toHaveLength(0);
});

test('delete targets an unselected archive without touching the active selected one',async({page})=>{
  await bridge(page,{live:true});await page.addInitScript(()=>window.testBridge.sessions.forEach(s=>s.archived_at='2026-09-20T00:00:00Z'));await page.goto('/#/archives');
  await expect(page.locator('.chat-title')).toHaveText('音乐部署');
  await page.getByRole('button',{name:'删除 代码代理',exact:true}).click();const dialog=page.getByRole('dialog');
  await expect(dialog).toContainText('代码代理');await dialog.getByRole('button',{name:'确认永久删除'}).click();
  await expect(page.getByRole('button',{name:'代码代理',exact:true})).toHaveCount(0);await expect(page.getByRole('button',{name:'音乐部署',exact:true})).toBeVisible();
  const calls=await page.evaluate(()=>window.testBridge.calls.filter(c=>c.operation_id==='delete_session'));expect(calls).toHaveLength(1);expect(calls[0].path_parameters.session_id).toBe('s2');
});

test('archive deletion stays disabled when remote state cannot be verified',async({page})=>{
  await bridge(page,{linked:true,offline:true});await page.addInitScript(()=>window.testBridge.sessions[0].archived_at='2026-09-20T00:00:00Z');await page.goto('/#/archives');
  await page.getByRole('button',{name:'删除 音乐部署',exact:true}).click();const dialog=page.getByRole('dialog');
  await expect(dialog.getByRole('alert')).toBeVisible();await expect(dialog.getByRole('button',{name:'确认永久删除'})).toBeDisabled();
  expect(await page.evaluate(()=>window.testBridge.calls.filter(c=>c.operation_id==='delete_session'))).toHaveLength(0);
});
