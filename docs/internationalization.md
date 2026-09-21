# 正式 GUI 国际化

2026-09-17：正式应用 `apps/rove-gui` 接入 Vue I18n，支持简体中文与英文。入口为「设置 → 界面语言 / Settings → Interface language」，可以选择跟随系统、简体中文或 English。

## 语言与数据边界

- 首次默认跟随系统，按系统语言偏好选择已支持的中文或英文；没有匹配语言时回退英文。手动选择优先，保存在本机 WebView 的 `rove-gui-locale` 中，不通过 agent 同步到其他设备。存储不可用时仍能在当前窗口切换。
- 切换立即更新导航、页面、弹窗、表单校验、常见错误、组件无障碍提示、复制与朗读按钮，以及日期和数字格式。PrimeVue 使用相同语言设置；页面 `lang` 与标题也同步更新。
- 不翻译或修改聊天历史、正在输出的内容、用户草稿、模型/设备/网络名称、接口地址及标识符。语言切换不重建会话、不取消或重新提交作业。新填入的建议草稿采用当前语言，已经输入的草稿保留原文。
- 协议字段、枚举与错误代码保持稳定。已知错误代码对应本地化说明；未知诊断信息保留原文，便于排查。语言设置不包含任何账号或模型密钥。
- 朗读优先使用当前界面语言的本地语音；不启用云端语音兜底，不改写被朗读的原文。切换语言不打断已启动的朗读。

本次范围为正式 GUI；CLI 的 JSON/API 输出、agent 日志及用户内容不自动翻译。独立原型 `http://10.1.2.237:4173/` 未被替换。此前 Ubuntu/Android 安装包不包含本次改动，需后续重新打包和实机验收。

## 维护入口

- `apps/rove-gui/src/i18n.ts`：语言解析、持久偏好、系统/跨窗口变化与格式化。
- `apps/rove-gui/src/locales/zh-CN.ts`、`en.ts`：两份对应的界面词条；插值参数必须一致。
- `apps/rove-gui/src/primevue-locales.ts`：组件库文案与无障碍标签。
- 页面使用 `t()`，不要在组件中增加硬编码界面文案。不要将用户内容交给 `notice_text()`；该函数只处理受控通知与错误。

使用 Vue I18n 的 Composition API，参考 [官方安装说明](https://vue-i18n.intlify.dev/guide/installation) 与 [Composition API 文档](https://vue-i18n.intlify.dev/guide/advanced/composition)。依赖版本固定在 GUI 的 package-lock.json。

## 验证

```sh
cd apps/rove-gui
npm test
npm run build
PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH=/path/to/chromium npm run test:e2e -- --workers=1
```

单元测试检查词条/插值一致、全部消息编译、系统语言回退、日期/状态/错误、复制原文与安全转义，并阻止 Vue 页面新增硬编码中文（语言自称除外）。浏览器检查中英文实时切换、重载保存、流式订阅与草稿保留、320/1440px 英文页面布局、组件关闭提示，以及禁用动态代码求值时的页面渲染。

浏览器测试使用隔离的 Tauri IPC fixture，不代表 Android WebView、原生系统语言事件或真实语音已实机验收。禁用 `eval`/`Function` 的检查也不替代完整原生 CSP 验证。
