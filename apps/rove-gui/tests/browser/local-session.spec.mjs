import {test,expect} from '@playwright/test';
import {bridge} from './bridge.mjs';

for(const viewport of [{width:1100,height:760},{width:320,height:640},{width:390,height:844},{width:844,height:390}]){
  test(`local creation and visible retry error ${viewport.width}x${viewport.height}`,async({page},info)=>{
    await page.setViewportSize(viewport);
    await bridge(page,{empty:true,os:viewport.width===1100?'windows':'android',localDeviceId:'00000000-0000-4000-8000-000000000111'});
    await page.goto('/');
    // The self-ID and a different peer must have exactly the same creation semantics.
    for(const device of ['00000000-0000-4000-8000-000000000111','00000000-0000-4000-8000-000000000222']){
      await page.evaluate(()=>location.hash='#/devices');
  await expect(page.getByRole('button',{name:'选择远端设备',exact:true})).toHaveCount(0);
      await page.evaluate(()=>{location.hash='#/sessions';window.testBridge.failSessionCreate=true;});
      await page.getByRole('button',{name:'全局添加',exact:true}).click();
      await page.locator('.menu-panel').getByRole('button',{name:'新会话',exact:true}).click();
      const error=page.locator('.session-creation-error');
      await expect(error).toContainText('session_create_failed');
      await page.waitForTimeout(3300); // The normal refresh must not erase the error.
      await expect(error).toBeVisible();
      await page.evaluate(()=>{Object.defineProperty(navigator,'clipboard',{configurable:true,value:{writeText:async text=>{window.copiedError=text;}}});});
      await error.getByRole('button',{name:'复制失败信息',exact:true}).click();
      expect(await page.evaluate(()=>window.copiedError)).toContain('session_create_failed');
      expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
      await page.screenshot({path:info.outputPath(`error-${device.slice(-3)}.png`)});
      await page.evaluate(()=>window.testBridge.failSessionCreate=false);
      await page.getByRole('button',{name:'全局添加',exact:true}).click();
      await page.locator('.menu-panel').getByRole('button',{name:'新会话',exact:true}).click();
      await expect(error).toHaveCount(0);
      await expect(page.getByRole('textbox',{name:'消息',exact:true})).toBeVisible();
      const calls=await page.evaluate(()=>window.testBridge.calls.filter(c=>c.operation_id==='create_session').slice(-2));
      expect(calls).toHaveLength(2);expect(calls[0].body).toEqual(calls[1].body);
      for(const c of calls){expect(c.target).toBeUndefined();expect(c.body.execution_target).toBeUndefined();}
      expect(await page.evaluate(()=>window.testBridge.calls.filter(c=>c.operation_id==='get_model_catalog').at(-1).target)).toBeUndefined();
      await page.screenshot({path:info.outputPath(`created-${device.slice(-3)}.png`)});
      if(viewport.width!==1100)await page.getByRole('button',{name:'返回会话列表',exact:true}).click();
    }
  });
}
