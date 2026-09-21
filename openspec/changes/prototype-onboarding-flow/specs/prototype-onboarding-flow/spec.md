## Purpose

在独立浏览器原型中提供连贯的首次使用引导，让手机与电脑用户了解模型、网络和设备的关系，并能开始会话或重新查看引导而不丢失已有配置和历史，以便在正式实现前审核交互。

## ADDED Requirements

### Requirement: Four page introduction
原型 MUST 将欢迎、添加模型、连接设备、开始对话合为四页引导；欢迎页 MUST 显示“你的设备，随你漫游”和“数据自己掌握，跳出平台控制”。MUST 支持前后切换、稍后配置和已有配置提示，且持续标明模拟边界。

#### Scenario: Configure while in the guide
- **WHEN** 用户在第二页保存模型或在第三页创建/加入网络
- **THEN** 对话框关闭后仍处于引导，不重复初始化会话，网络创建后可查看名片

### Requirement: Device connection tutorial
第三页 MUST 演示创建网络、分享名片、另一设备扫码或导入并加入的顺序，提供暂停和减少动态效果支持。MUST 提供独立于动画的创建与加入操作，不将动画当作真实设备连接结果。

#### Scenario: Reduced motion preview
- **WHEN** 浏览器偏好减少动态效果或用户暂停动画
- **THEN** 教学仍可通过静态步骤理解，所有操作仍可访问

### Requirement: Start from suggestions or an empty conversation
第四页 MUST 提供已有场景提示和“新会话”；点击提示 MUST 新建会话并填入对应草稿，不自动提交。没有模型时 MUST 引导添加模型并保留所选提示。

#### Scenario: Start from a cinema suggestion
- **WHEN** 用户已配置模型并选择私人影院提示
- **THEN** 进入新会话，保留影院草稿且没有新执行任务，引导结束

### Requirement: Quiet conversation list
会话列表 MUST 移除本机 agent/此设备等固定说明和重复新建/归档入口；新建会话与归档管理 MUST 可通过右上角加号访问。窄屏 MUST 单独展示列表或详情，不能在列表下堆叠欢迎页。

#### Scenario: Open conversations on a phone
- **WHEN** 用户结束引导后查看会话列表
- **THEN** 只显示会话记录或简短空状态，菜单能新建/打开归档，返回列表不重新播放引导

### Requirement: Replay on next launch without data loss
设置 MUST 提供“重置使用引导”，只影响下次启动，不立即跳转、不修改模型、网络、设备、会话、草稿或任务。原型刷新/重新打开视为启动；完成后不在随后启动重复显示。

#### Scenario: Request a replay
- **WHEN** 用户在设置重置引导并切换页面后重新加载原型
- **THEN** 切页期间不显示引导，加载后从第一页开始，既有配置和历史保留

#### Scenario: Storage cannot be written
- **WHEN** 浏览器阻止保存重置偏好
- **THEN** 页面提示无法保存而不是谎称下次会重置，当前操作仍可使用

### Requirement: Explicit clean review separate from replay
原型 MUST 提供独立的清空演示数据确认入口，展示模型、网络和会话数量及不可恢复说明。仅打开入口或取消 MUST NOT 清空数据。确认后 MUST 清除当前浏览器的演示业务数据并立即进入第一页，刷新后保持空状态；MUST NOT 修改正式应用、主题或其他存储。写入失败 MUST 显示错误，不宣称清空成功。

#### Scenario: Reset six saved models for review
- **WHEN** 用户已有六个型号并确认清空演示数据
- **THEN** 模型、连接、网络、设备、服务、会话和任务均为零，第二页显示添加模型而不是已有配置，刷新后仍为零
