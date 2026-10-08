import {test,expect} from '@playwright/test';
import {bridge} from './bridge.mjs';

for(const viewport of [{width:1100,height:760},{width:320,height:640},{width:390,height:844},{width:844,height:390}])test(`network editing ${viewport.width}x${viewport.height}`,async({page},info)=>{
  await page.setViewportSize(viewport);
  await bridge(page,{services:true,os:viewport.width===1100?'windows':'android'});
  await page.addInitScript(()=>{
    const invoke=window.__TAURI_INTERNALS__.invoke;
    window.__TAURI_INTERNALS__.invoke=async(cmd,args)=>{
      const r=args?.request;
      if(['stop_network','start_network'].includes(r?.operation_id)){window.testBridge.calls.push(structuredClone(r));window.testBridge.editRunning=r.operation_id==='start_network';return {status_code:202,body:{state:window.testBridge.editRunning?'starting':'stopping'}};}
      if(r?.operation_id==='update_network'){window.testBridge.calls.push(structuredClone(r));localStorage.setItem('edit-fixture',JSON.stringify(r.body));return {status_code:200,body:{network_id:'n1',display_name:r.body.display_name,state:'stopped'}};}
      const result=await invoke(cmd,args);
      const saved=JSON.parse(localStorage.getItem('edit-fixture')||'null');
      if(r?.operation_id==='get_network'&&r.path_parameters.network_id==='n1')Object.assign(result.body,{display_name:saved?.display_name??'家庭网络',state:window.testBridge.editRunning?'running':'stopped'});
      if(r?.operation_id==='get_network_join_config'&&saved)Object.assign(result.body,saved);
      return result;
    };
  });
  await page.goto('/#/networks/n1');
  await expect(page.getByRole('button',{name:'管理此设备',exact:true})).toHaveCount(0);
  await page.getByRole('button',{name:'网络名片',exact:true}).click();
  await expect(page.locator('.network-card textarea')).not.toHaveValue('');
  const oldCard=await page.locator('.network-card textarea').inputValue();
  await page.getByRole('dialog').getByRole('button',{name:'返回网络',exact:true}).click();
  await page.getByRole('button',{name:'编辑网络',exact:true}).click();
  const dialog=page.getByRole('dialog'),key=dialog.getByLabel('网络密钥');
  await expect(key).toHaveAttribute('readonly','');
  await expect(key).toHaveAttribute('type','password');
  await expect(dialog.getByRole('button',{name:'重新生成',exact:true})).toHaveCount(0);
  await dialog.getByRole('button',{name:'显示',exact:true}).click();
  await expect(key).toHaveValue('fixture-secret');
  await dialog.getByRole('button',{name:'隐藏',exact:true}).click();
  await dialog.getByLabel('网络名称',{exact:true}).fill('客厅网络');
  await dialog.getByLabel('初始节点 1',{exact:true}).fill('https://directory.example/node');
  await dialog.getByRole('button',{name:'添加初始节点',exact:true}).click();
  await dialog.getByLabel('初始节点 2',{exact:true}).fill('txt://bootstrap.example.com');
  await dialog.getByRole('button',{name:'保存修改',exact:true}).scrollIntoViewIfNeeded();
  expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
  await page.screenshot({path:info.outputPath('edit.png')});
  await dialog.getByRole('button',{name:'保存修改',exact:true}).click();
  await expect(dialog).toHaveCount(0);
  await expect(page.getByRole('heading',{name:'客厅网络',exact:true})).toBeVisible();
  const saved=await page.evaluate(()=>window.testBridge.calls.find(c=>c.operation_id==='update_network'));
  expect(saved.target).toBeUndefined();expect(saved.path_parameters.network_id).toBe('n1');
  expect(saved.body.easytier).toMatchObject({network_name:'home',network_secret:'fixture-secret',bootstrap_peers:['https://directory.example/node','txt://bootstrap.example.com']});
  await page.getByRole('button',{name:'网络名片',exact:true}).click();
  await expect(dialog.locator('.qr svg')).toBeVisible();
  const newCard=await dialog.locator('textarea').inputValue();
  expect(newCard).not.toBe(oldCard);
  const decoded=JSON.parse(Buffer.from(newCard,'base64url').toString('utf8'));
  expect(decoded.display_name).toBe('客厅网络');
  expect(decoded.easytier).toEqual(saved.body.easytier);
  const qrCalls=await page.evaluate(()=>window.testBridge.calls.filter(c=>c.command==='network_config_qr'));
  expect(qrCalls.map(c=>c.config)).toEqual([oldCard,newCard]);
  await expect(dialog).toHaveCSS('opacity','1');
  await page.screenshot({path:info.outputPath('updated-card.png'),animations:'disabled'});
  await dialog.getByRole('button',{name:'返回网络',exact:true}).click();
  await page.reload();await page.getByRole('button',{name:'编辑网络',exact:true}).click();
  await expect(dialog.getByLabel('网络名称',{exact:true})).toHaveValue('客厅网络');
  await expect(dialog.getByLabel('初始节点 2',{exact:true})).toHaveValue('txt://bootstrap.example.com');
  await dialog.getByRole('button',{name:'返回网络',exact:true}).click();
  await page.evaluate(()=>window.testBridge.editRunning=true);
  await page.locator('.network-page-heading').getByRole('button',{name:'刷新',exact:true}).click();
  await page.getByRole('button',{name:'编辑网络',exact:true}).click();
  await expect(dialog.getByRole('button',{name:'保存并重启网络',exact:true})).toBeEnabled();
  await expect(dialog.getByRole('status')).toContainText('连接会短暂中断');
  await expect(key).toHaveAttribute('readonly','');
  await dialog.getByRole('button',{name:'保存并重启网络',exact:true}).scrollIntoViewIfNeeded();
  await page.screenshot({path:info.outputPath('running.png')});
  await dialog.getByRole('button',{name:'保存并重启网络',exact:true}).click();
  await expect(dialog).toHaveCount(0);
  const mutations=await page.evaluate(()=>window.testBridge.calls.filter(c=>['stop_network','update_network','start_network'].includes(c.operation_id)).map(c=>c.operation_id));
  expect(mutations).toEqual(['stop_network','update_network','start_network']);
  expect(await page.evaluate(()=>window.testBridge.editRunning)).toBe(true);
});

test('late QR generation cannot overwrite a different network card',async({page})=>{
  await bridge(page,{services:true});
  await page.addInitScript(()=>{
    const invoke=window.__TAURI_INTERNALS__.invoke;let qr=0;
    window.__TAURI_INTERNALS__.invoke=async(cmd,args)=>{
      if(cmd==='network_config_qr'){
        qr++;
        if(qr===1)return new Promise(resolve=>{window.releaseOldQr=()=>resolve('<svg data-generation="old" viewBox="0 0 1 1"></svg>');});
        return '<svg data-generation="new" viewBox="0 0 1 1"></svg>';
      }
      return invoke(cmd,args);
    };
  });
  await page.goto('/#/networks/n1');
  await page.getByRole('button',{name:'网络名片',exact:true}).click();
  await expect.poll(()=>page.evaluate(()=>typeof window.releaseOldQr)).toBe('function');
  await page.evaluate(()=>location.hash='#/networks/n2');
  await page.getByRole('button',{name:'网络名片',exact:true}).click();
  await expect(page.locator('.qr svg')).toHaveAttribute('data-generation','new');
  const payload=await page.locator('.network-card textarea').inputValue();
  expect(JSON.parse(Buffer.from(payload,'base64url').toString()).network_id).toBe('n2');
  await page.evaluate(()=>window.releaseOldQr());
  await expect(page.locator('.qr svg')).toHaveAttribute('data-generation','new');
  await expect(page.locator('.network-card textarea')).toHaveValue(payload);
});
