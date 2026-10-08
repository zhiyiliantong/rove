import {test,expect} from '@playwright/test';
import {bridge} from './bridge.mjs';

for(const viewport of [{width:1100,height:760},{width:320,height:640},{width:390,height:844},{width:844,height:390}]) {
  test(`recovering network remains stoppable ${viewport.width}x${viewport.height}`,async({page},info)=>{
    await page.setViewportSize(viewport);
    await bridge(page,{services:true,os:viewport.width===1100?'windows':'android'});
    await page.addInitScript(()=>{
      const invoke=window.__TAURI_INTERNALS__.invoke;
      window.testBridge.recoveryState='starting';
      Object.defineProperty(navigator,'clipboard',{configurable:true,value:{writeText:async text=>{window.testBridge.copiedRecovery=text;}}});
      window.__TAURI_INTERNALS__.invoke=async(cmd,args)=>{
        const r=args?.request;
        if(r?.operation_id==='stop_network'){
          window.testBridge.calls.push(structuredClone(r));
          window.testBridge.recoveryState='stopped';
          return {status_code:202,body:{state:'stopping'}};
        }
        const result=await invoke(cmd,args);
        if(r?.operation_id==='get_network')Object.assign(result.body,{
          state:window.testBridge.recoveryState,
          last_error:window.testBridge.recoveryState==='starting'?{
            code:'overlay_unavailable',message:'EasyTier runtime observation unavailable; retrying automatically in 30 seconds (stage: runtime_observation, os_error: None)'
          }:null
        });
        return result;
      };
    });
    await page.goto('/#/networks/n1');
    await expect(page.getByRole('alert')).toContainText('正在自动重试');
    const copy=page.getByRole('button',{name:'复制失败信息',exact:true});
    await expect(copy).toBeVisible();
    await copy.click();
    expect(await page.evaluate(()=>window.testBridge.copiedRecovery)).toContain('retrying automatically in 30 seconds');
    const stop=page.getByRole('button',{name:'停止',exact:true});
    await expect(stop).toBeEnabled();
    await stop.scrollIntoViewIfNeeded();
    expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
    await page.screenshot({path:info.outputPath('recovering-network.png'),animations:'disabled'});
    await stop.click();
    await expect(stop).toBeDisabled();
    await expect(page.getByRole('alert')).toHaveCount(0);
    const calls=await page.evaluate(()=>window.testBridge.calls.filter(c=>c.operation_id==='stop_network'));
    expect(calls).toHaveLength(1);
    expect(calls[0].path_parameters.network_id).toBe('n1');
    expect(calls[0].target).toBeUndefined();
  });
}
