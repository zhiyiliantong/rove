# 即时对话与 Android HTTPS 修正

2026-09-20，按用户三张截图调整正式 GUI，纳入 `bootstrap-rove` 第 16 组。没有改变独立原型或新增私有业务接口。

## 已实现

- 会话只保留连续消息区，移除单独“消息和工具历史”与重复的执行设备标题栏；标题点击改名，移动端顶部返回列表，归档/恢复/删除移入全局加号。既有 agent 消息、作业和工具 API 保留，数据不删除。
- 标题和输入框保持在视口内，消息和列表独立滚动。流式输出只在接近底部时跟随，上翻阅读时不抢位置；回到最新按钮浮在输入框上方，不挤压横屏布局。
- 空白会话建议默认开启，设置可关闭并持久保存。该设置仅影响界面，不修改远端设备或提交作业。中文英文同步。
- 型号选择和可编辑配置名称合为一份列表。密钥输入后离开表单字段自动获取目录，不自动推理；失败保留参考列表，支持手动补充、显式重试。旧请求不能覆盖新提供方/密钥的配置；刷新保留手改名称。

## DeepSeek 超时原因

242 原 APK 的 Android 日志出现 `rustls-platform-verifier 0.7.0` 的 `Expect rustls-platform-verifier to be initialized` panic。HTTPS 证书校验时请求任务崩溃，SDK 随后显示 30 秒本机响应超时；这不是密钥格式错误的证据。

修复在 MainActivity 启动嵌入 agent 前初始化 reqwest 使用的同版本验证器。Gradle 从 Cargo 元数据定位匹配的 Android AAR，本地 Maven 仓库仅处理 rustls 组件，Proguard 保留 JNI 校验类。不禁用证书校验、不安装额外信任根，不返回保存的明文密钥。

真实 Android 验证：

- 不带凭据查询 OpenRouter 公开 HTTPS 目录成功，返回 447 个型号；最终 APK 重测同样为 200。
- loopback HTTP 目录与 Rig 模拟推理分别返回 200；只有模拟服务收到请求，没有实际供应商计费调用。
- 自签名 HTTPS 夹具被拒绝：代理返回 502，服务端记录 TLS 拒绝一次、HTTPS 业务请求为零。未向 Android 信任库导入测试证书。
- 不带密钥查询 DeepSeek 两种地址均在约 0.9–1.4 秒内返回有界目录错误，没有此前的 30 秒任务崩溃；**不据此判断真实密钥有效性或推理权限**。

## 验证和交付

24 项前端单元、49 项全量浏览器回归通过。覆盖横竖屏流式滚动、标题改名、归档草稿恢复、建议开关持久化、自动目录查询/失败回退/过期响应、密钥显隐和引导门槛。类型检查、生产构建、OpenSpec strict、52 操作 OpenAPI/功能对等校验、Rust 格式与 diff 检查通过。未改 agent 推理实现，本轮不重复声称此前整套 Rust 测试已重跑。

制品目录：`/mnt/data/rove-deliveries/2026-09-20-chat/`。

- 最终 APK：`rove-android-arm64-review-debug.apk`，Android arm64，同一 Debug v2 签名。
- SHA-256：`11820eea4a708b6926a4e2b5fda6acbfb5e0eb17bdb7f5f917fb03db1a638b42`。
- 前端：`index-CONfdh9W.js` / `index-BcT1oYT-.css`。
- `before-upgrade.json` / `after-upgrade.json` 对比通过：设备身份、10 条会话 ID、2 个型号 ID、引导状态和 2 份草稿保留；没有卸载、清数据或回滚用户新增配置。
- `android-tls-results.json` / `final-https.json` / `deepseek-no-credentials.json` 为无真实凭据网络证据。`native-model-form.json` 和截图验证实际 WebView 的合并行、默认全选、名称可编辑、空密钥默认隐藏、引导保存禁用及无横向溢出。
- 242 已覆盖安装并打开已有模型目录；240 原 scrcpy 进程仍在。当前用户的 2 个型号尚未真实测试通过，保留引导门槛，没有为展示会话而绕过引导。用户可点击已有型号的“测试模型”验证真实授权与额度。

本轮不重新打 Ubuntu 包，不宣称 Android → Windows overlay、后台常驻或官方会员授权完成。第 16 组完成后总进度 81/90，原剩余 9 项不变。
