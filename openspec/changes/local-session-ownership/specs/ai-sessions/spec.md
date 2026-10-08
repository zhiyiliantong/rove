## ADDED Requirements

### Requirement: Runtime observation failures preserve network intent
共享网络生命周期 MUST 区分暂时状态观察失败与明确的权限、隔离及配置失败。暂时失败 MUST NOT 自动撤销用户启动意图或直接停止底层实例；未经重新验证的业务入口 MUST 关闭。自动重查 MUST 采用有上限的退避，成功后恢复入口和清除错误，不重放业务操作。历史禁用网络 MUST NOT 因升级自动启动。

#### Scenario: Temporary runtime observation failure
- **WHEN** 已启用网络的实例列表或状态读取暂时失败
- **THEN** enabled 保持 true，状态为 starting、展示自动重试错误并清空有效入口；后续观察及原有安全校验成功后恢复 running

#### Scenario: User stops a recovering network
- **WHEN** 用户在自动恢复期间停止网络
- **THEN** 立即取消退避等待并处理停止，enabled 保持 false，不自动重新启动

#### Scenario: Admission or permission failure
- **WHEN** 原生权限、接口、路由或隔离校验明确失败
- **THEN** 不把它当作普通连接故障自动绕过或重新弹出授权，保持安全失败及明确错误

### Requirement: Remote device operations start in conversations
GUI MUST NOT 提供设备行管理按钮或全局执行目标切换入口。网络浏览及别名编辑 MUST 保持本机上下文；远程操作 MUST 由会话明确指定目标。既有执行记录的兼容读取不受影响。

#### Scenario: Browse and rename a device
- **WHEN** 用户打开网络设备列表
- **THEN** 可以原位编辑本机别名，没有“管理此设备”入口；新建会话仍属于本机

### Requirement: Network editing preserves the network key
网络编辑 MUST 复用创建网络的名称、初始节点及地址配置项。key MUST 仅可查看，GUI 不可修改/重新生成，服务端 MUST 拒绝替换 key。运行中 MUST 允许保存，预先提示自动重启及短暂断连。共享流程 MUST 等待停止完成后写入配置，再等待运行状态后报告完成；原本停止的网络 MUST 保持停止。失败 MUST 区分停止失败、保存失败/结果不确定、已保存但启动未完成。

#### Scenario: Edit a stopped network
- **WHEN** 用户保存网络名称和初始节点
- **THEN** 保存至原 network_id，key 与原 EasyTier 身份名称不变；刷新显示修改后的配置

#### Scenario: Regenerate a card after editing
- **WHEN** 用户编辑保存网络后再次点击网络名片
- **THEN** MUST 重新读取最新已保存配置，生成同内容的 Base64 与二维码；旧二维码、复制状态和异步生成结果不得覆盖新名片

#### Scenario: View a running network
- **WHEN** 运行中的网络打开编辑视图
- **THEN** key 可显示但只读，保存按钮为“保存并重启网络”，提示连接短暂中断
- **WHEN** 用户保存
- **THEN** 自动停止、保存并重新启动该网络，等待实际状态完成，失败时显示具体阶段且保留编辑内容

### Requirement: Device aliases belong only to the local device
设备别名 MUST 仅保存在当前设备，同一目标在不同设备可有不同别名。别名 MUST 本机唯一，绑定 network_id/device_id 而不是固定 IP，不向网络同步。GUI MUST 支持本地编辑与清除；会话 MUST 使用本机解析结果调用对应 agent，不将未知或离线别名回退成本机执行。

#### Scenario: Independent names and changing addresses
- **WHEN** 两台设备为同一个远端设备设置不同别名，随后远端地址改变
- **THEN** 各自仅使用本地名称，解析返回原身份的当前地址；重名保存被拒绝，离线目标明确失败

#### Scenario: Edit a name in place
- **WHEN** 用户点击网络详情中的设备名称
- **THEN** 名称原位置变为输入框，不新增表单或弹窗；回车/失焦保存，Esc 取消，输入法选词不误提交，失败保留草稿

### Requirement: Native command output preserves local-language text
会话工具输出 MUST 在进入事件、历史和模型上下文前转换为 Unicode，stdout 与 stderr MUST 使用同一规则。Windows 本地代码页输出 MUST NOT 无条件作为 UTF-8 有损读取；读取分块 MUST NOT 拆坏正常多字节字符。不修改系统全局代码页，不重写旧损坏历史。

#### Scenario: Chinese ping output on Windows
- **WHEN** Windows agent 执行中文 ping，输出本地代码页字节
- **THEN** 新工具事件、结果与会话历史保留正确中文，UTF-8 程序输出仍正常

### Requirement: New GUI sessions belong to the local device
GUI 新建会话 MUST 始终由本机 agent 创建且不绑定 execution_target。设备浏览和配置目标 MUST NOT 改变新会话归属或默认模型来源。既有远端关联会话 MUST 保留原执行语义，不静默迁移、删除或重放。

#### Scenario: Create after selecting any device
- **WHEN** 用户选择本机或远端设备后新建会话
- **THEN** 创建请求发往本机且不带远端执行目标，新消息由本机 agent 编排

### Requirement: Delegate remote work inside a local conversation
本机 agent MUST 通过统一协议工具按明确的 network_id/device_id 调用远端 agent，允许发现、读取状态、创建远端执行会话、提交和查询作业。远端执行记录 MUST NOT 替换用户本机会话。远端不可达 MUST 明确返回错误，不回退成本机执行；重试提交 MUST 使用原 request_id。模型 MUST 不把本机 system_exec 当作远端命令执行。

#### Scenario: Remote execution and unreachable target
- **WHEN** 本机会话需要操作指定的其他设备
- **THEN** 工具连接该设备 agent，结果回到本机会话；连接失败不产生本地替代操作

### Requirement: Creation errors remain visible in session lists
创建失败 MUST 在桌面和手机列表页可见且可复制，后台刷新 MUST NOT 清除创建错误，显式重试成功后清除。

#### Scenario: Failed creation while viewing mobile list
- **WHEN** 手机列表页创建请求失败且后台轮询成功
- **THEN** 错误仍可见且可复制，用户可重试，不创建重复会话

## MODIFIED Requirements

### Requirement: Origin-owned sessions retain explicit execution links
新 GUI 会话 MUST 遵循本机归属规则，不再根据所选设备创建整段远端执行关联。旧版已创建的关联和显式 API 创建的执行记录 MUST 兼容保留不可变的 network_id/device_id 与远端会话关联。执行设备 MUST 保存真实作业和工具事实；客户端关闭或发起端重启 MUST NOT 中断远端已接受作业，也不得自动重放未确认请求。旧的执行端会话 MUST 保留原记录，不静默迁移。

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
