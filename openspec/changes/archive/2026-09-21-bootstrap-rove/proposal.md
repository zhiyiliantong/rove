## Why

个人的手机、平板和电脑分散在不同网络与操作系统上，组网、安装软件和访问自托管服务需要重复配置。Rove 为这些对等设备提供统一入口，让用户通过对话装配设备能力，并从其他设备继续操作和使用服务。

## What Changes

- 建立 Rust workspace，采用 EasyTier 组网、Rig 驱动 `rove-agent`、Tauri + Vue 提供 GUI；`rove` CLI 与 GUI 使用同一 SDK，提供对等功能。
- 提供详细需求、用户故事及架构/模块/进程/数据/部署模型；以 OpenAPI 3.1 定义 agent 与密文服务接口，本机 socket 复用同一 operationId 和数据 schema。
- 每台设备具有相同的逻辑 `rove-agent`，负责本机 AI 执行、状态和服务发布；本地客户端通过 socket API 访问，远程 agent 调用也封装在 `rove-sdk`。
- 支持个人管理、分享并同时加入多个实际网段不重叠的网络；保存配置不等于加入。自动模式由原版 EasyTier 分配，手动模式指定网段和各设备独立 IP，不维护 DHCP 补丁。网络成员被完全信任，可操作设备和使用服务，不新增 Rove 账号、角色或权限审批。
- 为设备和网络保存稳定业务 ID，将 EasyTier 实例、peer 和地址作为适配层信息。
- 支持二维码、加密配置 URL 和手动录入；公网配置服务只保存密文，分享链接默认七天有效、可重复导入。
- AI 通过通用执行和服务发布工具装配软件，不预设音乐服务器、coding agent 等产品的固定部署工作流。
- 支持同一会话串行、不同会话并行，默认每设备四个运行中作业；作业断线继续执行，并支持查询、取消和重连。
- 提供持久 TCP 服务代理和地址、应用认证信息登记，使网络成员可使用浏览器或原生客户端访问服务。
- 首版完成桌面和无界面设备的完整闭环；移动端保留相同核心与 GUI 接入，移动网络原生集成、后台常驻和原生工具扩展后续交付。通用账户数据管理、插件市场和 AI 自动跨设备编排不在首版范围。

## Capabilities

### New Capabilities

- `device-runtime`: 稳定设备身份、agent 进程与状态归属、平台运行方式及发行包组成。
- `overlay-network`: 多网络配置、EasyTier 实例生命周期和设备发现。
- `network-sharing`: 网络加入配置、二维码与 URL 导入、密文托管和手动配置。
- `agent-communication`: OpenAPI 接口契约、本机 socket 映射、远程 SDK 调用、协议兼容和请求去重。
- `ai-sessions`: 设备模型配置、会话、并发作业、流式输出和通用工具执行。
- `service-publishing`: TCP 代理、服务登记、凭据展示和重启恢复。
- `user-interfaces`: Vue GUI 与交互式 CLI 的功能对等和跨设备使用流程。

### Modified Capabilities

无；仓库尚无已实现的应用或既有能力规格。

## Impact

- 新建基础 crates、SDK、agent、CLI、Tauri GUI 和独立密文配置服务；具体依赖方向见 design。
- 引入 Rig、EasyTier 配套程序、Tauri、Vue、Rust 异步运行时、持久化和标准认证加密实现；实施时固定并验证版本，不依赖未经验证的上游内部 API。
- 新增本机 socket 协议、overlay 远程 API、服务监听端口及公网密文上传/下载 API。
- 桌面安装需处理 EasyTier 系统服务和用户级 agent；无界面发行包须包含 CLI，移动平台的后台与 VPN 能力分阶段处理。
- 完全信任网络意味着分享加入配置即分享设备操作能力。分享载荷不包含模型密钥、设备身份、会话或服务凭据；链接到期不撤销已经加入的成员。
