# overlay-network Specification

## Purpose

定义个人网络的创建、保存、加入与设备发现，使 Rove 能在多个网络中找到稳定的对等设备，并将网络实例、密钥及路由变化交给网络层处理而不造成业务身份漂移。

## Requirements

### Requirement: Multiple personal networks
Rove MUST 支持个人保存、管理和分享多个网络；每台设备 MAY 同时加入多个实际 IPv4 网段不重叠的网络，网段相互包含也 MUST 拒绝。保存不表示加入，启动请求加入，停止只退出指定本机网络。每个网络 MUST 有稳定 network_id 和独立运行/错误状态。自动模式 MUST 由原版 EasyTier DHCP 分配，MUST NOT 同时要求指定地址池；手动模式 MUST 关闭 DHCP，使用指定网段和本机独立 IP。未运行的配置 MAY 使用相同网段。

#### Scenario: Add another network
- **WHEN** 设备已保存一个网络且用户导入另一个网络
- **THEN** 两份网络配置分别保留，导入不启动第二个网络或中断当前连接

#### Scenario: Start another nonoverlapping network
- **WHEN** A 已运行，用户启动实际网段与 A 不重叠的 B
- **THEN** B 使用独立接口、监听和发现结果，A 的入口和服务继续运行

#### Scenario: Concurrent starts
- **WHEN** 两个客户端同时请求启动不同网络
- **THEN** 启动串行检查已知网段；其他自动网络网段尚未确定时返回 network_address_pending，静态冲突返回 network_subnet_conflict，不创建冲突实例

#### Scenario: DHCP discovers an overlapping subnet
- **WHEN** 新网络由 EasyTier 获得的网段与已加入网络重叠
- **THEN** 新网络不得开放执行或服务入口，其底层实例停止并保留明确冲突错误；不修改已加入网络的配置或自动选择另一地址池

#### Scenario: Static address is not portable
- **WHEN** 另一设备导入手动网络名片
- **THEN** 导入不复制本机固定 IP；未填写接收设备的有效主机 IP 时启动返回 local_address_required

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

### Requirement: Honest bootstrap transport tests
GUI 与 CLI MUST 可逐条调用 test_network_peer；探测 MUST 有超时和并发上限，不保存或修改网络，不发送入网凭据。TCP/WS/WSS 的 TCP 连接成功 MUST NOT 被表示为 EasyTier 身份认证、TLS 或加入成功；未实现协议探测 MUST 返回 unsupported。

#### Scenario: Datagram bootstrap test
- **WHEN** 用户测试 UDP/QUIC/WG 初始节点
- **THEN** 当前版本明确显示不支持探测，不以发送数据包成功冒充远端可达
