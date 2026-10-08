## ADDED Requirements

### Requirement: Assisted network creation and card transition
正式 GUI 的创建网络表单 MUST 预填可编辑的 `net-xx` 风格名称，并在初始节点区域提供获取公共节点入口。初始节点输入 MUST 直接接受四种发现 URI和七种最终连接 URI；对发现 URI执行“测试连通”时自动预览解析链，不提供需要先复制到另一输入区的独立解析步骤。候选结果 MUST 在用户确认后才加入表单，保存时保留原始发现 URI。加载、未配置、空结果、逐项失败、陈旧缓存和目录整体失败 MUST 可区分。设置页 MUST 允许在所选 agent 上配置或清除发现服务根地址。

创建成功后，GUI MUST 立即进入该新网络的网络名片视图，名片 MUST 来自 agent 返回的已保存加入配置，而非提交前表单副本。创建失败 MUST 保持表单、用户修改及候选选择；从引导弹窗创建时名片 MUST 保持可见，直到用户明确关闭或继续，不得因父级弹窗状态重置而闪退。

发现 URI 的“测试连通”操作 MUST 先通过所选目标 agent 解析到最终节点，再对最终节点执行受支持的有限探测，并展示本次解析链；该操作 MUST NOT 用最终节点覆盖表单中的原始发现 URI，也 MUST NOT 把解析或端口可达描述为入网成功。

网络标签 MUST 遵循已确认原型的互斥视图结构：网络标签只展示网络列表，创建和加入入口 MUST 只放在右上角全局「＋」菜单中并打开独立弹窗，创建表单、加入表单、网络名片和编辑表单不得铺在列表页。已有网络 MUST 从列表显式进入名片；创建成功 MUST 直接替换为该网络名片；返回操作 MUST 回到网络列表而不是留下隐藏或重复表单。

#### Scenario: Create from the regular network page
- **WHEN** 用户提交有效创建表单且 agent 成功保存网络
- **THEN** GUI 刷新网络列表并立即显示新网络名片，名片中的名称、密钥、地址模式和最终初始节点与 agent 已保存配置一致

#### Scenario: Create from onboarding
- **WHEN** 用户在四步引导的创建网络弹窗中创建成功
- **THEN** 同一弹窗显示新网络名片，用户关闭名片后仍回到引导的连接设备步骤

#### Scenario: Creation fails after selecting public nodes
- **WHEN** 用户已选择公共节点但创建请求被 agent 拒绝或网络请求失败
- **THEN** GUI 显示错误并保留名称、密钥、地址模式和初始节点，不显示名片或虚构网络记录

#### Scenario: Network list uses prototype navigation
- **WHEN** 用户打开网络标签并在列表、创建、加入、名片或编辑之间切换
- **THEN** 网络标签显示列表，右上角「＋」分别提供创建和加入；一次只打开一个操作弹窗，创建成功转到名片，关闭弹窗后回到列表

### Requirement: Public discovery remains usable on responsive clients
公共节点选择、TXT 解析结果和网络名片 MUST 在正式 GUI 的桌面及移动布局中完整可达，长 URL MUST 在自身区域换行或滚动而不造成整页横向溢出；所有新增操作、状态和错误 MUST 提供中英文文本及可访问名称。

#### Scenario: Select a node on Android-sized viewport
- **WHEN** 用户在窄屏设备获取多个公共节点并展开一个长连接地址
- **THEN** 用户仍可查看状态、选择节点并提交创建，底部导航和主要操作不被横向内容挤出视口

#### Scenario: Compact network navigation and mobile card
- **WHEN** 用户打开网络列表、名片或网络操作表单
- **THEN** 列表不重复显示“我的网络”标题和通用长说明，刷新以页头图标提供；操作视图通过左上角带可访问名称的回退图标返回，不显示“返回网络”文字按钮；手机竖屏和紧凑横屏使用全屏布局，固定回退栏和名片复制栏，中间内容独立滚动并适配安全区域

### Requirement: Devices belong to network details
网络列表中的条目 MUST 能进入该网络的独立详情页面。详情 MUST 展示网络身份、实际状态和可用的 overlay 地址，并在其下展示绑定同一 `network_id` 的设备。网络名片与管理操作 MUST 位于该网络上下文中；列表页 MUST NOT 提供跨网络全局设备浏览入口。网络详情和设备请求 MUST 使用相同的目标 agent，不能把远端网络状态与本地网关的设备列表拼接；不支持的二次远端选择 MUST NOT 静默执行。

#### Scenario: Open one network and return
- **WHEN** 用户点击网络列表中的条目
- **THEN** 导航到该 network_id 的详情页并请求其状态及设备，显示该网络名称；左上角回退回列表，名片/编辑关闭后仍回原网络详情

#### Scenario: Switching networks rejects stale or mismatched data
- **WHEN** 用户切换网络，或设备响应中存在不同的 network_id
- **THEN** 旧请求不得更新新详情，设备列表不得显示跨网络行；失败明确展示，不伪造在线设备或回退为其他网络
