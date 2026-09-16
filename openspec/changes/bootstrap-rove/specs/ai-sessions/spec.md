## Purpose

定义每个对等设备的 AI 配置、会话归属和并发执行行为，让多个用户或窗口能同时推进不同任务，并在断线、取消和进程重启时理解实际执行状态而不丢失对话历史。

## ADDED Requirements

### Requirement: Per-device default model configuration
每台 agent MUST 独立保存默认提供方、接口地址、模型及所需凭据，支持本机与可信远端设置。网络加入 MUST NOT 自动复制模型配置；未配置模型 MUST 返回明确错误且不执行工具。

#### Scenario: Configure remote device
- **WHEN** 用户在 A 上为 B 设置有效模型配置并向 B 提交消息
- **THEN** B 使用自身保存的配置执行 AI，A 的默认模型配置不变

#### Scenario: Missing model configuration
- **WHEN** 向没有模型配置的目标提交 AI 作业
- **THEN** 目标报告需要配置模型，不执行部署命令或静默借用另一设备的凭据

### Requirement: Session belongs to execution device
会话 MUST 由实际运行 AI 的目标设备创建、持久保存并分配 `session_id`；可信客户端 MUST 能列出、查看并继续该设备的会话。目标离线时 MUST 明确报告不可达，不自动迁移执行。

#### Scenario: Continue from another device
- **WHEN** 用户从另一客户端打开目标设备已有的会话
- **THEN** 它读取同一会话历史，并可在该目标上提交后续消息

### Requirement: Sequential sessions and bounded parallel runs
同一会话 MUST 最多有一个运行中作业，后续作业按提交顺序排队；不同会话 MUST 可并发。设备默认 MUST 支持四个运行中作业，并允许 GUI 与 CLI 配置正整数并发上限；等待中的作业 MUST 显示 `queued`，不占运行槽。

#### Scenario: Independent sessions overlap
- **WHEN** 两个不同会话提交作业且并发槽充足
- **THEN** 两个作业可同时推进，一个会话的长命令不阻塞另一个会话开始

#### Scenario: Sequential conversation
- **WHEN** 同一会话的第一个作业尚未结束，又收到第二条消息
- **THEN** 第二个作业排队，并在第一个结束后使用已持久化的最新会话上下文执行

#### Scenario: Future queued input is excluded
- **WHEN** 同一会话中第二个作业的消息正在排队，第一个仍在调用模型
- **THEN** 第二条消息只保存在待执行输入中，不进入第一个作业的模型上下文；轮到第二个作业时再进入历史

#### Scenario: Capacity reached
- **WHEN** 四个独立作业正在默认配置的设备执行，第五个作业提交成功
- **THEN** 第五个返回自身 `run_id` 和 `queued` 状态，在有可用槽时运行

#### Scenario: Lower concurrency while busy
- **WHEN** 用户将并发上限调低到当前运行数以下
- **THEN** 已运行作业继续，新作业等待运行数降到新上限以下

### Requirement: General-purpose tools without fixed deployment workflows
AI MUST 能通过通用本机命令执行和服务发布能力装配软件；单个作业中的工具调用 MUST 顺序执行，不同作业的命令 MUST 允许并发。工具结果 MUST 包含成功或失败及可理解的输出；不可用平台操作 MUST 返回 `unsupported`。

#### Scenario: Concurrent command results
- **WHEN** 两个不同会话的作业执行命令
- **THEN** 系统允许它们重叠执行，各自输出归属于各自作业；实际包管理器锁或端口冲突作为工具错误返回

#### Scenario: Model emits multiple tool calls
- **WHEN** 一个作业的模型响应包含多个工具调用
- **THEN** agent 按确定顺序逐个执行并归档结果，不在该作业内同时启动这些工具

#### Scenario: Command resource limits
- **WHEN** 模型调用 `system_exec`
- **THEN** 命令最多接受 65,536 个字符，超时默认为 300 秒且只接受 1 至 86,400 秒；超限输入在启动进程前被拒绝，stdout/stderr 各保留最多 65,536 个字符的尾部并明确标记截断

### Requirement: Explicit cancellation and interruption
关闭客户端或网络断线 MUST NOT 取消已接受作业。用户 MUST 能按 `run_id` 取消等待或运行中的作业；取消 MUST 停止后续模型与工具步骤并尝试终止该作业受管理的子进程，MUST NOT 声称已回滚完成的系统变更。agent 重启 MUST 将旧的未完成作业标为 `interrupted`，不自动执行旧队列。

#### Scenario: Cancel one of several runs
- **WHEN** 用户取消一个运行中作业
- **THEN** 只有该作业接收取消信号，其他会话继续运行，取消结果反映子进程停止情况

#### Scenario: Cancel queued run
- **WHEN** 用户取消尚未开始的作业
- **THEN** 作业变为 `cancelled`，不调用模型或工具

#### Scenario: Restart with pending work
- **WHEN** agent 重启且数据库中存在运行中或排队中的旧作业
- **THEN** 这些作业成为 `interrupted`，历史保留，用户需重新提交才会开始新的执行

### Requirement: Traceable execution output
作业 MUST 保存实际使用的模型标识、状态、消息、工具结果及有序输出，失败、取消与中断 MUST 同样可查询。凭据字段 MUST 不作为默认状态输出或日志字段展示。

#### Scenario: Tool fails after earlier success
- **WHEN** 一个工具失败但此前已有成功的工具步骤
- **THEN** 作业记录保留先前结果与当前错误，客户端不会把整个过程展示为从未执行
