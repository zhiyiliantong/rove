# conversation-archive-management Specification

## Purpose

为 Rove 独立浏览器原型提供可恢复的会话整理方式，让用户从日常列表归档历史会话，并在归档管理中安全查看、恢复或永久删除记录，同时保留进行中任务的上下文和既有浏览器数据。

## Requirements

### Requirement: Archive without losing context

用户 MUST 能从会话列表的操作菜单及会话详情归档会话。归档 MUST 从日常会话列表隐藏该记录，但保留标识、消息、草稿、模型及设备引用、初始化标识和任务记录；MUST NOT 取消或重启任务。归档当前打开的会话后 MUST 返回会话列表并提示可在归档管理恢复。

#### Scenario: Archive a conversation with a running task
- **WHEN** 用户归档一个含草稿且任务仍在进行的会话
- **THEN** 会话从日常列表消失，归档管理可找到该会话；草稿不丢失，原任务继续按现有模拟规则推进，不因归档中止

### Requirement: Accessible archive management

会话区及设置页 MUST 提供“归档管理”入口，共用归档页面。该页面 MUST 展示已归档会话的标题、归档时间、任务进行中提示和查看、恢复、删除操作；没有记录时 MUST 显示可理解的空状态。桌面和移动端 MUST 均可使用，不依赖鼠标悬停；长标题 MUST NOT 造成整页横向溢出。

#### Scenario: Open archive management on a narrow screen
- **WHEN** 用户在窄屏通过会话区或设置页进入归档管理
- **THEN** 看到相同的归档记录，能通过触摸或键盘使用查看、恢复和删除操作，也能返回正常会话列表

### Requirement: Read archived conversations and restore them

归档会话 MUST 可查看消息与任务进度；继续发送或修改草稿、模型及执行目标前 MUST 先恢复。恢复 MUST 使用原会话标识和内容，移回日常列表而不创建副本、不重新初始化网络、不重放任务。访问已删除或不存在的会话 MUST 提供返回列表的提示。

#### Scenario: Restore and continue
- **WHEN** 用户在归档管理恢复一个会话
- **THEN** 该条从归档列表消失，原标识对应的会话出现在日常列表，原草稿与历史内容保留，用户可继续对话

#### Scenario: Open an archived conversation using its old link
- **WHEN** 用户访问已归档会话的详情链接
- **THEN** 页面标注已归档并展示历史内容，提供恢复入口，不允许直接发送或修改配置

### Requirement: Confirmed deletion with task protection

归档管理 MUST 提供单条永久删除；确认内容 MUST 包含会话标题、不可恢复提示，以及会话消息、草稿和关联任务历史将删除的范围。取消确认 MUST 不修改任何记录。删除 MUST 仅适用于已归档且没有 queued、running、cancelling 任务的会话；拒绝删除 MUST 保留全部记录并说明原因。成功删除 MUST 清理对应会话及关联任务历史，不影响其他会话、模型、网络、设备和服务。MUST NOT 提供批量清空。

#### Scenario: Cancel deletion
- **WHEN** 用户点击删除但取消确认
- **THEN** 会话、草稿及关联任务历史完整保留

#### Scenario: Refuse deletion while a task remains active
- **WHEN** 用户尝试删除仍有排队、执行中或取消中任务的已归档会话
- **THEN** 删除被拒绝，提示等待完成或恢复会话后取消任务，数据不丢失

#### Scenario: Delete a completed archived conversation
- **WHEN** 用户确认删除没有进行中任务的已归档会话
- **THEN** 该会话及其关联任务历史被移除，刷新后不会重新出现，已部署服务及其他数据保持不变

### Requirement: Persistent state and explicit prototype contract

归档和恢复状态 MUST 在浏览器允许本地存储时跨刷新保留。旧版快照中的会话 MUST 默认未归档且保留既有内容；初始化网络会话被归档或删除 MUST NOT 重置首次引导状态。归档、恢复和删除 MUST 在原型 OpenAPI 契约中定义请求、响应、缺失记录和冲突错误；界面 MUST 保持仅模拟当前浏览器数据的边界，不宣称实现了真实跨设备删除或后台执行。

#### Scenario: Load a previous prototype snapshot
- **WHEN** 用户加载没有归档字段的旧版本快照
- **THEN** 既有会话正常显示且内容完整；归档并刷新后仍留在归档管理，不重复生成初始化会话

#### Scenario: Repeated actions and missing identifiers
- **WHEN** 客户端重复归档或恢复已处于目标状态的会话，或操作不存在的会话
- **THEN** 重复状态操作不产生副本或重写归档时间；不存在的标识返回契约定义的缺失记录错误，不产生新会话
