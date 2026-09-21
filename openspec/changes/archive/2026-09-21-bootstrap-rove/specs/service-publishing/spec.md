## Purpose

定义将本机已部署服务发布到 Rove 网络的行为，使网络成员获取可用地址和应用认证信息，并在设备重启、网络地址变化或取消发布时看到与实际代理状态一致的服务目录。

## ADDED Requirements

### Requirement: Persistent TCP publication
发布服务 MUST 创建从指定网络的本设备地址与分配端口到本机回环目标端口的 TCP 代理，并保存稳定 `service_id`、所属网络、所属设备、名称和目标。系统 MUST 在代理成功后才报告可用。

#### Scenario: Publish a localhost service
- **WHEN** 用户或 AI 发布本机可访问的音乐服务器端口
- **THEN** 网络成员获得服务记录及代理入口，可使用应用协议通过该入口访问本机服务

#### Scenario: Port cannot be bound
- **WHEN** 发布代理无法获得监听端口
- **THEN** 发布返回明确错误或选择可用端口，不登记一个虚假的可用入口

### Requirement: Distributed service directory and application access information
服务记录 MUST 保存在提供服务的设备，可信网络成员 MUST 能查询并查看访问地址与 `access_info`。HTTP/HTTPS 服务 MUST 提供浏览器可用的对应协议 URL；应用认证 MUST 由应用自身处理。

#### Scenario: Open a service from another device
- **WHEN** 用户在同一网络的另一设备查看在线服务
- **THEN** 界面提供当前地址及已登记的应用账号或其他认证说明，浏览器或客户端直接连接代理入口

### Requirement: Recover publication after restart or address change
agent 重启或网络恢复时 MUST 恢复保存的发布配置；地址变更 MUST 更新同一服务的入口。不可用的网络、端口或目标 MUST 被报告为不可用或未知，不继续声称服务健康。

#### Scenario: Restart hosting device
- **WHEN** 发布过服务的设备重启并恢复网络
- **THEN** agent 重新建立代理并保留服务 ID，客户端查询到当前入口或明确恢复错误

### Requirement: Unpublish without uninstalling
用户 MUST 能取消发布；取消 MUST 关闭代理并从活动目录移除该入口，MUST NOT 隐式停止、删除或卸载目标应用。

#### Scenario: Unpublish music server
- **WHEN** 用户取消音乐服务器的发布
- **THEN** Rove 代理入口停止接受新连接，目标应用及其音乐数据保持原状

### Requirement: Explicit service configuration update
用户 MUST 能显式更新服务名称、目标、监听端口和认证信息，同时保留服务身份与网络归属；更新失败 MUST 保留旧发布或明确报告冲突，不能把失败更新当作成功。

#### Scenario: Replacement port unavailable
- **WHEN** 用户更新为被占用的监听端口
- **THEN** 更新返回错误，旧发布定义与仍可用的入口不被悄悄丢弃

### Requirement: Dialogue management shares the API implementation
agent MUST 提供契约定义的 `rove_api` 通用工具，复用服务查询、发布、更新和取消发布的 API 校验及实现；MUST NOT 创建与 CLI 不一致的独立服务目录。工具 MUST 返回真实结果或错误，不支持任意 URL 或隐式远端切换。GUI 服务页 MUST 仅提供查询和访问，不增加服务管理表单。

#### Scenario: Manage a service through a conversation
- **WHEN** 模型调用 `rove_api` 更新名称并取消服务发布
- **THEN** CLI 和其他客户端查询到同一目录变化，代理入口关闭而目标应用继续运行，工具结果记录在原会话

### Requirement: Explain maintenance impact before confirmation
GUI 的管理入口 MUST 新建检查草稿且不自动提交，模型系统指令 MUST 要求先说明破坏性或中断操作的目标、影响及恢复限制，再等待用户后续确认。该约定 MUST NOT 被描述为额外 Rove 权限系统或任意命令的硬安全保证。在线数据目录迁移未实现时 MUST 明示，不指示模型搬动运行中数据库。

#### Scenario: Open maintenance from the service directory
- **WHEN** 用户点击服务页的对话管理入口
- **THEN** 新会话关联当前选择的执行设备，已有会话不被改写，草稿要求先检查再确认，不自动发布、修改或取消任何服务
