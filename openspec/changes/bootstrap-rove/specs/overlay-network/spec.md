## Purpose

定义个人网络的创建、保存、加入与设备发现，使 Rove 能在多个网络中找到稳定的对等设备，并将网络实例、密钥及路由变化交给网络层处理而不造成业务身份漂移。

## ADDED Requirements

### Requirement: Multiple personal networks
Rove MUST 支持个人保存、管理和分享多个网络，但每台设备同时 MUST 最多加入一个网络。保存配置不表示已加入；启动表示请求加入，停止表示退出本机网络。每个网络 MUST 有稳定 `network_id`，网络列表 MUST 显示各自运行或错误状态。地址分配 MUST 交由原版 EasyTier DHCP，Rove MUST NOT 要求不同个人网络使用不同地址段。

#### Scenario: Add another network
- **WHEN** 设备已保存一个网络且用户导入另一个网络
- **THEN** 两份网络配置分别保留，导入不启动第二个网络或中断当前连接

#### Scenario: Switch the joined network
- **WHEN** 设备尚未退出网络 A，用户请求启动网络 B
- **THEN** 返回 409 network_already_joined，提示先停止 A，不启动 B；A 停止成功后才能启动 B，设备身份与其他配置保持不变

#### Scenario: Concurrent starts
- **WHEN** 两个客户端同时请求启动不同网络
- **THEN** 启动流程串行核对本机加入状态，至多一个网络取得加入资格；启动失败但底层实例尚未确认清理时仍不允许另一个网络启动

#### Scenario: One network fails
- **WHEN** 一个网络实例启动失败
- **THEN** 该网络报告原因，其他网络的配置不会被删除或替换

### Requirement: Network identity is separate from local instance identity
分享导入 MUST 保留网络 `network_id`，并使用导入设备自身的本地实例身份与设备身份。同一网络的重复导入 MUST NOT 静默创建重复逻辑网络或覆盖冲突配置。

#### Scenario: Two devices join one network
- **WHEN** 两台设备导入同一加入配置
- **THEN** 两者报告相同 `network_id` 和各自的 `device_id`，本地实例标识不作为跨设备网络身份

#### Scenario: Repeated import
- **WHEN** 用户再次导入已存在且内容相同的网络配置
- **THEN** 系统返回已有网络；同 ID 但配置不同时报告冲突并要求显式更新

### Requirement: Fully trusted network membership
成功加入网络的成员 MUST 能调用该网络内 Rove 设备提供的操作和服务，不需要额外 Rove 账号、角色、邀请审批或授权票据。底层入网凭据和网络加密 MUST 由 EasyTier 处理。

#### Scenario: Shared network member connects
- **WHEN** 另一用户通过有效网络配置加入并连接到可用的 Rove 设备
- **THEN** 对方可以查询、提交操作和使用服务，不出现额外 Rove 权限申请

### Requirement: Discover Rove peers with current routes
设备发现 MUST 从网络中的候选节点识别 Rove 设备，以稳定身份展示，并更新当前路由地址。普通网络节点、离线设备和协议不兼容设备 MUST 有可区分结果。

#### Scenario: Peer address changes
- **WHEN** 已知设备以新的网络地址重新可达
- **THEN** 发现结果更新同一设备的地址，不把它当作全新设备

#### Scenario: Non-Rove node
- **WHEN** 候选网络节点没有可用的 Rove 协议接口
- **THEN** 它不被报告为可执行 Rove 作业的在线设备

### Requirement: Network lifecycle preserves unrelated state
用户 MUST 能启动、停止和删除本机保存的网络；停止或删除本机网络 MUST NOT 被描述为撤销其他成员的资格，且 MUST NOT 自动卸载通过该网络部署的应用。

#### Scenario: Stop a network
- **WHEN** 用户停止本机某网络
- **THEN** 该网络入口不再提供服务，网络配置仍可重新启动，已部署应用不被自动删除
