## Purpose

定义图形界面与命令行作为同一应用的两种交互方式，确保没有图形环境的用户仍可完成网络、设备、AI 和服务操作，并使多设备漫游时目标与执行结果清晰可见。

## ADDED Requirements

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
GUI 与 CLI MUST 显示当前网络和目标设备；提交后 MUST 展示作业 ID、排队或执行状态以及可查询输出。设备离线、协议不兼容和平台不支持 MUST 有可区分提示。

#### Scenario: Remote device becomes unavailable
- **WHEN** 用户尝试向离线目标发送消息
- **THEN** 界面报告目标不可达，不悄悄改为在当前设备执行

### Requirement: Clients attach to existing runtime
重复打开 GUI 或 CLI MUST 连接已有的本机运行实例；关闭界面 MUST NOT 隐式停止设备网络、服务代理或已接受作业。启动失败 MUST 提供可理解的处理信息。

#### Scenario: Switch from GUI to CLI
- **WHEN** 用户关闭 GUI 并通过 CLI 查看之前启动的作业
- **THEN** CLI 展示同一个作业的当前状态和输出
