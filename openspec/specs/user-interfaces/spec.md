# user-interfaces Specification

## Purpose

定义图形界面与命令行作为同一应用的两种交互方式，确保没有图形环境的用户仍可完成网络、设备、AI 和服务操作，并使多设备漫游时目标与执行结果清晰可见。

## Requirements

### Requirement: Icon navigation and inline conversation naming
移动底栏 MUST 只显示图标，保持横向全宽，仅降低高度，并保留双语无障碍名称和至少 44px 触控区域。会话标题 MUST 原地编辑、回车/失焦保存、Esc 取消，MUST NOT 通过弹窗改名；空白或保存失败保留输入，输入法确认不得误提交。

#### Scenario: Switch conversations during title save
- **WHEN** 改名尚未返回时用户切换会话
- **THEN** 响应仅更新原会话，当前会话、草稿与标题编辑不被旧请求覆盖

### Requirement: Archive management exposes per-row recovery and deletion
加号菜单 MUST 移除“归档会话”，保留“归档管理”。普通会话列表 MUST 保留归档入口，归档列表 MUST 在每条记录旁提供恢复和删除。删除 MUST 明确目标并二次确认，检查该目标的活跃作业和未确认请求，网络检查失败不得当作可删除。

#### Scenario: Deleting an unselected archived conversation
- **WHEN** 用户点击非当前选中归档会话的删除
- **THEN** 弹窗名称、检查请求及删除操作均使用该行 ID；只有确认后才删除，取消无副作用，后端拒绝后仍保留记录

### Requirement: Instant messaging conversation layout
Rove GUI SHALL display one continuous conversation, without a separate history panel, in a viewport-bound shell with a stationary title and composer. Clicking the title SHALL allow renaming an active conversation using the existing OpenAPI session update. Mobile details SHALL use the top-level back button, without a duplicate device header. Archive management SHALL remain available in the global add menu; archiving an individual conversation SHALL use the conversation list.

#### Scenario: Read earlier messages during streaming
- **WHEN** the user scrolls upwards while a response streams
- **THEN** the reading position and top bar SHALL remain stable, and an explicit latest-message action SHALL resume following output

#### Scenario: Turn off suggestions
- **WHEN** the user disables the default-on conversation suggestions setting
- **THEN** empty conversations SHALL hide suggestions across restart without changing the agent configuration or submitting a job

### Requirement: Compact model selection and Android TLS
Model selection SHALL combine each provider model identifier and its editable generated configuration name. Leaving completed credential fields SHALL fetch the provider directory without automatic inference; failure SHALL retain a usable reference/manual list and stale replies SHALL NOT overwrite newer input. Android SHALL initialize and package the matching native platform certificate verifier before agent networking, and SHALL NOT bypass certificate validation.

#### Scenario: Fetch model identifiers without inference
- **WHEN** a user enters an API credential and leaves the field
- **THEN** the GUI SHALL query the existing model discovery API, default newly discovered models to selected, and preserve edited configuration names on refresh

#### Scenario: Untrusted HTTPS server
- **WHEN** an Android model provider presents an untrusted certificate
- **THEN** the agent SHALL reject the connection rather than bypass verification or accept the model as tested

### Requirement: Persistent four-step introduction
正式 GUI MUST 提供欢迎、模型配置、连接设备、开始对话四步导航，支持中文/英文及窄屏。配置 MUST 使用真实本机 agent；动画 MUST 标注教学而非连接状态。第四页建议 MUST 仅准备新会话草稿，无模型时先配置后续接，不自动提交。会话列表 MUST 隐藏重复代理提示，新建与归档入口集中在顶栏加号。

#### Scenario: Replay without data loss
- **WHEN** 用户在设置重置导航页
- **THEN** 当前页面不跳转，下次启动显示第一页，原有身份、模型、网络、会话、草稿和作业均保留

#### Scenario: Resume a suggestion after adding models
- **WHEN** 未配置模型的用户点击第四页建议并成功保存模型
- **THEN** 创建本机会话并填入原建议草稿，不因旧初始网络会话跳离导航，不提交作业

### Requirement: Chinese and English interface localization
正式 GUI MUST 支持简体中文与英文，首次根据系统语言选择，允许在设置切换并持久保存。界面文本、状态、时间格式和无障碍标签 MUST 使用统一语言资源；切换 MUST NOT 改写历史用户内容或中断执行。

#### Scenario: Switch language during a conversation
- **WHEN** 用户已有草稿和执行中的作业并切换语言
- **THEN** 导航与操作文案即时切换，草稿、用户命名、模型型号、已发生内容与订阅保持不变，重启应用保持语言选择

#### Scenario: Unsupported system locale
- **WHEN** 初始系统语言不属于中文或英文，且用户没有保存偏好
- **THEN** 界面回退英语，设置仍允许选择简体中文

### Requirement: Accepted prototype guides the shipping interface
正式 GUI MUST 按已验收原型提供会话、服务、网络分区、设置和模型次级入口、桌面侧栏及窄屏底栏；MUST 使用真实 SDK/agent 状态，MUST NOT 引入演示种子或用 mock 结果宣称执行成功。GUI 的服务管理入口 MUST 转为会话，服务页只查看和访问；既有 CLI/API 管理功能保留兼容。尚未迁移的后端语义 MUST 明确标注，不冒充已完成。

#### Scenario: Navigate during a running conversation
- **WHEN** 用户在正式 GUI 中切换服务或设置页面后返回会话
- **THEN** 会话选择、未发送草稿和已接受作业关联保留，继续展示同一作业的真实进度，切页不提交或取消任务

#### Scenario: Android system controls overlap the WebView
- **WHEN** Android 采用 edge-to-edge，系统栏、屏幕缺口或软键盘占用可视区域
- **THEN** 原生壳根据实际窗口 insets 调整 WebView 可用区域，顶部操作和移动底栏不被系统控件覆盖，不依赖固定机型像素或重复叠加 CSS 安全区域

### Requirement: Verified model before completing introduction
首次/重播引导 MUST 在至少一个已保存模型测试成功后才允许跳过或完成；获取型号列表不满足此条件。添加模型 MUST 提供实际测试入口、费用提示与密钥显隐，默认隐藏、关闭后恢复隐藏。长表单 MUST 支持纵向滚动，在短屏中不出现横向溢出；欢迎页主操作适配短屏。

#### Scenario: No verified model during introduction
- **WHEN** 当前设备没有配置或只有未经测试的型号
- **THEN** 跳过及完成入口不可用，提示添加/测试模型；测试失败不解锁，成功并保存后才允许继续，进入会话前再次读取 agent 目录验证。

### Requirement: Readable streaming responses
正式 GUI MUST 以用户消息和 agent 应答展示真实任务输出，支持安全富文本、代码和公式显示、全文及分段复制和用户主动触发的本地朗读；没有相应浏览器能力时 MUST 明确降级。来自模型的 HTML 和外部图片 MUST NOT 自动执行或加载，原始工具结果 MUST 仍可查阅。

#### Scenario: Copy user messages and failed replies
- **WHEN** 用户查看已发送消息或失败作业
- **THEN** 对应内容旁常显复制图标，复制消息原文或当前显示的错误代码和说明；复制成功只保留图标，不显示“已复制”文字，AI 回复及分段复制同样遵守；剪贴板不可用时提供可选择的只读文本及双语反馈，失败没有输出时仍可复制，复制不提交或重试作业。

#### Scenario: Render and copy untrusted streamed content
- **WHEN** 模型输出包含代码、公式或不可信 HTML
- **THEN** 界面安全显示内容并允许复制源文，不执行 HTML，不将输出送到云端朗读，也不丢失作业身份和取消入口

### Requirement: GUI and CLI functional parity
每项 GUI 业务操作 MUST 有对应 CLI 操作，覆盖网络创建、加入、分享、启动、停止、删除、设备查看、模型设置、会话操作、作业查询和取消、并发设置及服务管理。相机扫描与打开浏览器等界面动作 MUST 提供等价 URL 输入或地址输出方式。

#### Scenario: Complete workflow without GUI
- **WHEN** 用户仅通过 `rove` 在无界面设备操作
- **THEN** 用户能加入网络、选择设备、配置模型、对话、查询作业、发布服务并取得访问地址

### Requirement: Interactive and scriptable CLI
CLI MUST 提供可发现的交互式入口和明确子命令；查询和操作结果 MUST 支持结构化输出，执行失败 MUST 返回非零退出码。需要人工输入时 MUST 在无交互模式明确报错或接受对应参数。

#### Scenario: List devices in automation
- **WHEN** 调用 CLI 请求结构化设备列表
- **THEN** 标准输出为可解析结果，诊断信息不混入结果载荷

### Requirement: Explicit execution target and run state
GUI 与 CLI MUST 明确操作所处网络及目标设备，不要求在每条消息重复展示。GUI 对话 MUST 隐藏执行详情和成功后的“已完成”，保留运行中取消、失败和离线提示；CLI 与 API MUST 继续提供作业 ID、状态以及可查询输出。设备离线、协议不兼容和平台不支持 MUST 有可区分提示。

#### Scenario: Compact mobile conversation
- **WHEN** 手机用户进入具体会话
- **THEN** 底部主导航隐藏、顶栏收窄且图标触控区域至少 44px；返回会话列表恢复导航，桌面侧栏不受影响

#### Scenario: Direct model onboarding and network-scoped API sync
- **WHEN** 用户在引导页添加模型，或从本机网络列表发起 API 同步
- **THEN** 添加模型直接打开表单不经过列表；同步固定所选网络，仅允许选择该网络在线设备，不同步账号令牌

#### Scenario: Remote device becomes unavailable
- **WHEN** 用户尝试向离线目标发送消息
- **THEN** 界面报告目标不可达，不悄悄改为在当前设备执行

### Requirement: Clients attach to existing runtime
重复打开 GUI 或 CLI MUST 连接已有的本机运行实例；关闭界面 MUST NOT 隐式停止设备网络、服务代理或已接受作业。启动失败 MUST 提供可理解的处理信息。

#### Scenario: Switch from GUI to CLI
- **WHEN** 用户关闭 GUI 并通过 CLI 查看之前启动的作业
- **THEN** CLI 展示同一个作业的当前状态和输出
