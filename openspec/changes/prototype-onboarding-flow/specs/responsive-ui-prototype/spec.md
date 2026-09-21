## MODIFIED Requirements

### Requirement: First model onboarding
首次保存模型后原型 MUST 自动建立且只建立一次“初始化网络”本机会话，提供创建或加入网络路径。创建后 MUST 显示只包含网络名称、虚拟网段、网络密钥、初始节点的演示名片。四页引导进行中 MUST 保留引导上下文：第二页保存模型后进入第三页连接设备；从第四页场景进入模型配置时，保存后继续所选会话草稿，不自动发送。

#### Scenario: Add another model
- **WHEN** 用户已完成首次模型添加后再次添加型号
- **THEN** 原型保留原会话，不重复初始化，也不切换当前网络

#### Scenario: Configure a first model in the guide
- **WHEN** 用户在第二页添加第一个模型
- **THEN** 初始化会话只创建一次，页面进入第三页，用户可直接创建或加入网络，不跳出引导

### Requirement: Conversation driven services and resumable demonstration
服务页 MUST 只供查看与模拟访问，不提供服务管理按钮。原型 MUST 展示会话任务进度并在重新加载后恢复，明确这不证明真实后台执行。

音乐、私人影院和多台代码代理 MUST 作为部署第三方开源软件的使用场景表达：Rove 负责设备联通管理，由各设备的 rove-agent 部署配置软件并提供服务访问；专业业务及代理协作由第三方软件承担。MUST NOT 将场景提示描述为 Rove 已内建播放器、影音管理或多代码代理业务编排引擎。

对话 MUST 使用用户右侧、助手左侧的气泡布局，MUST NOT 以进度条或独立执行卡取代聊天。执行状态 MUST 以文字在助手气泡展示，保留取消与折叠详情；有关联的任务结果 MUST 原位更新，MUST NOT 重复追加同一结果。历史未关联消息和任务 MUST 保留。

助手气泡 MUST 支持模拟流式累积、Markdown 富文本与公式；MUST 禁止不可信 HTML 执行和自动外部图片请求。整条回复、代码、公式及说明段落 MUST 能单独复制；浏览器剪贴板不可用时 MUST 提供手动复制。朗读 MUST 由用户点击触发、可停止，能力不足时明确提示，MUST NOT 自动播放或静默改用云端语音。

#### Scenario: Rich streaming reply and local actions
- **WHEN** 用户发送“演示富文本和公式”
- **THEN** 助手气泡逐步输出固定演示 Markdown，完成后代码和公式可分别复制原文，整条可复制/朗读；无浏览器能力时提供真实降级反馈，刷新不重提任务

#### Scenario: Start with a conversation suggestion
- **WHEN** 用户打开四页引导的第四页或尚未发送消息的普通会话
- **THEN** 除原有建议外展示“建立自己的私人影院”和“管理多台代码代理”；点击仅填入草稿，保留已有输入，不自动创建执行任务；初始化网络会话不展示这些建议，欢迎标语留在引导第一页，会话列表不堆叠欢迎内容

#### Scenario: Discover Rove management through conversation
- **WHEN** 用户点击“管理 Rove”建议或设置页的“通过对话管理”
- **THEN** 原型新建目标为此设备的会话并填入检查空间和运行情况的草稿，不自动执行、不继承旧远端目标、不覆盖旧草稿；设置页保留手动配置并说明当前仅演示，以及正式接入后的删除/迁移/中断需说明影响并确认
- **AND** 无模型时提示先添加模型而不新建管理会话，不重复首次模型添加后的初始化会话；引导第四页发起的配置成功后继续所选管理草稿

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
