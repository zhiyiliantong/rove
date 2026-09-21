// Render the prototype's own UI animation; no generated artwork or real network data.
// Usage: node scripts/export-network-tutorial.mjs <base-url> <new-output.gif>
import { chromium } from '@playwright/test';
import { mkdtemp, mkdir, access } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
const base = process.argv[2] ?? 'http://10.1.2.237:4173';
const output = resolve(process.argv[3] ?? 'public/tutorial/network-join-v1.gif');
try { await access(output); throw new Error(`Refusing to overwrite ${output}`); }
catch (e) { if (e.code !== 'ENOENT') throw e; }
await mkdir(dirname(output), { recursive: true });
const frames = await mkdtemp(join(tmpdir(), 'rove-network-tutorial-'));
const browser = await chromium.launch({ executablePath: process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH });
try {
  const page = await browser.newPage({ viewport: { width: 1000, height: 1000 }, deviceScaleFactor: 1 });
  await page.goto(base);
  await page.getByRole('navigation', {name:'引导步骤'}).getByRole('button', {name:'3 连接设备'}).click();
  await page.getByRole('button',{name:'暂停连接动画'}).click();
  await page.evaluate(() => document.fonts.ready);
  const labels=['1 创建你的网络','2 分享网络名片','3 另一台设备加入','4 设备，从此相连'];
  let count=0;
  for(const label of labels) {
    await page.getByRole('list',{name:'连接设备步骤'}).getByRole('button',{name:label}).click();
    for(let frame=0;frame<16;frame++) {
      // Keep transitions and the small moving connection dot visible in the exported sequence.
      await page.locator('.film-link').evaluateAll((links,time)=>{
        for(const link of links) for(const animation of link.getAnimations({subtree:true})) { animation.pause(); animation.currentTime=time; }
      },frame*125);
      await page.locator('.network-film').screenshot({path:join(frames,`${String(count++).padStart(3,'0')}.png`),animations:'allow'});
      await page.waitForTimeout(80);
    }
  }
  const result=spawnSync('ffmpeg',['-hide_banner','-loglevel','error','-n','-framerate','8','-i',join(frames,'%03d.png'),'-filter_complex','[0:v]split[a][b];[a]palettegen=stats_mode=diff[p];[b][p]paletteuse=dither=bayer','-loop','0',output],{encoding:'utf8'});
  if(result.status!==0) throw new Error(result.stderr || 'ffmpeg failed');
  console.log(JSON.stringify({output,frames,frame_count:count,fps:8,duration_seconds:count/8}));
} finally { await browser.close(); }
