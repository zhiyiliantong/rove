# responsive-ui-prototype Specification

## Purpose

提供独立于实际设备运行时的浏览器交互原型，让用户在手机、平板和桌面上评审同一套功能与体验，并明确区分演示状态、真实平台能力和后续需要接入的设备操作。

## Requirements

### Requirement: Independent non-overlapping network connections
原型 MUST 允许本机同时连接多个虚拟 IPv4 范围不相交的网络。MUST 检查不同掩码导致的包含关系；浏览和连接新网络 MUST NOT 隐式断开其他网络。DHCP 未返回实际范围时 MUST 显示等待而不是已连接。网络密钥 MUST 默认随机十六进制并支持显隐；初始节点 MUST 支持多项和逐项模拟测试。

#### Scenario: A broader CIDR contains an already joined subnet
- **WHEN** 本机连接了 192.168.100.0/24 后尝试连接 192.168.100/16
- **THEN** 原型按 192.168.0.0/16 判断重叠，拒绝新连接并保持原网络连接

#### Scenario: DHCP produces a conflicting range
- **WHEN** 待分配网络收到与已连接网络重叠的模拟网段
- **THEN** 该网络进入冲突状态，原连接不变，用户可修改配置后重试；离线未分配时可以取消等待

#### Scenario: Edit a tested initial peer
- **WHEN** 用户测试多个初始节点后编辑其中一条地址
- **THEN** 只有该条测试结果失效，不将旧响应显示为新地址的结果，不向真实地址发出探测

### Requirement: Vendor and authentication selection
服务商 MUST 表示厂商而非认证类型；切换厂商 MUST 自动填写可编辑的默认接口地址。API 密钥与账号登录 MUST 二选一。不支持的账号适配器 MUST 明确不可用，切换输入 MUST 使旧验证与型号选择失效。

#### Scenario: Change vendor after simulated account login
- **WHEN** 用户在 OpenAI 模拟账号登录后切换 DeepSeek
- **THEN** 地址更新为 DeepSeek 官方默认地址，认证变为 API 密钥，旧登录/型号选择不沿用，账号登录入口标为尚未接入

### Requirement: Platform navigation and conversation controls
会话加号中的模型入口 MUST 打开模型选择；MUST NOT 显示执行设备下拉。桌面 MUST 提供文件入口而不提供拍照/照片专用入口；移动端 MUST 在加入网络内提供扫一扫，MUST NOT 在全局菜单重复设置扫一扫；导航 MUST 保持底栏，包括横屏。

#### Scenario: Rotate a mobile preview
- **WHEN** 移动平台或移动外观从竖屏切换到横屏或平板宽度
- **THEN** 会话/服务/网络仍在底部，不显示桌面侧栏；扫一扫仍可访问且标注演示

### Requirement: Independent browser preview
原型 MUST 在普通浏览器中提供可交互预览，不要求安装 Rove、Tauri 或启动 agent；MUST 持续标明模拟数据，不执行真实部署或保存真实凭据。

#### Scenario: Review without native runtime
- **WHEN** 用户在未安装 Rove 的浏览器中打开预览地址
- **THEN** 用户可以浏览和操作演示流程，并明确看到原型标识，不收到缺少原生桥接的错误

### Requirement: Responsive functional parity
原型 MUST 在手机、平板、桌面和连续窗口缩放时保持主要功能可达，不因导航收起而丢失操作；普通页面 MUST NOT 因长名称、UUID 或 URL 产生整页横向溢出。

#### Scenario: Resize while editing
- **WHEN** 用户正在填写表单或编辑对话输入并调整窗口宽度或横竖屏
- **THEN** 输入与目标保持不变，导航和主要操作仍可访问，内容重排而不是整体缩成不可读的桌面画面

### Requirement: Consistent theme and interaction
原型 MUST 提供统一的明暗主题、组件状态、可见键盘焦点和可理解的表单错误；状态 MUST NOT 只用颜色区分。

#### Scenario: Change theme and operate with keyboard
- **WHEN** 用户切换明暗主题并使用键盘访问导航和表单
- **THEN** 当前页面和输入保留，焦点可见，文本、错误与按钮在所选主题下保持可读

### Requirement: Reviewable scenario states
原型 MUST 覆盖经确认的设备、网络、对话/作业、服务与设置流程，并提供可重置的空状态、在线、离线、排队、失败与 unsupported 场景；MUST NOT 将模拟网络连接或工具结果宣称为实际设备操作成功。

#### Scenario: Unavailable device or platform capability
- **WHEN** 演示目标离线或请求未提供的平台能力
- **THEN** 原型显示相应不可达或 unsupported 反馈，不偷偷改为本机成功，并允许恢复到可继续评审的场景

### Requirement: Explicit execution context
原型 MUST 显示执行操作所针对的设备与必要网络上下文，并区分浏览范围、当前连接和目标设备；会话归发起设备本地 agent，执行任务及服务归执行设备。

#### Scenario: Browse another saved network
- **WHEN** 用户查看另一网络的保存配置
- **THEN** 原型不将该浏览行为报告为自动加入网络，也不把已有会话迁移到另一设备

### Requirement: First model onboarding
首次保存模型后原型 MUST 自动建立且只建立一次“初始化网络”本机会话，提供创建或加入网络路径。创建后 MUST 显示只包含网络名称、虚拟网段、网络密钥、初始节点的演示名片。

#### Scenario: Add another model
- **WHEN** 用户已完成首次模型添加后再次添加型号
- **THEN** 原型保留原会话，不重复初始化，也不切换当前网络

### Requirement: Shared connection and multiple model records
原型 MUST 允许一次演示认证后批量选择型号，每型号一记录，共享连接凭据引用；接口地址 MUST 可编辑，默认模型 MUST 单选。原型 MUST NOT 保存或发送用户真实账号凭据。

#### Scenario: Edit shared connection
- **WHEN** 用户修改两个型号共用的连接配置
- **THEN** 两个型号引用更新后的连接，删除其中一个不删除另一个

### Requirement: Generated model names and discoverable catalogs
每条模型 MUST 有按厂商、型号、序号生成的独立显示名称，不修改真实型号 ID；批量和重复导入 MUST 避免显示名称冲突。原型 MUST 在认证前展示按厂商区分的常用型号，标注来源/核对日期及未验证权限；MUST 支持模拟 API/账号获取、失败回退与手动填写。预置列表 MUST NOT 伪装成账号已授权清单。

用户 MUST 能分别手动修改自动生成的显示名称，并恢复自动名称。保存 MUST 去除首尾空格，拒绝空名称、批内重名和与现有型号记录重名；失败 MUST NOT 部分写入连接/型号。型号 ID 和共享凭据引用 MUST 不因名称修改而改变。

预置型号 MUST 默认全部选中，并允许取消；供应商切换或表单失效重置时 MUST 选择当前目录全部预置，不沿用其他供应商 ID。

#### Scenario: Default model selection
- **WHEN** 用户打开添加模型或切换到另一供应商
- **THEN** 当前供应商预置型号全部选中，显示各条可编辑名称，用户可取消不需要的项

#### Scenario: Override display names in a batch
- **WHEN** 用户选择两个型号，分别修改名称并保存
- **THEN** 两条模型保存各自名称和原始型号 ID，仍共享连接，刷新可恢复；重复或空名称显示错误，不产生部分记录

#### Scenario: Repeat a batch import
- **WHEN** 用户再次导入同厂商同型号
- **THEN** 每条新记录使用新的 UUID 与递增显示名，旧会话引用不改变，批量凭据仍只由共享连接引用

#### Scenario: Discovery fails
- **WHEN** 模拟型号获取失败
- **THEN** 页面保留预置与手动输入，明确错误，不标为已验证；账号模式仍需登录成功方可保存

### Requirement: Searchable provider presets
原型 MUST 提供按中文/英文名称、别名或类别可搜索的供应商目录，覆盖模型厂商、聚合推理平台和本地服务；默认地址 MUST 可编辑，并展示地区/工作空间/套餐/协议注意事项。目录 MUST 由独立原型 OpenAPI 定义。新增供应商 MUST NOT 自动获得账号登录能力，也不能被表示为已经安装的适配器；参考型号 MUST NOT 冒充实际账号权限或本地安装列表。

新增目录 MUST 以项目固定 Rig 0.42.0 的专用 provider 模块筛选现有项，并返回明确 rig_provider 映射；MUST NOT 仅因为接口可能兼容而保留无专用模块的供应商或自定义新增入口。旧配置 MUST 保留并可编辑，不改写 provider、地址、型号或会话引用；退出目录的旧项 MUST NOT 因编辑而重新出现在新增目录。筛选依据 MUST 与 Rove 实际接入及型号能力验证区分。

#### Scenario: Filter unsupported new-provider presets
- **WHEN** 用户打开添加模型或搜索通义千问、火山方舟、硅基流动、自定义
- **THEN** 仅显示 12 个保留供应商的匹配结果；不匹配时提示更换关键词，不再引导使用自定义入口

#### Scenario: Edit a domestic GLM legacy connection
- **WHEN** 用户打开已有 zhipu 配置
- **THEN** 页面标记旧配置，保留国内站地址和原身份；新建 GLM 使用 zai 国际站预设，不自动迁移原账号或型号

#### Scenario: Switch from an authenticated account to an API provider
- **WHEN** 用户搜索并选择未接入账号登录的新供应商
- **THEN** 页面填入其默认地址和参考型号，切回 API 方式，禁用账号登录，清除前一供应商的模拟认证状态与型号选择，不发送外部请求

#### Scenario: Preserve existing custom configuration
- **WHEN** 供应商目录扩展后用户重新打开已有配置
- **THEN** 已保存的地址、型号、显示名称和引用保持不变；未知旧供应商使用自定义元数据回退，不因列表顺序变化误认其他供应商

### Requirement: Conversation driven services and resumable demonstration
服务页 MUST 只供查看与模拟访问，不提供服务管理按钮。原型 MUST 展示会话任务进度并在重新加载后恢复，明确这不证明真实后台执行。

音乐、私人影院和多台代码代理 MUST 作为部署第三方开源软件的使用场景表达：Rove 负责设备联通管理，由各设备的 rove-agent 部署配置软件并提供服务访问；专业业务及代理协作由第三方软件承担。MUST NOT 将场景提示描述为 Rove 已内建播放器、影音管理或多代码代理业务编排引擎。

对话 MUST 使用用户右侧、助手左侧的气泡布局，MUST NOT 以进度条或独立执行卡取代聊天。执行状态 MUST 以文字在助手气泡展示，保留取消与折叠详情；有关联的任务结果 MUST 原位更新，MUST NOT 重复追加同一结果。历史未关联消息和任务 MUST 保留。

助手气泡 MUST 支持模拟流式累积、Markdown 富文本与公式；MUST 禁止不可信 HTML 执行和自动外部图片请求。整条回复、代码、公式及说明段落 MUST 能单独复制；浏览器剪贴板不可用时 MUST 提供手动复制。朗读 MUST 由用户点击触发、可停止，能力不足时明确提示，MUST NOT 自动播放或静默改用云端语音。

#### Scenario: Rich streaming reply and local actions

- **WHEN** 用户发送“演示富文本和公式”
- **THEN** 助手气泡逐步输出固定演示 Markdown，完成后代码和公式可分别复制原文，整条可复制/朗读；无浏览器能力时提供真实降级反馈，刷新不重提任务

#### Scenario: Start with a conversation suggestion
- **WHEN** 用户打开欢迎页或尚未发送消息的普通会话
- **THEN** 除原有建议外展示“建立自己的私人影院”和“管理多台代码代理”；点击仅填入草稿，保留已有输入，不自动创建执行任务；初始化网络会话不展示这些建议

#### Scenario: Discover Rove management through conversation
- **WHEN** 用户点击“管理 Rove”建议或设置页的“通过对话管理”
- **THEN** 原型新建目标为此设备的会话并填入检查空间和运行情况的草稿，不自动执行、不继承旧远端目标、不覆盖旧草稿；设置页保留手动配置并说明当前仅演示，以及正式接入后的删除/迁移/中断需说明影响并确认
- **AND** 无模型时提示先添加模型而不新建管理会话，不改变首次模型添加后的初始化网络流程

#### Scenario: Code agent deployment suggestion
- **WHEN** 用户选择“管理多台代码代理”
- **THEN** 草稿明确为安装配置第三方开源代码代理并通过 Rove 访问服务，不自动发起代理协作或宣称原型已完成部署

#### Scenario: Read a reply with compact icon actions
- **WHEN** 用户在桌面或移动端阅读助手回复
- **THEN** 助手正文无整体有色外框，公式与代码使用独立浅色圆角块和右上角复制图标，长行只在块内滚动；整条复制及朗读图标位于底部，均有可访问名称与键盘操作，深色主题保持可读
- **AND** 说明复制不另占一行，桌面悬停或键盘聚焦可见，触屏常显；不新增点赞或分享行为，不改变现有流式和复制源文语义

#### Scenario: Job completes within a conversation
- **WHEN** 用户发送请求，任务从排队/执行进入完成
- **THEN** 请求后同一个助手气泡从状态文字更新为最终结果，无进度条；刷新不重复任务或回复

#### Scenario: Reopen an accepted task
- **WHEN** 用户发送模拟任务后刷新或重新打开原型
- **THEN** 原会话恢复并按演示时间线显示进度，不重复提交任务，目标不可达时显示等待连接
