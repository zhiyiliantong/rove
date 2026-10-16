## Why

当前正式 GUI 创建网络时要求用户先填写名称和初始节点，公共节点也只能手工查找、复制和逐条测试；创建成功后名片虽已在组件内生成，却没有稳定地进入网络名片视图。该流程阻碍首次组网，也容易把 EasyTier 的节点目录、发现地址和最终连接地址混为一谈。

## What Changes

- 补齐 Windows 与 Android 的实际 EasyTier 运行时，复用同一配置转换、生命周期、状态观察及设备发现模块；系统权限、虚拟网卡与 Android VPN 通道通过薄适配层接入。两端真实启停和 overlay 协议通信单独验收，不以安装或探测替代。

- 创建网络表单打开时自动生成可编辑的 `net-xx` 风格名称，并继续由后端生成稳定网络 ID、EasyTier 网络身份和随机密钥。
- 在设置中增加公共节点发现服务地址；Rove 后续发行版可预置一个 HTTPS 地址，用户仍可覆盖或清除。创建页通过所选目标 agent 请求该服务，目录不可用时保留手工输入且不得阻止创建。
- 在 Rove 同级目录建立独立 `rove-discovery` 服务项目。服务首先从 `https://ruixuan.online/uptime/api/status-page/easytier` 和对应 heartbeat API 读取公开社区节点，过滤打码/无效项并提供结构化节点目录和 EasyTier HTTP 发现 API；它不属于 Rove 二进制或信任边界。
- 完整支持 EasyTier 的 `http://`、`https://`、`txt://`、`srv://` 四种发现地址，以及 `tcp://`、`udp://`、`ws://`、`wss://`、`wg://`、`quic://`、`faketcp://` 七种最终连接地址。配置保留原始发现 URI，以便 EasyTier 在重连时重新解析；预览结果只用于展示和诊断。
- 创建网络成功后直接显示新网络的网络名片，且在引导弹窗和普通网络页保持一致；失败时保留表单，不跳转。
- 网络标签遵循已确认原型的信息架构：默认只显示网络列表，创建和加入从右上角全局「＋」打开弹窗，名片和编辑也使用互斥弹窗；创建成功转入名片，已有网络从列表显式查看名片。
- GUI 网络名片改为自包含的 Base64 JoinConfig 文本和同内容二维码，名片只保留复制配置分享操作；GUI 加入流程只在本地解码后调用受校验的手动导入，不再生成、展示或接受配置托管 URL。
- 扩展 agent/OpenAPI/SDK/CLI 所需的公共节点获取与发现解析契约，使远端目标上的 GUI 操作仍由目标 agent 发起，避免浏览器跨域、DNS 能力和平台差异。

## Capabilities

### New Capabilities

- `bootstrap-node-discovery`: 定义公共初始节点目录获取、HTTP(S)/TXT/SRV 发现解析、候选校验、去重、错误与信任边界。

### Modified Capabilities

- `overlay-network`: 创建网络使用自动生成的 `net-xx` 风格名称，并把目录候选或 EasyTier 原生发现 URI 作为可移植的初始节点保存。
- `user-interfaces`: 正式 GUI/CLI 提供公共初始节点获取入口，GUI 创建成功后进入对应网络名片且失败保持原表单。
- `network-sharing`: GUI 网络名片与导入从托管 URL 改为自包含 Base64 配置和同内容二维码。

## Impact

- `rove-agent` 网络 API、节点探测/发现模块、OpenAPI 与 SDK/CLI 客户端。
- `rove-gui` 的 `NetworkForm`、`NetworkPanel`、双语资源和创建/引导导航状态。
- 发现输入校验需覆盖 EasyTier 四种发现协议和七种最终协议；目录、HTTP、DNS TXT/SRV 都是不可信外部输入，需要超时、响应大小、跳数、条数和地址校验。
- Rove 不再依赖已失效的 `uptime.easytier.cn`。独立发现服务依赖 ruixuan 的公开 Uptime Kuma API，但该社区来源没有稳定契约，必须可替换、缓存、明确标注来源并在故障时降级。
