import {test,expect} from '@playwright/test';
import {bridge} from './bridge.mjs';

test('unavailable runtime shows an icon without pretending the network started',async({page})=>{
  await bridge(page,{services:true,os:'windows'});
  await page.goto('/#/networks');
  await page.getByRole('button',{name:'家庭网络',exact:true}).click();
  await page.evaluate(()=>{
    const original=window.__TAURI_INTERNALS__.invoke;
    window.__TAURI_INTERNALS__.invoke=(command,args)=>args?.request?.operation_id==='start_network'
      ?Promise.resolve({status_code:503,body:{error:{code:'overlay_unavailable',message:'EasyTier adapter is not configured; network remains stopped'}}})
      :original(command,args);
  });
  await page.getByRole('button',{name:'启动',exact:true}).click();
  const error=page.locator('.network-error');
  await expect(error).toContainText('EasyTier 网络运行时尚未就绪');
  await expect(error.locator('.pi-exclamation-circle')).toBeVisible();
  await expect(page.getByRole('button',{name:'启动',exact:true})).toBeEnabled();
  await expect(page.getByRole('button',{name:'停止',exact:true})).toBeDisabled();
});

for(const [width,height] of [[1100,760],[320,568],[390,844],[844,390]]){
  test(`VPN permission failure stays stopped and can be copied ${width}x${height}`,async({page})=>{
    await page.setViewportSize({width,height});
    await bridge(page,{services:true,os:'android'});
    await page.goto('/#/networks');
    await expect(page.locator('.network-page-heading')).toHaveCount(0);
    await expect(page.getByRole('button',{name:'刷新',exact:true})).toHaveCount(1);
    expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
    await page.screenshot({path:`/tmp/rove-network-list-${width}.png`});
    await page.getByRole('button',{name:'家庭网络',exact:true}).click();
    await page.evaluate(()=>{
      const original=window.__TAURI_INTERNALS__.invoke;
      window.__TAURI_INTERNALS__.invoke=(command,args)=>args?.request?.operation_id==='start_network'
        ?Promise.resolve({status_code:503,body:{error:{code:'overlay_permission_required',message:'VPN permission denied'}}})
        :original(command,args);
      Object.defineProperty(navigator,'clipboard',{configurable:true,value:{writeText:async text=>{window.copiedNetworkError=text;}}});
    });
    await page.getByRole('button',{name:'启动',exact:true}).click();
    const error=page.locator('.network-error');
    await expect(error).toContainText('系统网络授权未完成');
    await expect(page.locator('.notice').filter({hasText:'Android 使用系统 VPN'})).toBeVisible();
    await error.scrollIntoViewIfNeeded();
    await error.getByRole('button',{name:'复制失败信息',exact:true}).click();
    expect(await page.evaluate(()=>window.copiedNetworkError)).toContain('overlay_permission_required');
    await expect(page.getByRole('button',{name:'启动',exact:true})).toBeEnabled();
    await expect(page.getByRole('button',{name:'停止',exact:true})).toBeDisabled();
    expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
    await page.screenshot({path:`/tmp/rove-vpn-permission-${width}.png`});
  });
  test(`discovery stage errors keep the original URI ${width}x${height}`,async({page})=>{
    await page.setViewportSize({width,height});
    await bridge(page,{os:width===1100?'windows':'android'});
    await page.goto('/#/networks');
    await page.getByRole('button',{name:'全局添加',exact:true}).click();
    await page.getByRole('button',{name:'创建网络',exact:true}).click();
    const form=page.locator('.network-form');
    const input=form.getByLabel('初始节点 1',{exact:true});
    await input.fill('https://discovery.fixture/');
    await page.evaluate(()=>{
      const original=window.__TAURI_INTERNALS__.invoke;
      window.__TAURI_INTERNALS__.invoke=(command,args)=>args?.request?.operation_id==='resolve_bootstrap_discovery'&&window.testBridge.discoveryError
        ?Promise.resolve({status_code:504,body:{error:{code:window.testBridge.discoveryError,message:'fixture'}}})
        :original(command,args);
    });
    for(const [code,text] of [['discovery_http_timeout','发现请求超时'],['discovery_dns_timeout','DNS 查询超时']]){
      await page.evaluate(code=>{window.testBridge.discoveryError=code;},code);
      await form.getByRole('button',{name:'测试连通',exact:true}).click();
      const error=form.getByRole('alert');
      await expect(error).toContainText(text);
      await expect(error.locator('.pi-exclamation-circle')).toBeVisible();
      await error.scrollIntoViewIfNeeded();
      expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
      await expect(input).toHaveValue('https://discovery.fixture/');
    }
    await page.screenshot({path:`/tmp/rove-discovery-stage-error-${width}.png`});
    await page.evaluate(()=>{Object.defineProperty(navigator,'clipboard',{configurable:true,value:{writeText:async text=>{window.copiedNetworkError=text;}}});});
    const copy=form.getByRole('button',{name:'复制失败信息',exact:true});
    await copy.click();
    expect(await page.evaluate(()=>window.copiedNetworkError)).toContain('discovery_dns_timeout: fixture');
    expect(await page.evaluate(()=>window.copiedNetworkError)).toContain('DNS 查询超时');
    const bounds=await copy.boundingBox();expect(bounds.width).toBeGreaterThanOrEqual(44);expect(bounds.height).toBeGreaterThanOrEqual(44);
    await page.evaluate(()=>{navigator.clipboard.writeText=async()=>{throw Error('denied');};});
    await copy.click();
    const manual=form.getByRole('textbox',{name:'手动复制文本'});
    await expect(manual).toHaveValue(/discovery_dns_timeout: fixture/);
    await manual.focus();
    expect(await manual.evaluate(el=>el.selectionEnd-el.selectionStart)).toBeGreaterThan(0);
    expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
    await page.screenshot({path:`/tmp/rove-error-copy-${width}.png`});
    await form.getByRole('button',{name:'关闭手动复制',exact:true}).click();
    await page.evaluate(()=>{window.testBridge.discoveryError=null;});
    await form.getByRole('button',{name:'测试连通',exact:true}).click();
    await expect(form.getByRole('alert')).toHaveCount(0);
    await expect(form.getByText(/TCP 可达/)).toBeVisible();
    await expect(input).toHaveValue('https://discovery.fixture/');
  });
  test(`public discovery error is visible and recoverable ${width}x${height}`,async({page})=>{
    await page.setViewportSize({width,height});
    await bridge(page,{os:width===1100?'windows':'android'});
    await page.goto('/#/networks');
    await page.getByRole('button',{name:'全局添加',exact:true}).click();
    await page.getByRole('button',{name:'创建网络',exact:true}).click();
    const form=page.locator('.network-form');
    await form.getByLabel('网络名称',{exact:true}).fill('保留网络');
    await form.getByLabel('初始节点 1',{exact:true}).fill('txt://seed.fixture');
    await page.evaluate(()=>{window.testBridge.publicError='discovery_service_not_configured';});
    await form.getByRole('button',{name:'获取公共初始节点',exact:true}).click();
    const error=form.getByRole('alert');
    await expect(error).toContainText('尚未配置公共节点发现服务');
    await expect(error.locator('.pi-exclamation-circle')).toBeVisible();
    await error.scrollIntoViewIfNeeded();
    expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
    await page.screenshot({path:`/tmp/rove-network-error-${width}.png`});
    await expect(form.getByLabel('网络名称',{exact:true})).toHaveValue('保留网络');
    await expect(form.getByLabel('初始节点 1',{exact:true})).toHaveValue('txt://seed.fixture');
    await page.evaluate(()=>{window.testBridge.publicError='public_nodes_unavailable';});
    await form.getByRole('button',{name:'获取公共初始节点',exact:true}).click();
    await expect(error).toContainText('无法访问公共节点发现服务');
    await page.evaluate(()=>{window.testBridge.publicError=null;});
    await form.getByRole('button',{name:'获取公共初始节点',exact:true}).click();
    await expect(error).toHaveCount(0);
    await expect(form.getByRole('button',{name:'使用公共节点 Fixture public node'})).toBeVisible();
  });
}
