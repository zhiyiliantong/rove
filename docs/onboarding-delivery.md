# 四步导航正式应用迁移

2026-09-18，用户审核通过 `prototype-onboarding-flow` 后授权迁移，任务归入 `bootstrap-rove` 第 15 组。不是将浏览器原型打包成 APK；正式 Vue 页面仍通过 Tauri / SDK 调用嵌入 agent，未引入模拟数据或模拟登录。

## 交互

- 欢迎 → 添加模型 → 连接设备 → 开始对话，简体中文/英文。独立保存 UI 导航进度，不清业务数据。追加验收要求：仅当前设备至少一个已保存型号测试通过后允许跳过/完成；已有配置不等于可用，重播同样检查。
- 模型步骤复用真实批量配置；第四页建议在没有模型时先配置，成功后创建本机会话草稿，由用户确认发送。旧 `network_onboarding` 会话仍按既有协议创建，不抢占引导页面。会员登录仍明确不可用。
- 连接步骤使用可暂停、尊重减少动态效果偏好的教学动画，不将动画视为实际联网。创建/导入复用 NetworkPanel；Android native VPN 和后台能力限制仍在该步骤、网络页及设置提示。
- 会话列表删除重复标题和本机代理说明；新会话、归档管理集中到顶栏加号。移动端先显示列表，选择后进入详情；详情保留真实执行设备、作业状态、归档/恢复和删除能力。
- 设置的“重置导航页”只写 `rove-gui-introduction-v1.reset_pending`，下次启动生效；不会删除模型、网络、会话或中断作业。没有迁移原型专用的“清空演示数据”按钮。
- 未发送草稿按会话 ID 保存在本机 WebView `rove-gui-drafts-v1`，发送成功/删除会话移除相应草稿；本机存储失败在输入区提示复制备份。模型 API 密钥仍只交给 agent，不存此 UI key。

初次迁移没有新增业务 API；用户反馈后新增 OpenAPI `test_model`（`POST /v1/model-catalog/test`）及 `rove model test --body -`，SDK socket/overlay HTTP 均按契约调用。请求二选一：`model_id`，或 `provider/base_url/api_key/model`；默认目录不回显密钥。导航进度和未发送草稿是本机 UI 状态，无 SQL schema 迁移。

## 追加验收调整

- 新增/修改连接的密钥输入提供眼睛按钮，默认隐藏，关闭后重新隐藏；不新增读取已保存明文密钥的接口。已有连接可直接测试，无需把密钥取回界面。
- DeepSeek 测试请求显式关闭思考模式，避免默认思考耗尽短测试预算而误判不可用；不修改实际会话的推理偏好。依据 [DeepSeek 思考模式文档](https://api-docs.deepseek.com/guides/thinking_mode/)；接口路径依据 [Anthropic 兼容接口](https://api-docs.deepseek.com/guides/anthropic_api/)。
- 添加模型提供测试型号选择与真实推理测试，有小额计费提示；不自动测试全部型号。25 秒超时（小于 SDK 30 秒请求期限），最多 256 输出 tokens，收到非空文本即完成；不调用工具、不创建会话。这里只验证当前配置当时能应答，不保证未来额度/网络或所有型号能力。
- 私有 metadata 仅保存最多 1000 条配置 SHA-256 指纹/测试时间/结果，不保存未提交密钥。地址、提供方、密钥或型号改变则先前测试不匹配；后来失败覆盖成功。测试结果不随 API 配置同步到其他设备，目标需独立测试。
- 型号列表获取不是推理测试。引导内保存前至少测试一个所选型号；最终离开引导再次读取目录，并确保默认型号为已测试型号。已完成引导的用户不会被每次启动强制重测。
- DeepSeek 可选择 OpenAI 根地址（也兼容 `/v1`）或 Anthropic `/anthropic`，对应 Rig 原生适配器；手填 `/anthropic/v1` 也归一化，名称仍是 DeepSeek。提供方不支持枚举时仍可选择预置/手动型号并实际测试。
- 欢迎页适配 360×568；长表单使用纵向滚动，显式滚动条样式提示可继续向下；错误放在模型测试入口旁，不需要滚到表单底部寻找。
- 未将聊天中暴露的真实密钥用于请求、源码、日志或交付文档；用户需要先撤销/轮换该密钥，再自行进行真实提供方验收。

## 验证与边界

- 正式 GUI 24 项单元、42 项全量浏览器回归通过；最后滚动条/错误位置调整另有 10 项相关回归通过，包含 360×568 首屏开始按钮可见、320–1440 宽度、横屏、英文、减少动态效果、重播、草稿保留、不自动提交、密钥显隐和测试门槛。
- Rust：43 项 agent 单元分批通过（新增 API 后更新全操作覆盖计数），7 项目录/提供方/测试集成通过，7 项协议测试通过；CLI 单元 1 项、集成 6 项通过，首轮并行下 attach 项失败，单项重跑通过，未放宽断言。最终 Clippy `-D warnings`、52 操作 OpenAPI/对等矩阵与 OpenSpec strict 校验通过。
- 原型追加清空入口独立通过 50 项单元及 12 项导航/清空浏览器测试；用户审核已记录，4173 继续只作独立模拟原型。
- Android 使用同一 Debug 签名覆盖升级，不卸载或清数据。首次迁移保留 5 条会话/0 模型；用户后续添加数据后，模型测试版以新基线验证身份、10 条会话、2 个型号和中文偏好保留，不将用户追加数据回滚为旧基线。
- 原生回归脚本 `scripts/check-android-introduction.mjs` 仅连接明确转发到本机 19222 的 Rove Debug WebView。读取 agent 数据核对、操作四步导航、检查跳过禁用/密钥显隐/长表单滚动；不调用配置的提供方，不添加/删除真实模型或网络。
- 240 原有 scrcpy 窗口 `Rove-Android-242` 已恢复并激活，沿用原 SSH/ADB 隧道；不新增公网调试监听。

本轮不重打 Ubuntu 包，不代表官方账号登录、Android native VPN/后台常驻或 Android → Windows 控制已经完成。4174 上较早的 Ubuntu/Android 核心包不是本次导航 APK；本次制品与证据保存在 `/mnt/data/rove-deliveries/2026-09-18-onboarding/`。

最终验收 APK：`rove-android-arm64-review-debug.apk`，SHA-256 `90c34d9877b685e18eaa1e45f31ae874b6c1d5372d4c7be0835de6c11d3b8cab`，Android arm64 / 同一 v2 Debug 签名。对应前端 `index-wkEEDNuF.js`、`index-DPmhZQeQ.css`。较早的 `rove-android-arm64-debug.apk` / `rove-android-arm64-model-test-debug.apk` 保留作历史对照，不是最终包。

最终原生证据：`android-final-guide-review/result.json` 与截图，确认引导弹窗的 `save_requires_test=true`、`skip_disabled=true`、表单纵向可滚动且无横向溢出、原业务数据保留。校验脚本将模型新增按钮限定在当前引导对话框内，避免点击仍挂载但隐藏的主应用模型面板。`android-final-runtime.json` 核对加载资源版本及未知型号的 404（不访问供应商）。240 临时聚焦任务执行成功后已移除，保留原查看器及其隧道。
