## ADDED Requirements

### Requirement: Self-contained Base64 network cards
正式 GUI 的网络名片 MUST 把完整可移植 JoinConfig 序列化为 UTF-8 JSON，再编码为无 padding 的 Base64URL；可复制文本和二维码 MUST 使用同一载荷。名片 MUST 只提供「复制配置」分享操作，MUST NOT 显示导出文件或保存二维码按钮。二维码生成 MUST 接受 Base64 文本而非要求 URL，失败 MUST 显示明确错误及重试入口，MUST NOT 静默隐藏。载荷 MUST 保持 JoinConfig 的版本、网络 ID、显示名、EasyTier 密钥和原始初始节点，MUST NOT 添加设备私有数据。Base64 只是一种编码而不是加密，GUI MUST 明确标为敏感配置并限制解码后的最大大小。

#### Scenario: Export one portable card representation
- **WHEN** 用户创建网络或从网络列表查看名片
- **THEN** GUI 显示网络摘要及同一 Base64URL 配置的二维码和可复制文本，不请求公网配置托管服务

#### Scenario: Import Base64 configuration
- **WHEN** 用户粘贴 Base64URL、读取配置文件或扫描同内容二维码并确认导入
- **THEN** GUI 在本地有界解码为 JSON对象，并通过 `import_network` 的手动配置路径交给 agent 完整校验；无效 Base64、非 UTF-8、非对象 JSON或不合法 JoinConfig 均不得产生网络记录

## MODIFIED Requirements

### Requirement: QR and manual onboarding
二维码 MUST 编码与 Base64 文本相同的自包含 JoinConfig。支持相机扫描的平台 MUST 能将扫描结果交给同一 Base64 解码和手动导入流程。正式 GUI MUST NOT 要求 URL、配置托管服务或原始 JSON输入；CLI 的兼容入口不受本 GUI 变更强制移除。

#### Scenario: Scan a network code
- **WHEN** 有扫码能力的客户端扫描有效的 Base64 网络名片二维码
- **THEN** 它在本机解码并执行与粘贴 Base64 配置相同的验证和加入行为

#### Scenario: Work without a configuration server
- **WHEN** 用户拥有网络名片文本、二维码或配置文件
- **THEN** 用户无需连接配置托管服务即可导入，且失败时不保存半成品配置

#### Scenario: Configuration server unavailable
- **WHEN** 配置托管服务不可达但用户拥有 Base64 网络名片文本、二维码或配置文件
- **THEN** GUI仍可在本地解码并提交手动导入，过程不访问托管服务且失败时不保存半成品配置

## REMOVED Requirements

### Requirement: Client-side authenticated encryption
**Reason**: 正式 GUI 改为用户指定的自包含 Base64 网络名片，不再暴露 URL托管分享流程；Base64 明确不作为加密。

**Migration**: GUI 从 agent 读取 JoinConfig 并本地编码；导入时本地解码后使用既有手动导入和 agent 校验。agent/CLI 兼容 API暂不删除。

### Requirement: Simple reusable ciphertext hosting
**Reason**: 自包含 Base64 名片不上传配置，不再需要 GUI 的密文托管、下载或过期语义。

**Migration**: 使用 Base64 文本、配置文件或同内容二维码直接传递；已有旧客户端 URL继续由兼容 API处理，正式 GUI不再生成或接受。
