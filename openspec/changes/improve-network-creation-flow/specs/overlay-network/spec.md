## ADDED Requirements

### Requirement: Shared native runtime across desktop and mobile
Windows 与 Android MUST 复用配置转换、网络生命周期、身份、状态和发现逻辑，仅系统能力通过平台适配提供。Windows MUST 接入真实 EasyTier 虚拟网卡；Android MUST 使用应用所属 VpnService 与 EasyTier 原生实例，不得以宿主网络或端口探测替代。网络 running MUST 来自已核验的实例、通道与地址事实。

#### Scenario: Start from either platform
- **WHEN** 用户在 Windows 或 Android 的 Rove 中启动已保存网络并完成必要系统授权
- **THEN** Rove 启动实际 EasyTier 实例，从实际运行状态取得地址和候选成员，并通过该网络通道提供 Rove 协议；拒绝授权或通道失败时明确失败，不伪造 running

#### Scenario: Install Rove without an external VPN application
- **WHEN** 用户在 Windows 安装 Rove 并启动网络
- **THEN** 所需 EasyTier 核心和虚拟网卡组件由 Rove 发行物提供并管理，不要求用户另行安装 EasyTier 或 Mihomo；Android 使用系统 VpnService 而非额外 VPN 应用，必要系统授权保持显式

#### Scenario: Revoke mobile VPN or stop a network
- **WHEN** Android 系统撤销 VPN 或用户停止指定网络
- **THEN** 对应入口停止，设备状态不再在线，配置和独立设备身份保留，其他仍运行网络不得被静默替换

#### Scenario: Verify cross-platform membership
- **WHEN** Windows 和 Android 使用同一网络配置实际入网
- **THEN** 两端报告相同 network_id 与不同 device_id，经 overlay 完成真实 Rove 操作，停止后对应入口失效，物理网不得绕过 overlay 边界

### Requirement: Generated network creation defaults
创建网络请求省略显示名称时，Rove MUST 自动生成符合 `net-[a-z0-9]{2}` 的名称；正式 GUI 打开新建表单时 MUST 预填同格式名称并允许用户在提交前修改。自动名称只是显示与初始 EasyTier 网络名称的默认值，MUST NOT 替代稳定 `network_id`、随机网络密钥或设备身份，重复显示名称也 MUST NOT 合并两个网络。

#### Scenario: Create without naming the network
- **WHEN** 用户打开创建网络表单并不修改自动名称后提交
- **THEN** 保存的显示名称和初始 EasyTier 网络名称使用所显示的 `net-xx` 格式值，同时生成独立的 network_id 和网络密钥

#### Scenario: CLI omits display name
- **WHEN** CLI 或 API 创建请求未提供显示名称
- **THEN** agent 生成 `net-xx` 格式名称并在创建结果中返回，行为不依赖 GUI

### Requirement: Bootstrap discovery remains dynamic and portable
创建和导入网络 MUST 接受四种发现 URI和七种最终连接 URI，并按规范化 URI 去重。网络名片 MUST 分享用户选择的原始 URI，不包含公共目录响应、健康历史或搜索条件；发现预览 MAY 单独显示最近解析结果但 MUST NOT 写成静态替代值。已有合法直接连接 URL 的网络 MUST 保持兼容。

#### Scenario: Create with directory and TXT results
- **WHEN** 用户同时选择公共目录节点和 TXT 解析结果，其中两个结果指向同一规范化连接 URL
- **THEN** 新网络按原始 URI去重保存；直接 URI与发现 URI即使当前解析到同一节点也分别保留，以免破坏后续动态更新
