import { chromium } from '@playwright/test';
import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import { resolve, join } from 'node:path';
const base=process.argv[2] ?? 'http://10.1.2.237:4173';
const output=resolve(process.argv[3] ?? 'test-results/introduction-review');
await mkdir(output,{recursive:false});
const browser=await chromium.launch({executablePath:process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH});
const results=[];
try {
  for(const width of [320,390,844,1440]) {
    const context=await browser.newContext({viewport:{width,height:width===844?390:900},hasTouch:width<900});
    const page=await context.newPage(); const errors=[],external=[];
    page.on('pageerror',e=>errors.push(e.message));
    page.on('request',r=>{if(r.url().startsWith('http')&&new URL(r.url()).origin!==new URL(base).origin)external.push(r.url());});
    await page.goto(base); await page.locator('.intro-content h1').waitFor();
    for(const [index,label] of ['1 欢迎','2 添加模型','3 连接设备','4 开始对话'].entries()) {
      await page.getByRole('navigation',{name:'引导步骤'}).getByRole('button',{name:label}).click();
      if(index===2) { await page.getByRole('button',{name:'暂停连接动画'}).click(); await page.getByRole('list',{name:'连接设备步骤'}).getByRole('button',{name:'3 另一台设备加入'}).click(); }
      await page.waitForTimeout(650); // Capture the settled presentation, not the middle of a CSS transition.
      assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1),true,`overflow ${width}/${index}`);
      await page.screenshot({path:join(output,`${width}-step-${index+1}.png`),fullPage:true});
    }
    await page.getByRole('button',{name:'进入会话列表'}).click();
    await page.getByRole('button',{name:'全局添加'}).click();
    await page.getByRole('menuitem',{name:'归档管理',exact:true}).waitFor();
    await page.screenshot({path:join(output,`${width}-list-menu.png`),fullPage:true});
    await page.getByRole('menuitem',{name:'设置',exact:true}).click();
    await page.getByRole('button',{name:'重置使用引导'}).click();
    await page.reload(); await page.locator('.intro-step-0').waitFor();
    await page.evaluate(()=>document.documentElement.classList.add('dark'));
    await page.getByRole('navigation',{name:'引导步骤'}).getByRole('button',{name:'3 连接设备'}).click();
    await page.getByRole('button',{name:'暂停连接动画'}).click();
    await page.screenshot({path:join(output,`${width}-network-dark.png`),fullPage:true});
    const gif=await page.request.get(`${base}/tutorial/network-join-v1.gif`);
    assert.equal(gif.status(),200); assert.ok(gif.headers()['content-type'].startsWith('image/gif'));
    assert.deepEqual(errors,[]); assert.deepEqual(external,[]);
    results.push({width,pages:4,replay:true,gif:true,errors,external});
    await context.close();
  }
  await writeFile(join(output,'result.json'),JSON.stringify({base,results},null,2),{flag:'wx'});
  console.log(JSON.stringify({output,results}));
} finally {await browser.close();}
