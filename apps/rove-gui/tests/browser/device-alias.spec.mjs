import {test,expect} from '@playwright/test';
import {bridge} from './bridge.mjs';

const network='00000000-0000-4000-8000-000000000111';
for(const viewport of [{width:1100,height:760},{width:320,height:640},{width:390,height:844},{width:844,height:390}]){
  test(`local device aliases ${viewport.width}x${viewport.height}`,async({page},info)=>{
    await page.setViewportSize(viewport);
    await bridge(page,{sync:true,os:viewport.width===1100?'windows':'android'});
    await page.goto(`/#/networks/${network}`);
    const picker=page.locator('.picker');
    const title=picker.locator('.alias-name');
    const originalY=await title.evaluate(el=>el.getBoundingClientRect().y-el.closest('article').getBoundingClientRect().y);
    await picker.getByRole('button',{name:'编辑本地别名',exact:true}).click();
    const input=picker.getByRole('textbox',{name:'本地设备别名',exact:true});
    await expect(input).toBeFocused();
    const editY=await input.evaluate(el=>el.getBoundingClientRect().y-el.closest('article').getBoundingClientRect().y);
    expect(Math.abs(editY-originalY)).toBeLessThan(2);
    await expect(picker.locator('form')).toHaveCount(0);
    await expect(picker.getByRole('button',{name:'保存',exact:true})).toHaveCount(0);
    await input.fill('客厅小盒子');
    await input.dispatchEvent('keydown',{key:'Enter',isComposing:true,keyCode:229});
    await expect(input).toBeVisible();
    expect(await page.evaluate(()=>window.testBridge.calls.filter(c=>c.operation_id==='set_device_alias'))).toHaveLength(0);
    await input.press('Enter');
    await expect(title).toHaveText('客厅小盒子');
    expect(await page.evaluate(()=>window.testBridge.calls.filter(c=>c.operation_id==='set_device_alias').at(-1).target)).toBeUndefined();
    await page.reload();
    await expect(title).toHaveText('客厅小盒子');
    await title.click();await input.fill('取消修改');await input.press('Escape');
    await expect(title).toHaveText('客厅小盒子');
    expect(await page.evaluate(()=>window.testBridge.calls.filter(c=>c.operation_id==='set_device_alias'))).toHaveLength(0);
    // A name on another network must also block a local duplicate.
    await page.evaluate(()=>localStorage.setItem('fixture-device-aliases',JSON.stringify([...JSON.parse(localStorage.getItem('fixture-device-aliases')),{network_id:'00000000-0000-4000-8000-000000000333',device_id:'00000000-0000-4000-8000-000000000444',alias:'音乐盒子'}])));
    await picker.getByRole('button',{name:'编辑本地别名',exact:true}).click();
    await input.fill('音乐盒子');
    await input.press('Tab');
    await expect(picker.getByRole('alert')).toContainText('本机已使用这个别名');
    await expect(input).toHaveValue('音乐盒子');
    expect(await page.evaluate(()=>JSON.parse(localStorage.getItem('fixture-device-aliases'))[0].alias)).toBe('客厅小盒子');
    await page.screenshot({path:info.outputPath('duplicate.png')});
    await input.fill('客厅的小盒子很长的中文设备别名用于检查移动端换行和操作可达性');
    await input.press('Enter');
    await expect(title).toContainText('客厅的小盒子');
    expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
    await title.scrollIntoViewIfNeeded();
    await page.screenshot({path:info.outputPath('alias.png')});
    await picker.getByRole('button',{name:'编辑本地别名',exact:true}).click();
    await input.fill('');
    await input.press('Tab');
    await expect(title).toHaveText('同步目标');
    await expect(picker.getByRole('button',{name:'管理此设备',exact:true})).toHaveCount(0);
    // Management selection must not move newly created conversations.
    await page.getByRole('button',{name:'全局添加',exact:true}).click();
    await page.locator('.menu-panel').getByRole('button',{name:'新会话',exact:true}).click();
    await expect(page.getByRole('textbox',{name:'消息',exact:true})).toBeVisible();
    const created=await page.evaluate(()=>window.testBridge.calls.filter(c=>c.operation_id==='create_session').at(-1));
    expect(created.target).toBeUndefined();expect(created.body.execution_target).toBeUndefined();
  });
}
