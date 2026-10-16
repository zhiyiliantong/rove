import { test, expect } from '@playwright/test';
import {readFileSync} from 'node:fs';
const network_qr=JSON.parse(readFileSync(new URL('./network-qr.json',import.meta.url),'utf8'));

test.beforeEach(async ({ page }) => {
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  page.testErrors = errors;
});
test.afterEach(async ({ page }) => { expect(page.testErrors).toEqual([]); });

import {bridge} from './bridge.mjs';

test('network form separates address modes, tests individual peers and exports a private card',async({page})=>{
  await bridge(page);await page.goto('/#/networks');
  await expect(page.locator('.network-form')).toHaveCount(0);
  await page.getByRole('button',{name:'全局添加',exact:true}).click();
  await page.getByRole('button',{name:'创建网络',exact:true}).click();
  const form=page.locator('.network-form');
  await expect(form.getByLabel('网络名称',{exact:true})).toHaveValue(/^net-[a-z0-9]{2}$/);
  await form.getByLabel('网络名称',{exact:true}).fill('我的网络');
  const key=form.locator('input[autocomplete="off"]');
  await expect(key).toHaveAttribute('type','password');
  expect(await key.inputValue()).toMatch(/^[0-9a-f]{64}$/);
  await form.getByRole('button',{name:'显示',exact:true}).click();await expect(key).toHaveAttribute('type','text');
  await form.getByLabel('初始节点 1',{exact:true}).fill('tcp://fixture.invalid:11010');
  await form.getByRole('button',{name:'测试连通',exact:true}).click();
  await expect(form.getByText(/TCP 可达/)).toBeVisible();
  await form.getByRole('button',{name:'添加初始节点',exact:true}).click();
  await form.getByLabel('初始节点 2',{exact:true}).fill('udp://fixture.invalid:11010');
  await form.getByRole('button',{name:'测试连通',exact:true}).nth(1).click();
  await expect(form.getByText('该协议暂不支持连通测试')).toBeVisible();
  await form.getByLabel('地址模式').selectOption({label:'手动指定网段'});
  await form.getByLabel('虚拟网段',{exact:true}).fill('192.168.100.7/24');
  await form.getByLabel('本机 IP（不随名片分享）',{exact:true}).fill('192.168.100.2');
  await form.getByRole('button',{name:'创建网络',exact:true}).click();
  await expect(page.getByRole('heading',{name:'网络名片',exact:true})).toBeVisible();
  const created=await page.evaluate(()=>window.testBridge.createdNetwork);
  expect(created.config.dhcp).toBe(false);expect(created.config.ipv4_cidr).toBe('192.168.100.0/24');
  expect(created.local_ipv4).toBe('192.168.100.2');expect(created.config.local_ipv4).toBeUndefined();
  await expect(page.locator('.network-card input')).toHaveCount(0);
  expect(await page.locator('.network-card textarea').inputValue()).not.toContain('local_ipv4');
  await page.getByRole('button',{name:'返回网络',exact:true}).last().click();
  await expect(page.locator('.network-card')).toHaveCount(0);
  await page.getByRole('button',{name:'全局添加',exact:true}).click();
  await page.getByRole('button',{name:'创建网络',exact:true}).click();
  await expect(form.getByLabel('地址模式')).toHaveValue('true');
  await form.getByLabel('网络名称',{exact:true}).fill('自动网络');
  await form.getByRole('button',{name:'创建网络',exact:true}).click();
  await expect.poll(()=>page.evaluate(()=>window.testBridge.createdNetwork.display_name)).toBe('自动网络');
  const automatic=await page.evaluate(()=>window.testBridge.createdNetwork);
  expect(automatic.config.dhcp).toBe(true);expect(automatic.config.ipv4_cidr).toBeUndefined();expect(automatic.local_ipv4).toBeNull();
});

test('public, four discovery and seven connector protocols require selection and preserve original URIs',async({page})=>{
  await bridge(page);await page.goto('/#/networks');
  await page.getByRole('button',{name:'全局添加',exact:true}).click();
  await page.getByRole('button',{name:'创建网络',exact:true}).click();
  const form=page.locator('.network-form');
  await form.getByRole('button',{name:'获取公共初始节点'}).click();
  await expect(form.getByText('udp://public.fixture:7178',{exact:false})).toBeVisible();
  await form.getByRole('button',{name:'使用公共节点 Fixture public node'}).click();
  await expect(form.getByLabel('初始节点 1',{exact:true})).toHaveValue('udp://public.fixture:7178');
  const discovery=['https://directory.fixture/node','http://directory.fixture/node','txt://seed.fixture','srv://seed.fixture'];
  for(const url of discovery){
    await form.getByRole('button',{name:'添加初始节点',exact:true}).click();
    const peerNumber=await form.locator('.peer-row').count();
    await form.getByLabel(`初始节点 ${peerNumber}`,{exact:true}).fill(url);
    const peerRow=form.getByLabel(`初始节点 ${peerNumber}`,{exact:true}).locator('..');
    await peerRow.getByRole('button',{name:'测试连通',exact:true}).click();
    await expect(peerRow.getByText('本次测试使用解析后的节点：tcp://resolved.fixture:7196')).toBeVisible();
    await expect(peerRow.getByText('TCP 可达（未验证入网）')).toBeVisible();
    await expect(form.getByLabel(`初始节点 ${peerNumber}`,{exact:true})).toHaveValue(url);
    await expect.poll(()=>page.evaluate(()=>window.testBridge.calls.filter(c=>c.operation_id==='test_network_peer').at(-1)?.body.endpoint)).toBe('tcp://resolved.fixture:7196');
  }
  const connectors=['tcp://node.fixture:11010','udp://node.fixture:11010','ws://node.fixture:11011','wss://node.fixture:11012','wg://node.fixture:11013','quic://node.fixture:11014','faketcp://node.fixture:11015'];
  for(const endpoint of connectors){
    await form.getByRole('button',{name:'添加初始节点',exact:true}).click();
    await form.getByLabel(`初始节点 ${await form.locator('.peer-row').count()}`,{exact:true}).fill(endpoint);
  }
  await form.getByRole('button',{name:'创建网络',exact:true}).click();
  await expect(page.getByRole('heading',{name:'网络名片',exact:true})).toBeVisible();
  const created=await page.evaluate(()=>window.testBridge.createdNetwork);
  expect(created.config.bootstrap_peers).toEqual(['udp://public.fixture:7178',...discovery,...connectors]);
});

test('device settings persist and clear the selected agent discovery service',async({page})=>{
  await bridge(page);await page.goto('/#/settings');
  const input=page.getByLabel('公共节点发现服务',{exact:true});
  await expect(input).toHaveValue('');
  await input.fill('https://directory.example');
  await page.getByRole('button',{name:'保存设置',exact:true}).click();
  await expect.poll(()=>page.evaluate(()=>window.testBridge.calls.filter(c=>c.operation_id==='update_settings').at(-1)?.body.discovery_service_url)).toBe('https://directory.example');
  await input.fill('');
  await page.getByRole('button',{name:'保存设置',exact:true}).click();
  await expect.poll(()=>page.evaluate(()=>window.testBridge.calls.filter(c=>c.operation_id==='update_settings').at(-1)?.body.discovery_service_url)).toBeNull();
});

test('creation failures preserve input and card retry never creates a second network',async({page})=>{
  await bridge(page);await page.goto('/#/networks');
  await page.getByRole('button',{name:'全局添加',exact:true}).click();
  await page.getByRole('button',{name:'创建网络',exact:true}).click();
  const form=page.locator('.network-form'),name=form.getByLabel('网络名称',{exact:true});
  await name.fill('保留输入');
  await page.evaluate(()=>window.testBridge.failCreate=true);
  await form.getByRole('button',{name:'创建网络',exact:true}).click();
  await expect(page.getByRole('alert')).toContainText('fixture create failed');
  await expect(name).toHaveValue('保留输入');
  await expect(page.getByRole('heading',{name:'网络名片',exact:true})).toHaveCount(0);
  await page.evaluate(()=>{window.testBridge.failCreate=false;window.testBridge.failCard=true;});
  await form.getByRole('button',{name:'创建网络',exact:true}).click();
  await expect(page.getByText(/网络已创建，但暂时无法读取名片/).first()).toBeVisible();
  await page.evaluate(()=>window.testBridge.failCard=false);
  await page.getByRole('button',{name:'重试读取名片'}).click();
  await expect(page.getByRole('heading',{name:'网络名片',exact:true})).toBeVisible();
  expect(await page.evaluate(()=>window.testBridge.calls.filter(call=>call.operation_id==='create_network').length)).toBe(2);
});

test('desktop network file import requires review and uses the normal API',async({page})=>{
  await bridge(page);await page.goto('/#/networks');
  await page.getByRole('button',{name:'全局添加',exact:true}).click();
  await page.getByRole('button',{name:'加入网络',exact:true}).click();
  await expect(page.getByRole('button',{name:'扫描二维码',exact:true})).toHaveCount(0);
  const config={schema_version:1,network_id:'fixture',display_name:'imported',easytier:{network_secret:'test-only'}};
  const encoded=Buffer.from(JSON.stringify(config)).toString('base64url');
  await page.getByLabel('读取 Base64 网络配置文件').setInputFiles({name:'network.rove',mimeType:'text/plain',buffer:Buffer.from(encoded)});
  await expect(page.getByRole('status')).toHaveText(/已读取并解码配置/);
  expect(await page.evaluate(()=>window.testBridge.calls.filter(c=>c.operation_id==='import_network'))).toHaveLength(0);
  await expect(page.getByPlaceholder('粘贴 Base64 网络配置')).toHaveValue(encoded);
  await page.getByRole('button',{name:'解码并导入',exact:true}).click();
  const imported=await page.evaluate(()=>window.testBridge.calls.find(c=>c.operation_id==='import_network'));
  expect(imported.body).toEqual({source:'manual',config});
  expect(await page.evaluate(()=>window.testBridge.calls.some(c=>c.operation_id==='create_network_share'||c.body?.source==='url'))).toBe(false);
});

for(const [width,height] of [[1100,760],[320,568],[390,844],[844,390]])test(`network details own their devices at ${width}px`,async({page})=>{
  await page.setViewportSize({width,height});await bridge(page,{services:true,os:width<950?'android':'linux'});
  await page.addInitScript(()=>{
    const invoke=window.__TAURI_INTERNALS__.invoke;
    window.__TAURI_INTERNALS__.invoke=async(command,args)=>{
      const result=await invoke(command,args);
      if(args?.request?.operation_id==='list_network_devices'){
        const network_id=args.request.path_parameters.network_id;
        result.body.items=[{network_id,device_id:'00000000-0000-4000-8000-000000000222',display_name:network_id==='n1'?'家庭电脑':'工作电脑',os:'windows',arch:'x86_64',state:'online',capabilities:['system_exec'],overlay_addresses:['10.50.0.2'],last_error:null}];
      }
      return result;
    };
  });
  await page.goto('/#/networks');
  await expect(page.getByRole('button',{name:'查看设备',exact:true})).toHaveCount(0);
  await page.getByRole('button',{name:'家庭网络',exact:true}).click();
  await expect(page).toHaveURL(/#\/networks\/n1$/);
  await expect(page.getByRole('heading',{name:'家庭网络',exact:true})).toBeVisible();
  await expect(page.getByRole('heading',{name:'设备',exact:true})).toBeVisible();
  await expect(page.getByText('家庭电脑',{exact:true})).toBeVisible();
  await expect(page.getByText('工作电脑',{exact:true})).toHaveCount(0);
  await expect(page.getByRole('combobox',{name:'网络',exact:true})).toHaveCount(0);
  await page.getByRole('button',{name:'返回网络',exact:true}).click();
  await page.getByRole('button',{name:'工作网络',exact:true}).click();
  await expect(page.getByText('工作电脑',{exact:true})).toBeVisible();
  await expect(page.getByText('家庭电脑',{exact:true})).toHaveCount(0);
  expect(await page.evaluate(()=>window.testBridge.calls.filter(c=>c.operation_id==='list_network_devices').map(c=>c.path_parameters.network_id))).toEqual(['n1','n2']);
  expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
});

test('network details ignore stale network responses and reject devices from another network',async({page})=>{
  await bridge(page,{services:true});await page.goto('/#/networks');
  await page.evaluate(()=>{
    const invoke=window.__TAURI_INTERNALS__.invoke;
    window.__TAURI_INTERNALS__.invoke=async(command,args)=>{
      const result=await invoke(command,args),request=args?.request;
      if(request?.operation_id==='get_network'&&request.path_parameters.network_id==='n1')await new Promise(resolve=>{window.testBridge.releaseNetwork=resolve;});
      if(request?.operation_id==='list_network_devices')result.body.items=[{network_id:'n1',device_id:'wrong',display_name:'错误网络设备'}];
      return result;
    };
  });
  await page.getByRole('button',{name:'家庭网络',exact:true}).click();
  await expect.poll(()=>page.evaluate(()=>!!window.testBridge.releaseNetwork)).toBe(true);
  await page.evaluate(()=>{location.hash='#/networks/n2';});
  await expect(page.getByRole('heading',{name:'工作网络',exact:true})).toBeVisible();
  await page.evaluate(()=>window.testBridge.releaseNetwork());
  await expect(page.locator('.picker [role=alert]')).toBeVisible();
  await expect(page.getByText('错误网络设备',{exact:true})).toHaveCount(0);
  await expect(page.getByRole('heading',{name:'工作网络',exact:true})).toBeVisible();
});

test('old device bookmarks cannot switch the local network context',async({page})=>{
  await bridge(page,{services:true});await page.goto('/#/devices');
  const target={network_id:'00000000-0000-4000-8000-000000000111',device_id:'00000000-0000-4000-8000-000000000222'};
  await expect(page.getByRole('button',{name:'选择远端设备',exact:true})).toHaveCount(0);
  await page.evaluate(()=>{location.hash='#/networks/n1';});
  await expect(page.getByRole('heading',{name:'家庭网络',exact:true})).toBeVisible();
  await expect.poll(()=>page.evaluate(()=>window.testBridge.calls.filter(c=>c.operation_id==='list_network_devices').length)).toBeGreaterThan(0);
  const calls=await page.evaluate(()=>window.testBridge.calls.filter(c=>['get_network','list_network_devices'].includes(c.operation_id)));
  expect(calls.every(c=>c.path_parameters.network_id==='n1'&&c.target===undefined)).toBe(true);
  await expect(page.locator('.picker').getByRole('button',{name:'选择设备',exact:true})).toHaveCount(0);
});

test('network tab keeps prototype list, card, create and join views separate',async({page})=>{
  await bridge(page,{services:true});await page.goto('/#/networks');
  await expect(page.locator('.network-page-heading')).toHaveCount(0);
  await expect(page.getByRole('button',{name:'刷新',exact:true})).toBeVisible();
  await expect(page.getByRole('heading',{name:'我的网络',exact:true})).toHaveCount(0);
  await expect(page.getByText(/可保存、分享并加入多个网段/)).toHaveCount(0);
  await expect(page.getByRole('button',{name:'创建网络',exact:true})).toHaveCount(0);
  await expect(page.getByRole('button',{name:'加入网络',exact:true})).toHaveCount(0);
  await expect(page.locator('.network-form')).toHaveCount(0);
  await expect(page.getByPlaceholder('粘贴 Base64 网络配置')).toHaveCount(0);
  await page.getByRole('button',{name:'家庭网络',exact:true}).click();
  await page.getByRole('button',{name:'网络名片',exact:true}).click();
  await expect(page.getByRole('heading',{name:'网络名片',exact:true})).toBeVisible();
  const payload=await page.getByLabel('Base64 网络配置（敏感）').inputValue();
  const decoded=JSON.parse(Buffer.from(payload,'base64url').toString('utf8'));
  expect(decoded).toMatchObject({schema_version:1,network_id:'n1',display_name:'家庭网络'});
  expect(payload).not.toContain('http://');
  await expect(page.getByRole('dialog')).toBeVisible();
  await expect(page.getByRole('img',{name:'网络分享二维码'})).toBeVisible();
  expect(await page.evaluate(()=>window.testBridge.calls.find(c=>c.command==='network_config_qr').config)).toBe(payload);
  await expect(page.getByRole('button',{name:'复制配置',exact:true})).toBeVisible();
  await expect(page.getByRole('dialog').getByRole('button',{name:'返回网络',exact:true})).toHaveText('');
  await expect(page.getByRole('button',{name:/导出|保存二维码/})).toHaveCount(0);
  expect(await page.evaluate(()=>window.testBridge.calls.filter(c=>c.operation_id==='create_network_share'))).toHaveLength(0);
  await page.getByRole('button',{name:'返回网络',exact:true}).last().click();
  await page.getByRole('button',{name:'全局添加',exact:true}).click();
  await page.getByRole('button',{name:'加入网络',exact:true}).click();
  await expect(page.getByRole('heading',{name:'加入网络',exact:true})).toBeVisible();
  await expect(page.locator('.network-form')).toHaveCount(0);
  await page.getByRole('button',{name:'返回网络',exact:true}).last().click();
  await page.getByRole('button',{name:'全局添加',exact:true}).click();
  await page.getByRole('button',{name:'创建网络',exact:true}).click();
  await expect(page.getByRole('heading',{name:'创建网络',exact:true})).toBeVisible();
  await expect(page.locator('.network-form')).toHaveCount(1);
  await expect(page.getByPlaceholder('粘贴 Base64 网络配置')).toHaveCount(0);
});

test('network card shows QR errors, retries and copies the same configuration on a narrow screen',async({page,context})=>{
  await page.setViewportSize({width:390,height:844});
  await context.grantPermissions(['clipboard-read','clipboard-write']);
  await bridge(page,{services:true});await page.goto('/#/networks');
  await page.evaluate(()=>{window.testBridge.failQr=true;});
  await page.getByRole('button',{name:'家庭网络',exact:true}).click();
  await page.getByRole('button',{name:'网络名片',exact:true}).click();
  const dialog=page.getByRole('dialog');
  await expect(dialog.getByText('二维码生成失败或配置过长，请重试或复制配置。')).toBeVisible();
  await page.evaluate(()=>{window.testBridge.failQr=false;});
  await dialog.getByRole('button',{name:'重试二维码'}).click();
  await expect(dialog.getByRole('img',{name:'网络分享二维码'})).toBeVisible();
  await expect(dialog.getByRole('button',{name:'重试二维码'})).toHaveCount(0);
  const payload=await dialog.getByLabel('Base64 网络配置（敏感）').inputValue();
  await dialog.getByRole('button',{name:'复制配置',exact:true}).click();
  await expect(dialog.getByRole('status')).toHaveText('已复制');
  expect(await page.evaluate(()=>navigator.clipboard.readText())).toBe(payload);
  for(const [width,height] of [[320,568],[390,844],[844,390]]){
    await page.setViewportSize({width,height});
    await expect.poll(async()=>Math.round((await dialog.boundingBox()).width)).toBe(width);
    const box=await dialog.boundingBox();
    expect(Math.round(box.x)).toBe(0);expect(Math.round(box.y)).toBe(0);expect(Math.round(box.height)).toBe(height);
    await dialog.locator('.network-flow-body').evaluate(el=>{el.scrollTop=0;});
    const back=dialog.getByRole('button',{name:'返回网络',exact:true}),copy=dialog.getByRole('button',{name:'复制配置',exact:true});
    await expect(back).toHaveText('');
    const backBefore=await back.boundingBox(),copyBefore=await copy.boundingBox();
    expect(backBefore.y).toBeLessThan(30);expect(copyBefore.y+copyBefore.height).toBeLessThanOrEqual(height);
    expect(copyBefore.y).toBeGreaterThan(height-100);
    const qrBox=await dialog.getByRole('img',{name:'网络分享二维码'}).boundingBox();
    expect(qrBox.y).toBeGreaterThanOrEqual(backBefore.y+backBefore.height);
    expect(qrBox.y+qrBox.height).toBeLessThanOrEqual(copyBefore.y);
    await dialog.locator('.network-flow-body').evaluate(el=>{el.scrollTop=el.scrollHeight;});
    expect((await back.boundingBox()).y).toBe(backBefore.y);expect((await copy.boundingBox()).y).toBe(copyBefore.y);
  }
  await page.setViewportSize({width:390,height:844});
  await dialog.getByRole('button',{name:'返回网络',exact:true}).click();
  await page.getByRole('button',{name:'全局添加',exact:true}).click();
  await page.getByRole('button',{name:'加入网络',exact:true}).click();
  await expect(page.getByRole('dialog').getByPlaceholder('粘贴 Base64 网络配置')).toBeVisible();
});

test('mobile scanner decodes real QR pixels, releases the camera, and waits for import',async({page})=>{
  await bridge(page,{os:'android'});
  await page.addInitScript(({rows})=>{
    const canvas=document.createElement('canvas'),scale=8;canvas.width=canvas.height=(rows.length+8)*scale;
    const draw=()=>{const ctx=canvas.getContext('2d');ctx.fillStyle='white';ctx.fillRect(0,0,canvas.width,canvas.height);ctx.fillStyle='black';rows.forEach((row,y)=>Array.from(row).forEach((pixel,x)=>{if(pixel==='1')ctx.fillRect((x+4)*scale,(y+4)*scale,scale,scale);}));};
    Object.defineProperty(navigator,'mediaDevices',{value:{getUserMedia:async()=>{
      const stream=canvas.captureStream(10);window.testCameraStream=stream;
      const timer=setInterval(()=>{if(stream.getTracks().every(t=>t.readyState==='ended'))clearInterval(timer);else draw();},80);return stream;
    }}});
  },network_qr);
  await page.goto('/#/networks');
  await page.getByRole('button',{name:'全局添加',exact:true}).click();
  await page.getByRole('button',{name:'加入网络',exact:true}).click();
  await page.getByRole('button',{name:'扫描二维码',exact:true}).click();
  await expect(page.getByPlaceholder('粘贴 Base64 网络配置')).toHaveValue(network_qr.payload);
  expect(await page.evaluate(()=>window.testCameraStream.getTracks().every(t=>t.readyState==='ended'))).toBe(true);
  expect(await page.evaluate(()=>window.testBridge.calls.filter(c=>c.operation_id==='import_network'))).toHaveLength(0);
  await page.getByRole('button',{name:'解码并导入',exact:true}).click();
  expect(await page.evaluate(()=>window.testBridge.calls.find(c=>c.operation_id==='import_network').body)).toEqual({source:'manual',config:network_qr.config});
});

test('service directory selects networks, reveals authentication safely and only opens web protocols',async({page})=>{
  await bridge(page,{services:true});await page.goto('/#/services');
  await expect(page.getByRole('heading',{name:'音乐',exact:true})).toBeVisible();
  await expect(page.getByRole('combobox',{name:'服务所属网络'})).toHaveValue('n1');
  await expect(page.getByText('fixture-password',{exact:false})).toHaveCount(0);
  await page.getByRole('button',{name:'查看认证说明'}).click();
  await expect(page.locator('pre.secret')).toHaveText('fixture-password <script>alert(1)</script>');
  await expect(page.getByRole('button',{name:'浏览器打开'})).toHaveCount(1);
  await page.getByRole('button',{name:'浏览器打开'}).click();
  expect(await page.evaluate(()=>window.testBridge.calls.find(c=>c.command==='plugin:opener|open_url').url)).toBe('http://10.20.30.4:8000/');
  await page.getByRole('combobox',{name:'服务所属网络'}).selectOption('n2');
  await expect(page.getByText('此设备在这个网络中暂无已发布服务。')).toBeVisible();
  await expect(page.locator('pre.secret')).toHaveCount(0);
  expect(await page.evaluate(()=>window.testBridge.calls.filter(c=>['publish_service','update_service','unpublish_service'].includes(c.operation_id)))).toHaveLength(0);
});

for(const configured of [true,false])test(`service management prepares a new conversation, configured=${configured}`,async({page})=>{
  await bridge(page,{services:true,configured});await page.goto('/#/services');
  await page.getByRole('button',{name:'通过对话管理',exact:true}).click();
  if(configured){
    await expect(page.getByRole('textbox',{name:'消息',exact:true})).toHaveValue(/先检查此设备的网络和服务目录/);
    expect(await page.evaluate(()=>window.testBridge.calls.filter(c=>c.operation_id==='create_session'))).toHaveLength(1);
  }else await expect(page.getByRole('heading',{name:'模型配置',exact:true})).toBeVisible();
  expect(await page.evaluate(()=>window.testBridge.calls.filter(c=>['submit_run','publish_service','update_service','unpublish_service'].includes(c.operation_id)))).toHaveLength(0);
});

test('language switch preserves drafts, running jobs and original content, then persists',async({page})=>{
  await bridge(page,{live:true});await page.goto('/');
  const message=page.getByRole('textbox',{name:'消息',exact:true});await message.fill('我的草稿 / Do not translate');
  await page.evaluate(()=>location.hash='#/settings');
  await page.getByRole('combobox',{name:'界面语言',exact:true}).selectOption('en');
  await expect(page.locator('html')).toHaveAttribute('lang','en');
  await expect(page.getByRole('heading',{name:'Settings',exact:true})).toBeVisible();
  await expect(page.getByRole('navigation',{name:'Main navigation',exact:true}).getByRole('link',{name:'Conversations',exact:true})).toBeVisible();
  await page.evaluate(()=>{location.hash='#/sessions';window.testBridge.events.push({run_id:'r1',seq:2,kind:'assistant_delta',data:{text:'\n\n执行继续 / stream continues'}});});
  await expect(page.getByRole('textbox',{name:'Message',exact:true})).toHaveValue('我的草稿 / Do not translate');
  await expect(page.getByRole('button',{name:'音乐部署',exact:true})).toBeVisible();
  await expect(page.getByText('执行继续 / stream continues',{exact:true})).toBeVisible();
  await expect(page.getByRole('button',{name:/Copy code \d+/}).first()).toBeVisible();
  expect(await page.evaluate(()=>window.testBridge.calls.filter(c=>['submit_run','cancel_run','update_settings'].includes(c.operation_id)))).toHaveLength(0);
  await page.reload();await expect(page.locator('html')).toHaveAttribute('lang','en');
  await page.evaluate(()=>location.hash='#/settings');
  await expect(page.getByRole('combobox',{name:'Interface language',exact:true})).toHaveValue('en');
  await page.getByRole('combobox',{name:'Interface language',exact:true}).selectOption('zh-CN');
  await expect(page.getByRole('heading',{name:'设置',exact:true})).toBeVisible();
});

test('legacy linked conversation stays compatible but new sessions never bind to selected devices',async({page})=>{
  await bridge(page,{linked:true});await page.goto('/');
  const input=page.getByRole('textbox',{name:'消息',exact:true});await input.fill('keep origin draft');
  await expect.poll(()=>page.evaluate(()=>window.testBridge.calls.filter(c=>c.operation_id==='get_model_catalog').at(-1)?.target?.device_id)).toBe('00000000-0000-4000-8000-000000000222');
  await page.evaluate(()=>location.hash='#/devices');
  await expect(page.getByRole('button',{name:'选择远端设备',exact:true})).toHaveCount(0);
  await page.evaluate(()=>location.hash='#/sessions');await expect(input).toHaveValue('keep origin draft');
  await page.getByRole('button',{name:'模型配置',exact:true}).click();await page.getByRole('button',{name:'完成选择'}).click();
  await expect.poll(()=>page.evaluate(()=>window.testBridge.calls.filter(c=>c.operation_id==='get_model_catalog').at(-1)?.target?.device_id)).toBe('00000000-0000-4000-8000-000000000222');
  await page.getByRole('button',{name:'全局添加',exact:true}).click();await page.getByRole('button',{name:'新会话',exact:true}).click();
  await expect(page.locator('.chat-title')).toContainText('新会话');
  const created=await page.evaluate(()=>window.testBridge.calls.filter(c=>c.operation_id==='create_session'));
  expect(created).toHaveLength(1);expect(created[0].target).toBeUndefined();expect(created[0].body.execution_target).toBeUndefined();
  await expect.poll(()=>page.evaluate(()=>window.testBridge.calls.filter(c=>c.operation_id==='get_model_catalog').at(-1)?.target??null)).toBeNull();
  expect(await page.evaluate(()=>window.testBridge.calls.filter(c=>['list_sessions','get_run','list_runs'].includes(c.operation_id)&&c.target))).toHaveLength(0);
});

test('persisted unconfirmed remote request is visible after reload and retries exact input',async({page})=>{
  await bridge(page,{linked:true,pendingRemote:true,offline:true});await page.goto('/');
  await expect(page.getByRole('heading',{name:'结果待确认的远端请求'})).toBeVisible();
  await expect(page.getByText('执行设备暂不可达：显示上次保存的进度，不代表实时状态。').first()).toBeVisible();
  await page.reload();await expect(page.getByRole('button',{name:'重试原请求'})).toBeVisible();
  expect(await page.evaluate(()=>window.testBridge.calls.filter(c=>c.operation_id==='submit_run'))).toHaveLength(0);
  await page.getByRole('button',{name:'重试原请求'}).click();
  await expect(page.getByRole('heading',{name:'结果待确认的远端请求'})).toHaveCount(0);
  const submissions=await page.evaluate(()=>window.testBridge.calls.filter(c=>c.operation_id==='submit_run'));
  expect(submissions).toHaveLength(1);expect(submissions[0].target).toBeUndefined();
  expect(submissions[0].body).toEqual({request_id:'pending-original-id',message:'原请求，不要重复执行',network_id:'00000000-0000-4000-8000-000000000111'});
});

for(const width of [320,1440])test(`English pages and component labels adapt at ${width}px`,async({page})=>{
  await page.setViewportSize({width,height:900});await page.addInitScript(()=>localStorage.setItem('rove-gui-locale','en'));await bridge(page);await page.goto('/#/models');
  await expect(page.getByRole('heading',{name:'Model settings',exact:true})).toBeVisible();
  await page.getByRole('button',{name:'Add models',exact:true}).click();const dialog=page.getByRole('dialog',{name:'Add models',exact:true});
  await expect(dialog.getByRole('combobox',{name:'Provider',exact:true})).toBeVisible();
  await expect(dialog.getByRole('button',{name:'Close',exact:true})).toBeVisible();
  await dialog.getByRole('button',{name:'Close',exact:true}).click();
  for(const hash of ['sessions','archives','services','networks','settings','models','devices']){
    await page.evaluate(hash=>location.hash='#/'+hash,hash);
    await expect.poll(()=>page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1)).toBe(true);
  }
  await page.evaluate(()=>location.hash='#/settings');await page.screenshot({path:`test-results/gui-settings-en-${width}.png`,fullPage:true});
});

test.describe('system locale',()=>{
  test.use({locale:'fr-FR'});
  test('unsupported system language falls back to English and a stored choice overrides it',async({page})=>{
    await bridge(page);await page.goto('/#/settings');await expect(page.locator('html')).toHaveAttribute('lang','en');
    await page.getByRole('combobox',{name:'Interface language'}).selectOption('zh-CN');await page.reload();await expect(page.locator('html')).toHaveAttribute('lang','zh-CN');
    await page.getByRole('combobox',{name:'界面语言'}).selectOption('system');await expect(page.locator('html')).toHaveAttribute('lang','en');
  });
});

test('localization renders without dynamic code evaluation',async({page})=>{
  await bridge(page);
  await page.addInitScript(()=>{
    localStorage.setItem('rove-gui-locale','en');
    window.eval=()=>{throw new Error('Dynamic evaluation disabled by test');};
    window.Function=new Proxy(window.Function,{apply(){throw new Error('Function evaluation disabled by test');},construct(){throw new Error('Function construction disabled by test');}});
  });
  await page.goto('/#/models');await expect(page.getByRole('heading',{name:'Model settings',exact:true})).toBeVisible();
  await page.getByRole('navigation',{name:'Main navigation',exact:true}).getByRole('link',{name:'Conversations',exact:true}).click();
  await expect(page.getByRole('button',{name:'Copy reply',exact:true})).toBeVisible();
});

test('desktop navigation preserves per-session drafts and routes the model entry', async ({ page }) => {
  await bridge(page); await page.goto('/');
  const message = page.getByRole('textbox', { name: '消息', exact: true });
  await message.fill('音乐草稿');
  await page.getByRole('button', { name: '代码代理', exact: true }).click();
  await message.fill('代码草稿');
  await page.getByRole('navigation', { name: '主导航', exact: true }).getByRole('link', { name: '服务' }).click();
  await expect(page.getByRole('heading', { name: '服务', exact: true })).toBeVisible();
  await page.getByRole('navigation', { name: '主导航', exact: true }).getByRole('link', { name: '会话' }).click();
  await expect(message).toHaveValue('代码草稿');
  await page.getByRole('button', { name: '音乐部署', exact: true }).click();
  await expect(message).toHaveValue('音乐草稿');
  await page.getByRole('button', { name: '模型配置', exact: true }).click();
  await expect(page.getByRole('dialog',{name:'选择模型'})).toBeVisible();
  await page.getByRole('button',{name:'添加 / 管理模型'}).click();
  await expect(page.getByRole('heading', { name: '模型配置', exact: true })).toBeVisible();
  expect(await page.evaluate(() => window.testBridge.calls.filter(c => c.operation_id === 'submit_run'))).toHaveLength(0);
});

test('real model UI imports multiple selected models, renames, selects and removes a connection',async({page})=>{
  await bridge(page);await page.goto('/#/models');
  await page.getByRole('button',{name:'添加模型',exact:true}).click();
  const dialog=page.getByRole('dialog',{name:'添加模型',exact:true});
  await expect(dialog.getByRole('checkbox').first()).toBeChecked();
  await dialog.getByLabel('提供商',{exact:true}).selectOption('deepseek');
  await expect(dialog.getByLabel('接口地址',{exact:true})).toHaveValue('https://api.deepseek.com');
  await dialog.getByLabel('API 密钥',{exact:true}).fill('test-secret-private');
  await dialog.getByRole('button',{name:'获取可用型号'}).click();
  await expect(dialog.getByLabel('real-chat',{exact:true})).toBeChecked();
  await expect(dialog.getByLabel('real-fast',{exact:true})).toBeChecked();
  await dialog.getByLabel('real-chat · 配置名称').fill('我的助手');
  await dialog.getByLabel('将本次第一个型号设为默认').check();
  await dialog.getByRole('button',{name:'保存模型',exact:true}).click();
  await expect(dialog).toBeHidden();
  await expect(page.getByText('我的助手',{exact:true})).toBeVisible();
  await page.getByRole('button',{name:'改名 我的助手',exact:true}).click();
  const rename=page.getByRole('dialog',{name:'修改模型名称'});await rename.getByLabel('模型名称').fill('家庭助理');await rename.getByRole('button',{name:'保存名称'}).click();
  await page.evaluate(()=>location.hash='#/sessions');
  await page.getByRole('button',{name:'模型配置',exact:true}).click();
  await page.getByRole('combobox',{name:'本会话选用模型'}).selectOption({label:'家庭助理'});
  await page.getByRole('button',{name:'完成选择'}).click();
  await page.getByRole('textbox',{name:'消息',exact:true}).fill('验证选用模型');await page.getByRole('button',{name:'发送消息'}).click();
  await expect.poll(()=>page.evaluate(()=>window.testBridge.calls.find(c=>c.operation_id==='submit_run')?.body.model_id)).toBe('c2-m0');
  await page.evaluate(()=>location.hash='#/models');
  await page.getByRole('button',{name:'移除连接',exact:true}).last().click();await page.getByRole('button',{name:'确认移除连接'}).click();
  await expect(page.getByText('家庭助理',{exact:true})).toHaveCount(0);
  expect(await page.evaluate(()=>JSON.stringify({...localStorage}))).not.toContain('test-secret-private');
  const calls=await page.evaluate(()=>window.testBridge.calls);expect(calls.find(c=>c.operation_id==='import_models').body.models).toHaveLength(2);
});

for(const width of [320,1440])test(`API sync selects an explicit peer and confirms replacement without exposing keys at ${width}px`,async({page})=>{
  await page.setViewportSize({width,height:900});await bridge(page,{sync:true});await page.goto('/#/models');
  await expect(page.getByRole('button',{name:'同步本机 API 配置',exact:true})).toHaveCount(0);
  await page.evaluate(()=>location.hash='#/networks');
  expect(await page.evaluate(async()=>{await document.fonts.load('16px primeicons');return document.fonts.check('16px primeicons');})).toBe(true);
  await page.getByRole('button',{name:'同步测试网络',exact:true}).click();
  await page.getByRole('button',{name:'同步本机 API 配置',exact:true}).click();
  const dialog=page.getByRole('dialog',{name:'同步本机 API 配置',exact:true});
  await expect(dialog.getByLabel('本机 API 连接')).toHaveValue('c1');
  await expect(dialog.getByRole('combobox',{name:'网络',exact:true})).toHaveCount(0);
  await expect(dialog).toContainText('同步测试网络');
  await dialog.getByRole('button',{name:'选择设备',exact:true}).click();
  await expect(dialog.getByLabel('替换目标连接')).toBeVisible();
  expect(await page.evaluate(()=>window.testBridge.calls.filter(c=>c.operation_id==='sync_model_connection'))).toHaveLength(0);
  await dialog.getByRole('button',{name:'确认同步',exact:true}).click();
  await expect(dialog.getByRole('alert')).toContainText('目标已有冲突配置');
  await dialog.getByLabel('替换目标连接').selectOption('remote-c');
  await expect(dialog.getByRole('button',{name:'确认同步',exact:true})).toBeDisabled();
  await dialog.getByRole('checkbox').check();
  await dialog.getByRole('button',{name:'确认同步',exact:true}).click();
  await expect(dialog.getByRole('status')).toContainText('目标设备已保存');
  const calls=await page.evaluate(()=>window.testBridge.calls.filter(c=>c.operation_id==='sync_model_connection'));
  expect(calls).toHaveLength(2);expect(calls[1].target).toBeUndefined();
  expect(calls[1].body).toEqual({target:{network_id:'00000000-0000-4000-8000-000000000111',device_id:'00000000-0000-4000-8000-000000000222'},overwrite:true,replace_connection_id:'remote-c'});
  expect(JSON.stringify(calls)).not.toContain('api_key');
  expect(await dialog.evaluate(el=>el.scrollWidth<=el.clientWidth+1)).toBe(true);
});

test('first model opens the agent-created network onboarding conversation',async({page})=>{
  await bridge(page,{configured:false,empty:true});await page.goto('/#/models');
  await page.getByRole('button',{name:'添加模型',exact:true}).click();
  const dialog=page.getByRole('dialog',{name:'添加模型',exact:true});
  await dialog.getByRole('button',{name:'保存模型',exact:true}).click();
  await expect(page.locator('.chat-title')).toHaveText('初始网络的会话');
  await expect(page.getByText('rove-agent · 初始网络引导',{exact:true})).toBeVisible();
  expect(await page.evaluate(()=>window.testBridge.calls.filter(c=>c.operation_id==='create_session'))).toHaveLength(0);
  await page.getByRole('button',{name:'加入网络',exact:true}).click();
  await expect(page.locator('.topbar strong')).toHaveText('网络');
  await expect(page.getByRole('button',{name:'刷新',exact:true})).toBeVisible();
});

for (const width of [320, 390, 850, 1440]) test(`responsive layout and dark theme at ${width}px`, async ({ page }) => {
  await page.setViewportSize({ width, height: 900 });
  await bridge(page); await page.goto('/');
  await page.getByRole('button',{name:'音乐部署',exact:true}).click();
  await expect(page.getByRole('heading', { name: '部署建议' })).toBeVisible();
  await page.screenshot({ path: `test-results/gui-chat-${width}.png`, fullPage: true });
  await expect(page.locator('.bottom-nav')).toBeHidden();
  for (const hash of ['sessions', 'networks', 'services', 'settings', 'models', 'devices']) {
    await page.evaluate(hash => { location.hash = '#/' + hash; }, hash);
    await expect.poll(() => page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1)).toBe(true);
  }
  await page.getByRole('button', { name: '切换明暗主题' }).click();
  await expect(page.locator('html')).toHaveClass('dark');
  await page.reload(); await expect(page.locator('html')).toHaveClass('dark');
});

test('real output is rich, has source-copy fallback and local-only speech fallback', async ({ page }) => {
  await bridge(page);
  await page.addInitScript(() => {
    Object.defineProperty(navigator, 'clipboard', { value: { writeText: async () => { throw new Error('denied'); } } });
    Object.defineProperty(window, 'speechSynthesis', { value: { getVoices: () => [], cancel() {} } });
  });
  await page.goto('/');
  await expect(page.locator('.katex').first()).toBeVisible();
  await page.getByRole('button', { name: /^复制代码/ }).click();
  await expect(page.getByRole('textbox', { name: '手动复制文本' })).toHaveValue('df -h\n');
  await page.getByRole('button', { name: '朗读回复' }).click();
  await expect(page.getByText(/不会把回复发送到云端朗读/)).toBeVisible();
  await expect(page.locator('progress')).toHaveCount(0);
});

test('mobile native host retains bottom tabs in landscape', async ({ page }) => {
  await page.setViewportSize({ width: 1200, height: 800 });
  await bridge(page, { os: 'android' }); await page.goto('/');
  await expect(page.getByRole('navigation', { name: '移动主导航' })).toBeVisible();
  await expect(page.getByRole('navigation', { name: '主导航', exact: true })).toBeHidden();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1)).toBe(true);
});

test('mobile conversation list and detail are separate without losing drafts', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await bridge(page); await page.goto('/');
  const message = page.getByRole('textbox', { name: '消息', exact: true });
  await expect(page.getByRole('complementary',{name:'会话列表'})).toBeVisible();
  await page.getByRole('button',{name:'音乐部署',exact:true}).click();
  await message.fill('保留这条草稿');
  await expect(page.getByRole('complementary', { name: '会话列表' })).toBeHidden();
  await page.getByRole('button', { name: '返回会话列表' }).click();
  await expect(message).toBeHidden();
  await page.getByRole('button', { name: '代码代理', exact: true }).click();
  await expect(message).toHaveValue('');
  await page.getByRole('button', { name: '返回会话列表' }).click();
  await page.getByRole('button', { name: '音乐部署', exact: true }).click();
  await expect(message).toHaveValue('保留这条草稿');
});

test('stream continues through navigation, recovers snapshot after disconnect and cancels without resubmit', async ({ page }) => {
  await bridge(page, { live: true }); await page.goto('/');
  await expect(page.getByText('正在接收输出…')).toBeVisible();
  await page.evaluate(() => { location.hash = '#/services'; window.testBridge.events.push({ run_id: 'r1', seq: 2, kind: 'assistant_delta', data: { text: '\n\n流式片段' } }); });
  await expect.poll(() => page.evaluate(() => window.testBridge.calls.some(c => c.command === 'agent_events' && c.afterSeq === 2))).toBe(true);
  await page.evaluate(() => { location.hash = '#/sessions'; });
  await expect(page.getByText('流式片段', { exact: true })).toBeVisible();
  await page.evaluate(() => { window.testBridge.snapshot.snapshot_seq = 3; window.testBridge.snapshot.output_tail = '恢复后的快照'; window.testBridge.failEvents = true; });
  await expect(page.getByText('恢复后的快照', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: '取消此作业' }).click();
  await expect(page.getByRole('button', { name: '取消此作业' })).toHaveCount(0);
  const calls = await page.evaluate(() => window.testBridge.calls);
  expect(calls.filter(c => c.operation_id === 'cancel_run')).toHaveLength(1);
  expect(calls.filter(c => c.operation_id === 'submit_run')).toHaveLength(0);
});

test('lost submit response keeps request ID across session switches', async ({ page }) => {
  await bridge(page, { loseSubmit: true }); await page.goto('/');
  await page.getByRole('textbox', { name: '消息', exact: true }).fill('同一个请求');
  await page.getByRole('button', { name: '发送消息' }).click();
  await expect(page.getByRole('alert')).toContainText('test response lost');
  await page.getByRole('button', { name: '代码代理', exact: true }).click();
  await page.getByRole('button', { name: '音乐部署', exact: true }).click();
  await page.getByRole('button', { name: '发送消息' }).click();
  const calls = await page.evaluate(() => window.testBridge.calls.filter(c => c.operation_id === 'submit_run'));
  expect(calls).toHaveLength(2);
  expect(calls[0].body.request_id).toBe(calls[1].body.request_id);
  expect(calls[0].target).toBeUndefined();
});

test('welcome suggestions only prepare input; service page has no mutation controls', async ({ page }) => {
  await bridge(page, { empty: true }); await page.goto('/');
  await expect(page.getByText('数据自己掌握，跳出平台控制')).toBeVisible();
  await page.getByRole('button', { name: /建立自己的私人影院/ }).click();
  await expect(page.getByRole('textbox', { name: '消息', exact: true })).toHaveValue(/先不要执行安装/);
  await page.getByRole('textbox', { name: '消息', exact: true }).fill('已有草稿');
  await page.getByRole('button', { name: /管理多台代码代理/ }).click();
  await expect(page.getByRole('textbox', { name: '消息', exact: true })).toHaveValue(/^已有草稿\n帮我/);
  await page.evaluate(() => { location.hash = '#/services'; });
  await expect(page.getByRole('button', { name: '通过对话管理' })).toBeVisible();
  await expect(page.getByRole('button', { name: /发布服务|取消发布|修改发布/ })).toHaveCount(0);
  expect(await page.evaluate(() => window.testBridge.calls.filter(c => ['submit_run', 'publish_service', 'update_service', 'unpublish_service'].includes(c.operation_id)))).toHaveLength(0);
});

for(const configured of [true,false])test(`manage Rove prepares a local draft with configured=${configured}`,async({page})=>{
  await bridge(page,{configured});await page.goto('/#/settings');
  await page.getByRole('button',{name:'通过对话管理 Rove',exact:true}).click();
  if(configured){
    await expect(page.getByRole('textbox',{name:'消息',exact:true})).toHaveValue(/先只读检查.*不要删除文件、迁移存储或修改配置.*等我确认/);
    const created=await page.evaluate(()=>window.testBridge.calls.filter(c=>c.operation_id==='create_session'));
    expect(created).toHaveLength(1);expect(created[0].target).toBeUndefined();
  }else{
    await expect(page.getByRole('heading',{name:'模型配置',exact:true})).toBeVisible();
    expect(await page.evaluate(()=>window.testBridge.calls.filter(c=>c.operation_id==='create_session'))).toHaveLength(0);
  }
  expect(await page.evaluate(()=>window.testBridge.calls.filter(c=>c.operation_id==='submit_run'))).toHaveLength(0);
});

for(const width of [390,1440])test(`archive, restore drafts and confirmed delete at ${width}px`,async({page})=>{
  await page.setViewportSize({width,height:900});await bridge(page);await page.goto('/');
  const message=page.getByRole('textbox',{name:'消息',exact:true});
  const back=async()=>{if(width<850)await page.getByRole('button',{name:'返回会话列表',exact:true}).last().click();};
  const archive=async()=>{await back();await page.getByRole('button',{name:'归档 音乐部署',exact:true}).click();};
  const archives=async()=>{await page.getByRole('button',{name:'全局添加',exact:true}).click();await expect(page.locator('.global-menu').getByRole('button',{name:'归档会话',exact:true})).toHaveCount(0);await page.locator('.global-menu').getByRole('button',{name:'归档管理',exact:true}).click();};
  await page.getByRole('button',{name:'音乐部署',exact:true}).click();await message.fill('保存草稿');
  await archive();await expect(page.getByRole('button',{name:'音乐部署',exact:true})).toHaveCount(0);
  await archives();await page.getByRole('button',{name:'音乐部署',exact:true}).click();await expect(message).toHaveCount(0);
  await back();await page.getByRole('button',{name:'恢复 音乐部署',exact:true}).click();
  await page.locator('.conversation-list').getByRole('button',{name:'返回会话列表',exact:true}).click();
  await page.getByRole('button',{name:'音乐部署',exact:true}).click();await expect(message).toHaveValue('保存草稿');
  await archive();await archives();
  await expect(page.getByRole('button',{name:'删除 音乐部署',exact:true})).toBeVisible();
  await page.getByRole('button',{name:'删除 音乐部署',exact:true}).click();
  await expect(page.getByRole('dialog')).toContainText('不可恢复');
  await page.getByRole('button',{name:'取消',exact:true}).click();await expect(page.getByRole('button',{name:'音乐部署',exact:true})).toBeVisible();
  await page.getByRole('button',{name:'删除 音乐部署',exact:true}).click();
  await page.getByRole('button',{name:'确认永久删除'}).click();
  await expect(page.getByRole('button',{name:'音乐部署',exact:true})).toHaveCount(0);
  const calls=await page.evaluate(()=>window.testBridge.calls);
  expect(calls.filter(c=>c.operation_id==='delete_session')).toHaveLength(1);
  expect(calls.filter(c=>c.operation_id==='submit_run')).toHaveLength(0);
});
