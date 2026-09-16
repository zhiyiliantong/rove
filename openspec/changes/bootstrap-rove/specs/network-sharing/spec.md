## Purpose

定义简单一致的网络加入配置分享流程，让手机和平板通过扫码、电脑与终端通过 URL 或手动录入加入网络，同时确保公网配置托管服务只获得密文且不混入设备私有数据。

## ADDED Requirements

### Requirement: Portable network-only join configuration
加入配置 MUST 包含格式版本、稳定网络 ID、显示名和可移植的 EasyTier 入网配置；MUST NOT 包含分享者的设备 ID、本机实例 ID、固定设备地址、模型密钥、会话或服务凭据。

#### Scenario: Export from configured device
- **WHEN** 已配置模型和服务的设备分享网络
- **THEN** 分享载荷只包含入网所需配置及网络元数据，接收端生成或复用自身本地身份

### Requirement: Client-side authenticated encryption
URL 分享 MUST 在本机用新生成的随机密钥进行认证加密后上传；解密密钥 MUST 位于 URL fragment，密文服务 MUST NOT 接收该密钥或配置明文。客户端 MUST 在完整验证与解密成功后才保存网络配置。

#### Scenario: Valid URL import
- **WHEN** 用户导入有效分享 URL
- **THEN** 客户端下载密文、在本机解密和验证，随后加入对应网络

#### Scenario: Wrong key or tampered ciphertext
- **WHEN** 密钥错误、密文被篡改或载荷格式不受支持
- **THEN** 导入失败并给出明确原因，本机网络配置保持原状

### Requirement: Simple reusable ciphertext hosting
密文托管服务 MUST 提供无需用户账号的上传和下载，分享默认有效期为上传成功后七天；有效期内 MUST 允许重复下载，到期后 MUST 拒绝下载并安排清理。

#### Scenario: Repeated use before expiration
- **WHEN** 两台设备先后使用同一个未过期 URL
- **THEN** 两台设备均可下载并导入配置，第一次使用不消耗链接

#### Scenario: Expiration after joining
- **WHEN** 分享链接到期且已有设备成功加入
- **THEN** 后续下载报告已过期或不可用，已有设备网络成员关系不被改变

### Requirement: QR and manual onboarding
二维码 MUST 编码与 URL 导入相同的分享链接；支持相机扫描的平台 MUST 能将扫描结果交给同一导入流程。GUI 与 CLI MUST 提供手动入网配置入口，手动录入 MUST 不依赖密文托管服务。

#### Scenario: Scan a network code
- **WHEN** 有扫码能力的客户端扫描有效网络二维码
- **THEN** 它执行与导入对应 URL 相同的验证和加入行为

#### Scenario: Configuration server unavailable
- **WHEN** 分享托管不可达但用户拥有入网参数
- **THEN** 用户仍可通过手动配置加入，URL 流程报告下载失败且不保存半成品配置
