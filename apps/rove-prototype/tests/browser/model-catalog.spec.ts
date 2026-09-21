import { test, expect } from './fixtures';
async function openModels(page: import('@playwright/test').Page) { await page.goto('/#/models'); await page.getByRole('button',{name:'添加模型',exact:true}).click(); }
test('all preset models are selected by default, users can deselect and switching selects only the new vendor',async({page})=>{
 await openModels(page);await expect(page.locator('.catalog-list input:checked')).toHaveCount(4);await expect(page.locator('.model-name-field input')).toHaveCount(4);
 await page.getByRole('checkbox',{name:/GPT-5.6 Luna/}).uncheck();await expect(page.locator('.catalog-list input:checked')).toHaveCount(3);
 await page.getByRole('combobox',{name:'服务商',exact:true}).click();await page.getByRole('option',{name:'DeepSeek',exact:true}).click();await expect(page.locator('.catalog-list input:checked')).toHaveCount(2);
 await expect(page.getByRole('textbox',{name:'模型名称 deepseek-flash',exact:true})).toHaveValue('deepseek-flash-01');
});
test('generated names can be edited separately, reset, validated and restored on reload',async({page})=>{
 await page.setViewportSize({width:390,height:844}); await openModels(page);
 await page.getByRole('checkbox',{name:/GPT-6 Astra/}).check();await page.getByRole('checkbox',{name:/GPT-5.6 Sol/}).check();
 const first=page.getByRole('textbox',{name:'模型名称 gpt-6-astra',exact:true}),second=page.getByRole('textbox',{name:'模型名称 gpt-5.6-sol',exact:true});
 await expect(first).toHaveValue('openai-gpt-6-astra-01');await first.fill('写代码的助手');await second.fill('快速问答');
 await page.getByRole('checkbox',{name:/GPT-5.6 Sol/}).uncheck();await expect(first).toHaveValue('写代码的助手');await page.getByRole('checkbox',{name:/GPT-5.6 Sol/}).check();await expect(second).toHaveValue('快速问答');
 await page.getByRole('button',{name:'恢复自动名称 gpt-5.6-sol',exact:true}).click();await expect(second).toHaveValue('openai-gpt-5.6-sol-01');
 await first.fill('  ');await page.getByRole('button',{name:'保存模型',exact:true}).click();await expect(page.getByRole('alert')).toContainText('名称不能为空');
 await first.fill('custom-demo-chat-01');await page.getByRole('button',{name:'保存模型',exact:true}).click();await expect(page.getByRole('alert')).toContainText('名称重复');
 await first.fill('写代码的助手');await second.fill('写代码的助手');await page.getByRole('button',{name:'保存模型',exact:true}).click();await expect(page.getByRole('alert')).toContainText('名称重复');
 await second.fill('快速问答');await expect.poll(()=>page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1)).toBe(true);
 await page.getByRole('button',{name:'保存模型',exact:true}).click();await page.reload();await expect(page.locator('.model-row').filter({hasText:'写代码的助手'})).toBeVisible();await expect(page.locator('.model-row').filter({hasText:'快速问答'})).toBeVisible();
 const stored=await page.evaluate(()=>JSON.parse(localStorage.getItem('rove-prototype-v1')!));const a=stored.models.find((m:{name:string})=>m.name==='写代码的助手'),b=stored.models.find((m:{name:string})=>m.name==='快速问答');expect(a.model).toBe('gpt-6-astra');expect(b.model).toBe('gpt-5.6-sol');expect(a.connection_id).toBe(b.connection_id);
});
test('presets visible immediately and generated per-model names increment on repeated import',async({page})=>{
 await openModels(page);
 await expect(page.getByText('GPT-6 Astra',{exact:true})).toBeVisible(); await expect(page.getByText('本地预置',{exact:false})).toBeVisible();
 await page.getByRole('checkbox',{name:/GPT-6 Astra/}).check();
 await expect(page.getByRole('textbox', { name: '模型名称 gpt-6-astra', exact: true })).toHaveValue('openai-gpt-6-astra-01');
 await page.getByRole('checkbox',{name:/GPT-5.6 Sol/}).check();
 await expect(page.getByRole('textbox', { name: '模型名称 gpt-5.6-sol', exact: true })).toHaveValue('openai-gpt-5.6-sol-01');
 await page.getByRole('button',{name:'保存模型',exact:true}).click();
 await expect(page.locator('.model-row').filter({hasText:'openai-gpt-6-astra-01'})).toHaveCount(1);
 await page.getByRole('button',{name:'添加模型',exact:true}).click(); await page.getByRole('checkbox',{name:/GPT-6 Astra/}).check();
 await expect(page.getByRole('textbox', { name: '模型名称 gpt-6-astra', exact: true })).toHaveValue('openai-gpt-6-astra-02'); await page.getByRole('button',{name:'保存模型',exact:true}).click();await page.reload();
 await expect(page.locator('.model-row').filter({hasText:'openai-gpt-6-astra-02'})).toHaveCount(1);
});
test('account mode has presets, marks discovery as simulated, and provider changes clear selection',async({page})=>{
 const external:string[]=[];page.on('request',r=>{if(r.url().startsWith('http')&&!r.url().startsWith('http://127.0.0.1:4173'))external.push(r.url());});
 await openModels(page);await page.getByRole('radio',{name:'账号登录',exact:true}).check();
 await expect(page.getByRole('checkbox',{name:/GPT-6 Astra/})).toBeVisible();await expect(page.getByText('本地预置',{exact:false})).toBeVisible();
 await page.getByRole('button',{name:'模拟 Codex 账号登录'}).click();await expect(page.getByText('模拟获取结果',{exact:false})).toBeVisible();
 await page.getByRole('combobox',{name:'服务商',exact:true}).click();await page.getByRole('option',{name:'Anthropic（Claude）'}).click();
 await expect(page.getByText('Claude Sonnet 5',{exact:true})).toBeVisible();await expect(page.locator('.catalog-list input:checked')).toHaveCount(4);
 await page.getByRole('radio',{name:'账号登录',exact:true}).check();await page.getByRole('button',{name:'模拟 Claude Code 账号登录'}).click();await expect(page.getByText('模拟获取结果',{exact:false})).toBeVisible();
 await page.setViewportSize({width:320,height:844});await expect.poll(()=>page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1)).toBe(true);
 expect(external).toEqual([]);
});
test('failed discovery keeps presets and manual model entry usable',async({page})=>{
 page.on('dialog',d=>d.accept());await page.goto('/#/settings');await page.getByRole('combobox',{name:'演示场景'}).click();await page.getByRole('option',{name:'任务 / 模型获取失败'}).click();
 await openModels(page);await page.getByRole('button',{name:'模拟验证并获取型号'}).click();await expect(page.getByRole('alert')).toContainText('保留本地预置');
 await expect(page.getByRole('checkbox',{name:/GPT-6 Astra/})).toBeVisible();await page.getByLabel('手动补充型号',{exact:true}).fill('my-private-model');
 await expect(page.getByRole('textbox', { name: '模型名称 my-private-model', exact: true })).toHaveValue('openai-my-private-model-01');await page.getByRole('button',{name:'保存模型',exact:true}).click();
 await expect(page.locator('.model-row').filter({hasText:'openai-my-private-model-01'})).toBeVisible();
});
