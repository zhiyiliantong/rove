# agent-communication Specification

## Purpose

定义 Rove 客户端与本机及远端 agent 的一致通信契约，使 GUI、CLI 和设备间调用获得相同结果，并在协议版本不同、网络断开或请求重试时提供明确且可恢复的行为。

## Requirements

### Requirement: Machine-readable interface contracts
Agent 与密文配置服务的 HTTP API MUST 通过 OpenAPI 3.1 定义 operationId、参数、请求、响应和错误 schema。本机 socket MUST 复用 agent operationId 和业务 schema，并提供明确的帧与订阅映射；契约 MUST 可通过引用及 schema 校验。

#### Scenario: Build equivalent clients
- **WHEN** 开发者按 OpenAPI 实现 HTTP 与本机 socket 客户端
- **THEN** 相同业务操作的输入和结果遵守同一 schema，socket 封装只增加传输关联、目标与订阅信息

#### Scenario: Invalid contract change
- **WHEN** 接口文档引入无法解析的引用、重复 operationId 或与 schema 不符的示例
- **THEN** 契约验证失败，不能把该契约标记为实现就绪

### Requirement: Equivalent local and remote client API
Rove SDK MUST 提供本机 socket 连接及经 overlay 的远端连接能力，并共享请求、响应和错误语义。GUI 与 CLI MUST 经本机 agent 访问目标设备，本机 agent MUST 使用 SDK 完成远端客户端调用。

#### Scenario: Forward to remote execution device
- **WHEN** 用户在设备 A 选择设备 B 并发送消息
- **THEN** A 的 agent 保存本机会话及提交关联，通过 SDK 把请求送到 B；B 保存实际执行事实，A 展示 B 返回的结果；既有显式 target 调用保持兼容

### Requirement: Remote interface reachable only through overlay
远端执行接口 MUST 只接受来自已加入 overlay 的访问，本机客户端 MUST 使用本机 socket；物理局域网和公网 MUST NOT 因远端接口启用而获得未经入网的执行入口。

#### Scenario: Network interface changes
- **WHEN** overlay 地址改变或网络停止
- **THEN** 对应监听地址更新或停止，系统不会退回到开放所有物理接口的执行监听

### Requirement: Explicit protocol compatibility
握手 MUST 返回设备身份、协议支持范围和可用能力；客户端 MUST 在协议不兼容时报告错误并停止提交操作，不要求网络中所有设备同时升级。

#### Scenario: Unsupported peer protocol
- **WHEN** 两端没有共同支持的协议版本
- **THEN** SDK 返回不兼容结果，界面显示升级提示，目标不启动作业

### Requirement: Idempotent run submission
提交作业 MUST 携带 `request_id`，执行设备 MUST 将它与请求内容及 `run_id` 持久关联；相同请求重试 MUST 返回同一作业。相同 ID 携带不同内容 MUST 被拒绝。

#### Scenario: Response lost after acceptance
- **WHEN** 作业已被目标接受但响应丢失，客户端以同一 `request_id` 重试
- **THEN** 目标返回原 `run_id`，不会启动第二份作业

#### Scenario: Reused ID with changed payload
- **WHEN** 已有 `request_id` 被用于不同会话或不同消息
- **THEN** 目标返回冲突，不覆盖已有记录

### Requirement: Independent status and event access
已接受作业 MUST 能通过 `run_id` 查询状态和输出，事件 MUST 有可排序序号。断开事件连接 MUST NOT 隐式取消作业；SDK 重连 MUST 能恢复已有输出或通过明确的快照恢复路径继续展示。

#### Scenario: Reconnect from another client
- **WHEN** 用户关闭电脑界面后，从另一设备连接到仍在线的执行设备并打开原作业
- **THEN** 用户可查询当前状态、读取已有输出并继续接收后续事件，不重复提交作业

#### Scenario: Event history no longer retained
- **WHEN** 客户端使用已超出保留历史的事件游标重连
- **THEN** 接口明确返回历史过期，客户端以快照及其序号水位恢复，不把缺失输出当作从未发生

#### Scenario: Address reused by another device
- **WHEN** SDK 连接到的设备 ID 与预期目标不一致
- **THEN** 接口返回 target_mismatch，接收设备不执行该请求
