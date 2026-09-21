import {test,expect} from '@playwright/test';
import {bridge} from './bridge.mjs';

for(const [width,height,os] of [[320,568,'android'],[844,390,'android'],[1440,900,'linux']])test(`compact chat hides success chrome at ${width}px`,async({page})=>{
  await page.setViewportSize({width,height});await bridge(page,{os});await page.goto('/');
  if(os==='android')await expect(page.locator('.bottom-nav')).toBeVisible();
  await page.getByRole('button',{name:'音乐部署',exact:true}).click();
  await expect(page.getByRole('log')).toContainText('部署建议');
  await expect(page.getByRole('log').getByText('已完成',{exact:true})).toHaveCount(0);
  await expect(page.locator('.execution-details,.bubble-meta')).toHaveCount(0);
  await expect(page.locator('.bottom-nav')).toBeHidden();
  const bar=await page.locator('.topbar').boundingBox();expect(bar.height).toBeLessThanOrEqual(56);
  for(const button of await page.locator('.topbar .p-button:visible').all()){
    const box=await button.boundingBox();expect(box.height).toBeGreaterThanOrEqual(44);expect(box.width).toBeGreaterThanOrEqual(44);
  }
  if(os==='android'){
    await page.getByRole('button',{name:'返回会话列表',exact:true}).click();await expect(page.locator('.bottom-nav')).toBeVisible();
    await page.locator('.bottom-nav').getByRole('link',{name:'服务',exact:true}).click();await expect(page.locator('.bottom-nav')).toBeVisible();
  }else await expect(page.locator('.sidebar')).toBeVisible();
});

test('failed jobs remain visible and live jobs retain cancellation',async({page})=>{
  await bridge(page,{live:true});await page.goto('/');
  await expect(page.getByRole('button',{name:'取消此作业',exact:true})).toBeVisible();
  await page.evaluate(()=>{window.testBridge.snapshot.run.status='failed';window.testBridge.snapshot.run.error={code:'fixture_failed',message:'请检查设备连接'};});
  await expect(page.getByRole('log')).toContainText('请检查设备连接');
  await expect(page.locator('.execution-details')).toHaveCount(0);
});

test('session list requests descending order and prepends new sessions',async({page})=>{
  await bridge(page);await page.addInitScript(()=>window.testBridge.sessions.push({session_id:'s3',title:'最新会话',created_at:'2026-09-19T00:00:00Z'}));await page.goto('/');
  await expect(page.locator('.session-item')).toHaveText(['最新会话','音乐部署','代码代理']);
  await page.getByRole('button',{name:'全局添加',exact:true}).click();await page.getByRole('button',{name:'新会话',exact:true}).click();
  await expect(page.locator('.session-item').first()).toContainText('新会话');
  const calls=await page.evaluate(()=>window.testBridge.calls.filter(c=>c.operation_id==='list_sessions'));
  expect(calls.length).toBeGreaterThan(0);expect(calls.every(c=>c.query_parameters.order==='desc')).toBe(true);
});

test('network-scoped sync rejects discovery rows belonging to another network',async({page})=>{
  await bridge(page,{sync:true});await page.goto('/#/networks');
  await page.evaluate(()=>{const invoke=window.__TAURI_INTERNALS__.invoke;window.__TAURI_INTERNALS__.invoke=async(cmd,args)=>{
    const result=await invoke(cmd,args);if(args?.request?.operation_id==='list_network_devices')result.body.items[0].network_id='different-network';return result;
  };});
  await page.getByRole('button',{name:'同步本机 API 配置',exact:true}).click();
  const dialog=page.getByRole('dialog');await expect(dialog.getByRole('alert')).toBeVisible();
  await expect(dialog.getByRole('button',{name:'选择设备',exact:true})).toHaveCount(0);
  await expect(dialog.getByRole('button',{name:'确认同步',exact:true})).toBeDisabled();
  expect(await page.evaluate(()=>window.testBridge.calls.filter(c=>c.operation_id==='sync_model_connection'))).toHaveLength(0);
});
