import {test,expect} from '@playwright/test';
import {seed} from '../../src/mock';
for(const width of [390,1440]){
  test(`six existing models are only removed on confirmation and stay empty after reload at ${width}`,async({page})=>{
    const state=seed('daily');state.models=Array.from({length:6},(_,i)=>({...state.models[0]!,id:`model-${i}`,name:`model-${i}`}));
    await page.addInitScript(snapshot=>{
      if(!sessionStorage.getItem('reset-test-seeded')){
        localStorage.setItem('rove-prototype-v1',JSON.stringify(snapshot));
        localStorage.setItem('rove-prototype-introduction-v1',JSON.stringify({version:1,completed:false,step:1,resetPending:false}));
        localStorage.setItem('rove-prototype-theme','dark');localStorage.setItem('unrelated','keep');sessionStorage.setItem('reset-test-seeded','1');
      }
    },state);
    await page.setViewportSize({width,height:900});await page.goto('/#/review-reset');
    const modal=page.getByRole('dialog',{name:'清空演示数据并重新开始？'});
    await expect(modal).toContainText('6 个模型型号');
    await expect(modal.getByRole('button',{name:'取消，保留数据'})).toBeFocused();
    await modal.getByRole('button',{name:'取消，保留数据'}).click();
    await expect(page.getByText('已添加 6 个型号，现有配置会保留。')).toBeVisible();
    await page.getByRole('button',{name:'清空演示数据并重新开始',exact:true}).click();
    await modal.getByRole('button',{name:'确认清空并重新开始'}).click();
    await expect(page.locator('.intro-step-0')).toBeVisible();await expect(modal).toBeHidden();
    await page.reload();await page.getByRole('button',{name:'开始',exact:true}).click();
    await expect(page.getByRole('heading',{name:'从一个模型开始'})).toBeVisible();
    await expect(page.getByText('模型已准备好',{exact:true})).toHaveCount(0);
    const saved=await page.evaluate(()=>JSON.parse(localStorage.getItem('rove-prototype-v1')!));
    for(const key of ['models','connections','networks','devices','services','sessions','runs'])expect(saved[key]).toHaveLength(0);
    expect(saved.initialized).toBe(false);
    expect(await page.evaluate(()=>localStorage.getItem('unrelated'))).toBe('keep');await expect(page.locator('html')).toHaveClass('dark');
    expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1)).toBe(true);
  });
}
test('storage rejection keeps the six-model reset dialog open and never claims success',async({page})=>{
  await page.goto('/#/review-reset');await page.evaluate(()=>{Storage.prototype.setItem=()=>{throw Error('blocked');};});
  await page.getByRole('button',{name:'确认清空并重新开始'}).click();
  await expect(page.getByRole('alert')).toContainText('未能完成清空');await expect(page.getByRole('dialog')).toBeVisible();
});
