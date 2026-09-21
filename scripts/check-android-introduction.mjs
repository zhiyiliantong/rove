// Inspect only the explicitly forwarded Rove Debug WebView. No model calls or data resets.
import {mkdirSync,writeFileSync,readFileSync} from 'node:fs';
import {join} from 'node:path';
import assert from 'node:assert/strict';
const output=process.argv[2],baseline=process.argv[3];
if(!output||!baseline)throw Error('Usage: node scripts/check-android-introduction.mjs <new-output-dir> <before.json>');
mkdirSync(output,{recursive:false});
const before=JSON.parse(readFileSync(baseline,'utf8'));
const pages=await(await fetch('http://127.0.0.1:19222/json')).json();
const page=pages.find(p=>{try{return new URL(p.url).origin==='http://tauri.localhost'&&p.title.startsWith('Rove');}catch{return false;}});
if(!page)throw Error('Rove WebView not found');
const ws=new WebSocket(page.webSocketDebuggerUrl);let next=0;const requests=new Map();
ws.addEventListener('message',event=>{const response=JSON.parse(event.data);requests.get(response.id)?.(response);});
await new Promise((resolve,reject)=>{ws.addEventListener('open',resolve,{once:true});ws.addEventListener('error',reject,{once:true});});
async function command(method,params={}){const id=++next;return new Promise((resolve,reject)=>{const timer=setTimeout(()=>{requests.delete(id);reject(Error('CDP timed out'));},15000);requests.set(id,response=>{clearTimeout(timer);requests.delete(id);if(response.error||response.result?.exceptionDetails)reject(Error(JSON.stringify(response.error??response.result.exceptionDetails)));else resolve(response.result);});ws.send(JSON.stringify({id,method,params}));});}
async function evaluate(fn){return(await command('Runtime.evaluate',{expression:`(${fn.toString()})()`,awaitPromise:true,returnByValue:true})).result.value;}
async function screenshot(name){await command('Page.enable');const result=await command('Page.captureScreenshot',{format:'png'});writeFileSync(join(output,name+'.png'),Buffer.from(result.data,'base64'),{flag:'wx'});}
const report={pages:[],baseline_preserved:false,replay_pending:false};
try{
  for(let i=0;i<4;i++){
    await command('Runtime.evaluate',{expression:`document.querySelectorAll('.intro-progress button')[${i}].click();window.scrollTo(0,0)`});
    await new Promise(resolve=>setTimeout(resolve,250));
    const state=await evaluate(()=>({title:document.querySelector('.intro-content h1')?.textContent,width:innerWidth,height:innerHeight,scrollWidth:document.documentElement.scrollWidth,step:document.querySelector('.intro-progress [aria-current]')?.textContent}));
    assert.ok(state.title);assert.ok(state.scrollWidth<=state.width);report.pages.push(state);await screenshot('step-'+(i+1));
  }
  // This acceptance path deliberately does not call any configured provider.
  report.skip_disabled=await evaluate(()=>[...document.querySelectorAll('.intro-header button')].find(b=>b.textContent==='跳过引导').disabled);
  assert.equal(report.skip_disabled,true);
  await evaluate(()=>[...document.querySelectorAll('.intro-header button')].find(b=>b.textContent==='跳过引导').click());
  assert.equal(await evaluate(()=>document.querySelector('.introduction')!==null),true);
  await evaluate(()=>document.querySelectorAll('.intro-progress button')[1].click());
  await new Promise(resolve=>setTimeout(resolve,200));
  await evaluate(()=>document.querySelector('.intro-actions button').click());
  await new Promise(resolve=>setTimeout(resolve,200));
  // Existing configuration opens its catalog, not a new duplicate connection.
  await evaluate(()=>{if(!document.querySelector('input[aria-label="API 密钥"]'))[...document.querySelectorAll('.p-dialog .model-panel > .actions button')].find(b=>b.textContent==='添加模型').click();});
  await new Promise(resolve=>setTimeout(resolve,200));
  await evaluate(()=>{const p=document.querySelector('select[aria-label="提供商"]');p.value='deepseek';p.dispatchEvent(new Event('change',{bubbles:true}));});
  await new Promise(resolve=>setTimeout(resolve,200));
  report.model_form=await evaluate(()=>{const key=document.querySelector('input[aria-label="API 密钥"]'),content=key.closest('.p-dialog-content');return {key_type:key.type,key_empty:key.value==='',show_button:!!document.querySelector('button[aria-label="显示 API 密钥"]'),test_button:[...content.querySelectorAll('button')].some(b=>b.textContent==='测试模型'),scrollable:content.scrollHeight>content.clientHeight,width:content.clientWidth,scrollWidth:content.scrollWidth};});
  assert.equal(report.model_form.key_type,'password');assert.equal(report.model_form.key_empty,true);assert.equal(report.model_form.show_button,true);assert.equal(report.model_form.test_button,true);assert.equal(report.model_form.scrollable,true);assert.ok(report.model_form.scrollWidth<=report.model_form.width);
  report.save_requires_test=await evaluate(()=>[...document.querySelectorAll('.p-dialog button')].find(b=>b.textContent==='保存模型').disabled);
  assert.equal(report.save_requires_test,true);
  await screenshot('model-form');
  await evaluate(()=>document.querySelector('.model-test').scrollIntoView({block:'center'}));
  await new Promise(resolve=>setTimeout(resolve,200));await screenshot('model-test-control');
  await evaluate(()=>document.querySelector('button[aria-label="显示 API 密钥"]').click());
  assert.equal(await evaluate(()=>document.querySelector('input[aria-label="API 密钥"]').type),'text');
  await evaluate(()=>document.querySelector('button[aria-label="隐藏 API 密钥"]').click());
  await evaluate(()=>document.querySelector('select[aria-label="提供商"]').closest('.p-dialog-content').scrollTo(0,0));
  report.after=await evaluate(async()=>{
    const call=async operation_id=>{const result=await window.__TAURI_INTERNALS__.invoke('agent_call',{request:{kind:'request',correlation_id:crypto.randomUUID(),operation_id,path_parameters:{},query_parameters:{}}});if(result.status_code>=400)throw Error('Read failed: '+operation_id);return result.body;};
    const device=await call('get_device'),sessions=await call('list_sessions'),models=await call('get_model_catalog');
    return {device_id:device.device_id,sessions:sessions.items.map(x=>({id:x.session_id,title:x.title})),models:models.models.map(x=>({id:x.model_id,name:x.name})),locale:localStorage.getItem('rove-gui-locale')};
  });
  for(const key of ['device_id','sessions','models','locale'])assert.deepEqual(report.after[key],before[key]);
  report.baseline_preserved=true;
  writeFileSync(join(output,'result.json'),JSON.stringify(report,null,2)+'\n',{flag:'wx',mode:0o600});
  console.log(JSON.stringify(report,null,2));
}finally{ws.close();}
