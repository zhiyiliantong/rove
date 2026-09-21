import {test,expect} from '@playwright/test';
import {bridge} from './bridge.mjs';

for(const denied of [false,true])test(`message and failed reply copy preserve text, clipboard denied=${denied}`,async({page})=>{
  await page.setViewportSize({width:360,height:568});
  await bridge(page,{os:'android'});
  await page.addInitScript(denied=>{
    window.testBridge.snapshot.run.input_message='第一行\n  保留空格 <script>示例</script>';
    window.testBridge.snapshot.run.status='failed';
    window.testBridge.snapshot.run.error={code:'model_provider_failed',message:'Provider unavailable'};
    window.testBridge.snapshot.output_tail='';
    Object.defineProperty(navigator,'clipboard',{value:{writeText:async text=>{if(denied)throw Error('denied');window.copiedMessage=text;}}});
  },denied);
  await page.goto('/');
  await page.locator('.session-item').first().click();
  const original='第一行\n  保留空格 <script>示例</script>';
  await page.getByRole('button',{name:'复制消息',exact:true}).click();
  if(denied){
    await expect(page.getByRole('textbox',{name:'手动复制文本'})).toHaveValue(original);
    await page.getByRole('button',{name:'关闭手动复制'}).click();
  }else await expect.poll(()=>page.evaluate(()=>window.copiedMessage)).toBe(original);
  const error=await page.locator('.message-error > .error').innerText();
  expect(error).toContain('model_provider_failed');
  await page.getByRole('button',{name:'复制失败信息'}).click();
  if(denied)await expect(page.getByRole('textbox',{name:'手动复制文本'})).toHaveValue(error);
  else await expect.poll(()=>page.evaluate(()=>window.copiedMessage)).toBe(error);
  expect(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth)).toBe(false);
  expect(await page.evaluate(()=>window.testBridge.calls.some(c=>c.operation_id==='submit_run'))).toBe(false);
});
