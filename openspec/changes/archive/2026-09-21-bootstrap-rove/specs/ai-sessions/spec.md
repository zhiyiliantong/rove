## Purpose

定义每个对等设备的 AI 配置、会话归属和并发执行行为，让多个用户或窗口能同时推进不同任务，并在断线、取消和进程重启时理解实际执行状态而不丢失对话历史。

## ADDED Requirements

### Requirement: Persistent content-based conversation titles
agent MUST 为新会话持久记录 title_source。SessionCreate.auto_title=true 或省略标题 MUST 允许从首条已接受消息提取不超过 32 字符的简短标题，超长附省略号；MUST NOT 额外请求模型。显式设置标题 MUST 标记 manual 且不可被后续自动命名覆盖，旧记录无标记时 MUST 保留原名。

#### Scenario: Remote acceptance is recovered after a manual rename
- **WHEN** 发起端未收到远端接受响应，用户已手动改名，随后查询恢复同一作业
- **THEN** 确认结果不覆盖手动名称，不重复提交，创建幂等输入不变，重启保留名称和来源

### Requirement: Stable descending session lists
会话列表 MUST 支持 OpenAPI order=asc|desc 参数，GUI 与 CLI 默认最新创建在前；省略 API 参数保持升序兼容。排序依据 MUST 为 created_at 和 session_id，游标 MUST 绑定排序和筛选，消息本身保持正常时间顺序。

#### Scenario: Load older sessions while newer sessions arrive
- **WHEN** 用户读取倒序第一页，期间插入新会话或删除第一页末项，再使用原游标继续
- **THEN** 返回游标之后的较旧会话，不重复已读项，不漏掉既有较旧项，变更排序复用游标会报错

### Requirement: Persistent archive lifecycle
正式 agent MUST 保存会话归档时间，归档不取消或重启既有作业；已归档会话仍可查询历史与进度，但新作业和标题修改 MUST 返回 409 session_archived。恢复 MUST 使用同一 ID，重复归档 MUST 保留首次时间。列表可按 archived 过滤，省略时保留旧接口列出全部的语义。

#### Scenario: Archive and recover across clients
- **WHEN** 一个 SDK 客户端归档会话，另一个客户端或重启后的 agent 读取该会话
- **THEN** 归档时间与历史保留，恢复后可继续使用原会话，任务不重放

### Requirement: Deletion preserves execution deduplication
永久删除 MUST 仅允许已归档且没有 queued/running/cancelling 作业的会话；MUST 在事务中移除消息、工具上下文、作业、事件和输出，不删除服务或设备文件。系统 MUST 保留仅含 request_id 的删除墓碑，旧请求再次提交 MUST 返回 409 request_deleted，不能因删除历史重复执行副作用。SQLite 升级 MUST 备份，旧会话默认未归档。

#### Scenario: Retry a deleted job
- **WHEN** 会话已删除，旧客户端以曾被接受的 request_id 重试
- **THEN** 返回 request_deleted，不重新创建会话或作业，也不保留原输入正文在墓碑中

### Requirement: Per-device default model configuration
每台 agent MUST 独立保存默认提供方、接口地址、模型及所需凭据，支持本机与可信远端设置。网络加入 MUST NOT 自动复制模型配置；未配置模型 MUST 返回明确错误且不执行工具。

#### Scenario: Configure remote device
- **WHEN** 用户在 A 上为 B 设置有效模型配置并向 B 提交消息
- **THEN** B 使用自身保存的配置执行 AI，A 的默认模型配置不变

#### Scenario: Missing model configuration
- **WHEN** 向没有模型配置的目标提交 AI 作业
- **THEN** 目标报告需要配置模型，不执行部署命令或静默借用另一设备的凭据

### Requirement: Explicit API configuration synchronization
用户 MUST 能显式选择来源 API 连接和目标设备，同步连接及其全部型号名称。密钥 MUST 仅在 agent 间传递，不返回 GUI/CLI，不进入网络分享。目标 MUST 原子保存，重复相同快照 MUST 不创建副本；内容变化或替换已有连接 MUST 要求覆盖确认。默认模型 MUST 不被同步请求自动改变；替换时保留仍存在型号的目标 ID，不修改来源。账号 MUST 在各设备独立官方授权，MUST NOT 同步账号 session 或 OAuth 令牌。

#### Scenario: Synchronize and disconnect the source
- **WHEN** 用户将 API 连接同步到 B 后断开 A
- **THEN** B 保留独立凭据与型号，不依赖 A 在线；后续 A 修改配置不会自动改变 B

#### Scenario: Conflicting target or repeated request
- **WHEN** 同步快照已存在或目标内容有变化
- **THEN** 相同来源和内容返回已有记录；变化没有明确覆盖时返回冲突，事务不写入半份数据

#### Scenario: Account login is excluded
- **WHEN** 用户尝试同步账号登录连接或同步请求含 session/refresh_token
- **THEN** 请求被拒绝，界面提示逐设备登录；不将会员凭据转换为 Rig API key

### Requirement: Origin-owned sessions retain explicit execution links
新版 GUI 创建的会话 MUST 由发起端 agent 持久保存；本机执行仍使用原会话实现，远端执行 MUST 保存不可变的 network_id/device_id 与远端会话关联。执行设备 MUST 保存真实作业和工具事实；客户端关闭或发起端重启 MUST NOT 中断远端已接受作业，也不得自动重放未确认请求。旧的执行端会话 MUST 保留原记录，不静默迁移。

#### Scenario: Continue from another device
- **WHEN** 用户从另一客户端打开目标设备已有的会话
- **THEN** 它读取同一会话历史，并可在该目标上提交后续消息

#### Scenario: Origin restarts after remote acceptance
- **WHEN** 发起端退出或重启，执行端仍在线且原作业继续执行
- **THEN** 本机重新列出原会话关联，查询/订阅同一远端作业；不在本机创建替代作业，不将远端标为 interrupted

#### Scenario: Remote submit response is lost
- **WHEN** 远端接受作业后的响应丢失
- **THEN** 发起端保留原 request_id、消息和型号选择；只读刷新可关联已接受记录，显式重试沿用原输入，不自动重新提交；不同内容复用请求 ID 返回冲突

#### Scenario: Remote progress is unavailable
- **WHEN** 执行设备暂不可达，发起端已有保存的快照
- **THEN** 返回有明确 sync_error 的缓存进度与水位，而非虚假实时状态；事件订阅明确失败且不取消远端作业

#### Scenario: Delete a linked conversation
- **WHEN** 用户删除已归档远端关联
- **THEN** 未确认请求或已知非终态作业阻止删除；允许删除时只清理本机关联和缓存并保留请求墓碑，执行端历史与部署文件不被连带删除

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

### Requirement: Shared model connections and persistent selection
一个设备 MUST 允许多个连接及多个型号；型号引用共享凭据，目录读取 MUST 不回显密钥。批量添加 MUST 原子完成，配置名称可自动生成和手动修改。会话模型选择 MUST 持久化；缺失选择时才使用默认配置。会员认证没有真实适配器时 MUST 明确不可用。

#### Scenario: Selected model removed while jobs are queued
- **WHEN** 一个已接受作业选择的型号在开始前被移除，且设备另有默认模型
- **THEN** 该作业明确失败，不调用其他模型；已开始作业仍使用原快照，历史与请求去重保留

#### Scenario: Discover provider models
- **WHEN** 用户请求获取供应商型号列表
- **THEN** 执行设备使用相应认证枚举型号，不保存本次发现凭据，不泄露供应商错误正文，超限或失败明确返回；枚举结果不等于推理授权

#### Scenario: First model onboarding survives app closure
- **WHEN** 新设备第一次成功批量添加模型
- **THEN** agent 同事务创建唯一初始网络引导会话，退出应用后仍可读取，归档或删除后不重复创建；升级旧配置不插入额外会话

### Requirement: Explicit bounded model inference test
agent MUST 提供 OpenAPI 定义的 `test_model`，接受已保存型号 ID 或待保存的连接与型号（二选一）。测试 MUST 采用与正式对话相同的 Rig 协议适配器发起简短流式请求，不带工具、不创建会话/作业；MUST 设置短于 SDK 请求期限的超时，不回显密钥或提供方错误正文。DeepSeek 根地址和 /v1 MUST 使用 OpenAI 兼容协议，/anthropic 和 /anthropic/v1 MUST 使用 Anthropic 兼容协议。

#### Scenario: Configuration changes or a subsequent test fails
- **WHEN** 地址、密钥、提供方或型号改变，或该配置后来测试失败
- **THEN** 目录不再显示该配置测试通过；仅更改显示名称不影响结果，重启保留已验证配置的测试时间。未保存连接的测试仅保存私有指纹与结果，不保存明文密钥，不代表其他型号可用。
