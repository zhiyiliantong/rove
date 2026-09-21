# 用户消息与失败信息复制

2026-09-21，依据 `.images/会话改进.png` 的两处标注，纳入 bootstrap-rove 第 19 组。

用户消息气泡下方增加常显复制图标，复制原文并保留换行、空格。失败说明下方增加复制图标，复制当前显示的错误代码及本地化说明，即使没有 AI 输出也能使用。手机触控区域 44px，支持中文/英文反馈；剪贴板不可用时显示可选择的只读文本。复制操作不发起或重试 AI 作业。

后续反馈：复制成功只保留图标，不显示“已复制”文字；用户消息、失败信息及 AI 回复/分段复制保持一致。下文“已复制”证据描述的是此次调整前的验收包。

仅图标版本已覆盖安装 242：`rove-android-arm64-icon-only-debug.apk`，SHA-256 `3889606271054ad49a6bbb616d19013b3bd6b2bac0b96829e559aa3083d227e2`，签名校验、类型/构建及 2 项复制回归通过。`before-icon-only.json` / `after-icon-only.json` 完全一致。原生点击后只显示复制图标，见 `native-icon-only.json`；资源 `index-Cw-wAqo2.js`。

验证：9 项浏览器测试通过，覆盖复制成功/拒绝、消息原文、无输出失败、手机无横向溢出，以及现有固定顶栏/输入框和滚动行为；5 项国际化测试通过，类型检查和生产构建通过。接口与后台没有变化，沿用现有 OpenAPI 消息及错误字段。

制品目录 `/mnt/data/rove-deliveries/2026-09-21-copy/`，浏览器结果 `/mnt/data/rove-deliveries/2026-09-21-copy-tests/`。最终 APK 为 `rove-android-arm64-final-debug.apk`，签名校验通过，SHA-256：`e5555c80dfe62f9456aba5a0f2b14c893a91b740362f9c374c18303171996ace`。

242 已覆盖升级并打开最新失败会话。`before-upgrade.json` 与 `after-final-upgrade.json` 完全一致，12 条会话、4 个型号、草稿、身份及引导偏好保留；升级前无活动作业。未调用真实模型、修改会话或删除数据。

原生检查发现 PrimeVue 图标按钮默认高度覆盖了 44px，最终以 min-height 保证 44×44px，证据 `native-final-buttons.json`，加载 `index-bX3OgBM4.js`，无横向溢出。未聚焦时通过脚本点击会触发剪贴板拒绝，手动复制降级正常；最终实际 ADB 触屏点击用户消息与失败信息均显示“已复制”，见 `native-final-user-copy.json`、`native-final-touch-error-copy.json`。截图 `android-copy-final.png`。早期非 final APK 和未聚焦结果保留为过程记录。

第 19 组完成后进度 84/93，原先剩余 9 项平台/授权任务保留。OpenSpec strict 和 diff 检查通过。
