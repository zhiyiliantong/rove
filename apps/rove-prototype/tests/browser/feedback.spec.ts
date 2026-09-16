import { test, expect } from '@playwright/test';
test.beforeEach(async ({ page }) => { await page.goto('/'); });
test('network form hides random hex secret, supports DHCP, and independently probes editable peers', async ({ page }) => {
  const external: string[] = []; page.on('request', req => { if (req.url().startsWith('http') && !req.url().startsWith('http://127.0.0.1:4173')) external.push(req.url()); });
  await page.goto('/#/networks'); await page.getByRole('button', {name:'添加网络',exact:true}).click();
  await expect(page.getByRole('combobox',{name:'虚拟网段获取方式'})).toContainText('DHCP');
  const key=page.locator('#network-key'); await expect(key).toHaveAttribute('type','password');
  const first=await key.inputValue(); expect(first).toMatch(/^[a-f0-9]{64}$/);
  await page.getByRole('button',{name:'显示网络密钥',exact:true}).click(); await expect(key).toHaveAttribute('type','text');
  await page.getByRole('button',{name:'重新生成网络密钥'}).click(); expect(await key.inputValue()).not.toBe(first);
  await page.getByRole('button',{name:'添加初始节点'}).click(); await page.getByLabel('节点 2',{exact:true}).fill('udp://timeout.example.invalid:11010');
  await page.getByRole('button',{name:'测试节点 1',exact:true}).click(); await page.getByRole('button',{name:'测试节点 2',exact:true}).click();
  await expect(page.locator('.peer-entry').nth(0)).toContainText('模拟可达'); await expect(page.locator('.peer-entry').nth(1)).toContainText('模拟超时');
  await page.getByLabel('节点 1',{exact:true}).fill('tcp://new.example.invalid:11010'); await expect(page.locator('.peer-entry').nth(0).getByRole('status')).toHaveCount(0);
  await page.setViewportSize({width:320,height:844}); expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1)).toBe(true);
  await page.screenshot({path:'test-results/network-form-320.png',fullPage:true});
  await page.getByLabel('网络名称',{exact:true}).fill('DHCP 空间'); await page.getByRole('button',{name:'保存网络'}).click();
  await expect(page.getByRole('dialog',{name:'网络名片'})).toBeVisible(); await page.keyboard.press('Escape');
  await expect(page.locator('main')).toContainText('10.126.126.0/24');
  await page.goto('/#/networks'); await expect(page.locator('.network-list')).toContainText('我的漫游空间');
  expect(external).toEqual([]);
});
test('manual overlapping join keeps existing networks and reports normalized ranges', async ({ page }) => {
  await page.goto('/#/networks'); await page.getByRole('button',{name:'连接',exact:true}).click();
  await expect(page.locator('.network-context')).toHaveText('2 个网络已连接');
  await page.getByRole('button',{name:'添加网络',exact:true}).click();
  await page.getByRole('combobox',{name:'虚拟网段获取方式'}).click(); await page.getByRole('option',{name:'手动指定网段'}).click();
  await page.getByLabel('网段地址',{exact:true}).fill('10.42.100/16'); await expect(page.getByRole('dialog')).toContainText('规范化网段：10.42.0.0/16');
  await page.getByRole('button',{name:'保存网络'}).click(); await page.keyboard.press('Escape');
  await expect(page.locator('main')).toContainText('重叠'); await expect(page.locator('.network-context')).toHaveText('2 个网络已连接');
  await page.reload(); await expect(page.locator('main')).toContainText('网段冲突，未连接');
});
test('model menu opens actual selection; desktop only offers file attachments', async ({ page }) => {
  await page.goto('/#/sessions/welcome'); await expect(page.getByRole('combobox',{name:'执行设备'})).toHaveCount(0);
  await page.getByRole('button',{name:'添加附件或选择模型'}).click();
  await expect(page.getByRole('button',{name:'文件',exact:true})).toBeVisible(); await expect(page.getByRole('button',{name:'拍照',exact:true})).toHaveCount(0);
  await page.locator('.attachment-menu').getByRole('button',{name:'模型',exact:true}).click();
  await expect(page.getByRole('listbox')).toBeVisible(); await page.getByRole('option',{name:'custom-demo-fast-01',exact:true}).click();
  await expect(page.getByRole('combobox',{name:'会话模型'})).toContainText('demo-fast');
});
test('vendors set official editable endpoints; authentication modes are mutually exclusive', async ({ page }) => {
  await page.getByRole('button',{name:'全局添加'}).click(); await page.getByRole('menuitem',{name:'添加模型'}).click();
  await expect(page.getByLabel('接口地址',{exact:true})).toHaveValue('https://api.openai.com/v1');
  await page.getByRole('radio',{name:'账号登录',exact:true}).check();
  await expect(page.locator('#api-key')).toHaveCount(0); await page.getByRole('button',{name:'模拟 Codex 账号登录'}).click();
  await expect(page.getByText('演示认证完成 · 未验证真实模型可用性')).toBeVisible();
  await page.getByRole('combobox',{name:'服务商',exact:true}).click(); await page.getByRole('option',{name:'DeepSeek',exact:true}).click();
  await expect(page.getByLabel('接口地址',{exact:true})).toHaveValue('https://api.deepseek.com');
  await expect(page.getByRole('radio',{name:'API 密钥',exact:true})).toBeChecked(); await expect(page.getByRole('radio',{name:'账号登录',exact:true})).toBeDisabled();
  await expect(page.getByText('演示认证完成 · 未验证真实模型可用性')).toHaveCount(0);
  await page.getByLabel('接口地址',{exact:true}).fill('https://custom.example.invalid/v1');
  await page.getByRole('button',{name:'模拟验证并获取型号'}).click(); await page.getByRole('button',{name:'保存模型'}).click();
  const connections=await page.evaluate(()=>JSON.parse(localStorage.getItem('rove-prototype-v1')!).connections);
  expect(connections.at(-1)).toMatchObject({provider:'deepseek',base_url:'https://custom.example.invalid/v1',auth_kind:'api_key',credential_ref:'demo-only'});
});
test('mobile appearance keeps navigation at bottom in landscape and exposes scanner', async ({ page }) => {
  await page.setViewportSize({width:1024,height:768});
  await page.getByRole('combobox',{name:'外观预览'}).click(); await page.getByRole('option',{name:'移动外观',exact:true}).click();
  for (const viewport of [{width:390,height:844},{width:844,height:390},{width:1024,height:768}]) {
    await page.setViewportSize(viewport); await expect(page.locator('.sidebar')).toBeHidden();
    const nav=page.getByRole('navigation',{name:'移动主导航'}); await expect(nav).toBeVisible(); const box=await nav.boundingBox(); expect(Math.abs(box!.y+box!.height-viewport.height)).toBeLessThan(2);
  }
  await page.getByRole('button',{name:'全局添加'}).click(); await expect(page.getByRole('menuitem',{name:'扫一扫',exact:true})).toHaveCount(0); await page.getByRole('menuitem',{name:'加入网络',exact:true}).click();
  await expect(page.getByRole('button',{name:'扫一扫（演示）'})).toBeVisible(); await page.keyboard.press('Escape');
  await page.goto('/#/sessions/welcome'); await page.getByRole('button',{name:'添加附件或选择模型'}).click();
  // Navigation reload defaults to browser appearance; set mobile again without reloading.
  await page.getByRole('combobox',{name:'外观预览'}).click(); await page.getByRole('option',{name:'移动外观',exact:true}).click();
  await expect(page.getByRole('button',{name:'拍照',exact:true})).toBeVisible(); await expect(page.getByRole('button',{name:'照片 / 文件',exact:true})).toBeVisible();
});
