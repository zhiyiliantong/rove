# Runtime observation recovery — 2026-10-08 / r27

## Scope

用户明确暂停旧的跨设备重连/Mihomo 调查。本轮只修复共享生命周期将暂时运行时观察失败永久写为 disabled 的行为，不修改物理出口、代理、路由或系统防火墙策略。

实例列表/状态读取失败与调和超时：撤下未确认的业务入口、清空有效地址，保留 enabled 和底层实例，以 starting + 可复制错误表示自动恢复，重查延迟为 1/2/4/8/16/30 秒（封顶）。通过全部既有原生接口/路由/隔离校验后才恢复 running；显式停止取消恢复等待。权限/隔离等安全失败及明确实例错误保留原安全处理；不自动弹出系统授权、不重放作业、不迁移旧 enabled=false 状态。

## Shared verification

- `cargo test --locked -p rove-agent --features easytier-native network_runtime::tests -- --nocapture`：8/8。
- 其中故障注入使用真实拒绝连接的管理端口：连续两次失败仍 enabled=true，处于 starting、地址清空；退避期间不再次尝试；显式 stop 清除退避并写入 enabled=false/stopping。
- 覆盖退避上限、原生安全错误不误归类、agent 初始化保留恢复意图、历史 disabled 失败不自动重启。
- 编译后的完整 agent lib 测试：85/85，309.08 秒。
- `npm test`：最终前端 31/31 通过；前端生产构建通过，已有 chunk-size 警告。
- `.venv/bin/python scripts/check-contracts.py`：57 operations 通过；OpenSpec strict、diff whitespace 通过。
- 默认 Python 缺少 openapi_spec_validator，改用项目既有 `.venv` 校验，没有修改系统 Python。
- 界面回归发现通用 overlay_unavailable 中文翻译隐藏了自动重试含义，因此增加专用恢复提示翻译，原始诊断仍保留在复制内容中。不改变布局。
- 新增 browser/network-recovery.spec.mjs：4/4，1100×760、320×640、390×844、844×390。实际检查四张截图：提示换行无横向溢出、复制图标及停止按钮可达；交互验证复制保留原始诊断、停止请求只发一次且目标为本机当前网络。这是浏览器桥接桩测试，不是 Android 原生恢复或剪贴板实机验收。
- r25 是修正翻译前的中间 Windows 包（SHA256 `271da67084722a3a47e2592babc91d9ec339a403a91b831cac9242136330d888`），只传到 240，未安装。
- r26 Windows 包曾安装，但用户报告 `asset not found: index.html`，检查确认二进制未嵌入当前前端资源。此前只检查进程和 agent 启停，未检查实际窗口，不能作为 GUI 启动通过的证据。疑似与 Windows 编译期间 Android beforeBuildCommand 清空/重建共享 dist 的竞争有关；独立重新编译后资源键恢复，交付改用 r27。
- 构建时检查入口及引用文件，Windows 打包前核对二进制含当前前端资源键；后者可拦住检查后发生竞争导致的空包或旧包。前端构建与原生资源嵌入须串行，不能把 Windows 原生编译与 Android 前置前端构建并行。`scripts/test_frontend_assets.py` 覆盖缺入口、缺二进制、空资源、旧资源、缺样式、完整资源场景。
- 增加 `tests/verify-installed-ui.mjs`：通过实际安装窗口验证页面组件、当前 JS 返回 200、原生 get_device，并保存截图；不使用浏览器桥接桩。

## Native boundaries

Windows 240 原身份 `8f534803-8bb3-4940-a7e2-c35c50709632`，安装前无活动作业，12 个会话、3 个网络、2 个模型、1 个连接及本地别名 windows 已记录；原测试网络 10.203.240.3 运行中。

Android 原授权目标为 242 上的 Redroid。当前无法接入该目标；240 连接的实体手机不是原测试设备，未擅自安装或修改。本轮 Android 安装、VPN 原生恢复、双向通信均未验收。Windows 正常启停也不能代替“注入原生观察故障后自动恢复”的实机证据。

最终 r27 制品哈希、Windows 安装与保留结果记录在 docs/windows-installer-delivery.md。240 实际页面资源及原生 get_device 均 200，正常启停通过并恢复 running；原身份、12 会话、3 网络配置保持一致。临时调试入口已关闭。Android 构建和签名校验通过，但仍受上述实机环境阻碍。
