# device-runtime Specification

## Purpose

定义 Rove 在对等设备上的稳定身份、状态归属和运行方式，使用户在升级、切换界面和连接远程设备时获得一致行为，并明确桌面、无界面及移动端发行的能力边界。

## Requirements

### Requirement: Read-only storage inspection through the shared API
agent MUST 提供 `get_storage_usage` 检查自身数据目录的顶层文件元数据，CLI 和对话工具 MUST 复用该接口。检查 MUST NOT 读取凭据内容、跟随链接、递归任意目录或执行清理；统计不完整时 MUST 明示。逻辑字节统计 MUST NOT 被表示为可回收空间。

#### Scenario: Inspect a directory containing backups and links
- **WHEN** 用户通过对话或 CLI 检查 Rove 占用
- **THEN** 返回有界文件元数据与逻辑大小，目录或链接未统计时标记不完整，原文件与活动数据库不被修改

### Requirement: Stable installation identity
Rove MUST 在首次初始化时生成并保存非秘密的 `device_id`，保留数据的重启、升级和网络配置变化 MUST 保持该标识；入网导入 MUST NOT 覆盖它。

#### Scenario: Network configuration changes
- **WHEN** 已初始化设备更换网络配置或网络路由标识
- **THEN** 设备对外报告原有 `device_id`，历史会话和服务仍归属该设备

#### Scenario: Fresh installation
- **WHEN** 设备在没有既有 Rove 数据的情况下初始化
- **THEN** 系统生成新的 `device_id`，不复用分享者的设备身份

### Requirement: Peer runtime and authoritative local state
每台设备的逻辑 agent MUST 负责本机模型配置、会话、作业和服务状态；GUI、CLI 与远程客户端 MUST 通过 agent 修改这些数据。桌面同一用户数据目录 MUST 只允许一个活动 agent 实例。

#### Scenario: Multiple local clients
- **WHEN** GUI 与 CLI 同时连接同一设备
- **THEN** 两者读取同一份权威状态，关闭一个客户端不结束 agent 或另一个客户端的会话

#### Scenario: Duplicate agent startup
- **WHEN** 第二个 agent 尝试使用已被活动 agent 占用的数据目录
- **THEN** 它不成为第二个写入者，并报告已有实例

### Requirement: Operating system execution context
桌面 agent MUST 默认使用当前用户的操作系统身份，无界面运行 MUST 支持指定服务账号；系统 MUST 报告当前账号无法执行的操作，不把网络加入解释为操作系统提权。

#### Scenario: Privileged operation unavailable
- **WHEN** 操作需要当前运行账号没有的系统权限且无法完成系统授权
- **THEN** 作业返回可理解的权限错误，并允许用户通过系统机制处理后重试

### Requirement: Distribution includes usable interaction modes
桌面发行包 MUST 包含 GUI、`rove` CLI、agent 和配套网络程序；无界面发行包 MUST 包含 CLI、agent 和配套网络程序。发行物 MUST 标明系统与架构，且所声明支持的组合 MUST 有安装与启动验证记录。

#### Scenario: Headless installation
- **WHEN** 用户在没有图形环境的受支持设备安装无界面发行包
- **THEN** 用户可通过 `rove` 配置和查看应用，并启动网络和 agent，无需 GUI

### Requirement: Mobile runtime capability reporting
移动端 MUST 复用同一逻辑 agent 和界面协议，平台未提供的操作 MUST 返回 `unsupported`。首版移动壳 MUST 明示尚未交付的网络原生集成和后台能力，不把设备差异表示为非对等角色。

#### Scenario: Unavailable mobile operation
- **WHEN** 请求移动端执行未实现的本机命令或原生操作
- **THEN** 请求收到 `unsupported` 结果，界面仍可继续使用其他已提供功能
