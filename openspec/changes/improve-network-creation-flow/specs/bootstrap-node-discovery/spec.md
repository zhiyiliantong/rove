## Purpose

定义 Rove 如何通过可配置目录和 EasyTier 原生 HTTP(S)/TXT/SRV 发现地址取得初始连接节点，同时限制外部输入、准确表达健康度，并在发现服务不可用时保留手工配置能力。

## ADDED Requirements

### Requirement: Public bootstrap directory retrieval
Rove MUST 由用户当前选择的目标 agent 通过设置中的发现服务 HTTPS 根地址获取公共节点；该地址 MUST 可配置、可清除，发行版 MAY 提供缺省 HTTPS 地址。开发测试安装包 MAY 在打包时显式预置一个私网 IP 的 HTTP 根地址，但 agent MUST 只对安装包中完全相同的该地址放宽 HTTPS 要求，任意其他远程 HTTP 地址仍须拒绝；用户显式覆盖或清除后 MUST 优先使用持久设置。结果 MUST 包含显示名称、来源、原始发现或连接 URI、健康状态与目录提供的有限负载信息；MUST 去重并设置请求超时、响应大小和最大条数限制。目录数据 MUST 作为不可信外部输入校验，MUST NOT 接收或使用网络名称、网络密钥、脚本或任意 HTML。未配置服务时 MUST 返回明确的未配置错误而不尝试任意第三方地址。

#### Scenario: Fetch eligible public nodes
- **WHEN** 用户在创建网络页面请求获取公共初始节点，目录返回已审核的在线中继节点以及不符合条件的节点
- **THEN** Rove 只展示符合条件且连接 URL 使用受支持 EasyTier 协议的节点，并允许用户显式选入表单

#### Scenario: Directory is unavailable
- **WHEN** HTTPS 请求超时、证书无效、响应过大或 JSON 不符合契约
- **THEN** Rove 显示公共节点获取失败，不添加部分或未经校验的节点，并继续允许用户手工输入和创建网络

#### Scenario: Development package supplies a private HTTP default
- **WHEN** 开发安装包在打包时把 `http://10.1.2.237:8090` 写为默认发现服务，且用户尚未保存该设置
- **THEN** agent 使用该地址获取目录，设置页显示相同默认值；用户清除后保持未配置，且其他私网 HTTP 地址不能借此获得隐式许可

#### Scenario: Android embeds the package default
- **WHEN** Android 构建显式设置 `ROVE_DISCOVERY_SERVICE_URL`，且用户尚未保存该设置
- **THEN** 嵌入 agent 使用经过同等地址校验的构建默认值，而不是读取系统 app_process 旁的文件；用户保存的地址或显式清除优先，未提供构建值时仍明确报告未配置

### Requirement: EasyTier discovery protocols are supported
Rove MUST 接受 `http://`、`https://`、`txt://`、`srv://` 四种 EasyTier 发现地址，并由目标 agent 预览解析。HTTP(S) MUST 发送 GET、`User-Agent` 和 `X-Network-Name`，并按 EasyTier 语义解释 2xx 逐行正文或 3xx `Location`；TXT MUST 解析空格分隔候选；SRV MUST 查询 `_easytier._<protocol>.<domain>`。解析链 MUST 检测循环并限制为最多 16 跳。成功结果 MUST 同时返回原始 URI、发现链、合法候选和本次最终 URI；保存网络时 MUST 保留原始发现 URI，预览结果不得替代它。

#### Scenario: TXT record resolves a connector
- **WHEN** 用户输入 `txt://et2.lctn.cc.cd` 且其 TXT 记录包含合法的 `udp://host:port`
- **THEN** Rove 展示发现来源及最终 UDP 连接 URL，选择后保存去重后的原始 `txt://` URI并把最终 URL作为预览信息

#### Scenario: TXT record has no supported connector
- **WHEN** TXT 查询失败、超时、答案为空，或发现链只包含未知协议及无效地址
- **THEN** Rove 将该项标为无法解析且不加入网络配置，其他已输入或已选择节点保持不变

#### Scenario: HTTP discovery chains to DNS
- **WHEN** HTTPS 发现响应返回 `txt://bootstrap.example.com`，TXT 最终返回合法 TCP 节点
- **THEN** Rove 预览完整 HTTPS → TXT → TCP 链，保存 HTTPS 原始 URI，并拒绝循环或超过 16 跳的链

#### Scenario: Transient DNS packet loss within the lookup deadline
- **WHEN** DNS 首次查询丢包但重试可以得到有效答案
- **THEN** 内部单次等待与有限重试 MUST 能在既有 3 秒外层时限内执行，MUST NOT 使用超过总时限的单次默认等待导致重试永远不可达

#### Scenario: Discovery failure identifies the failing stage
- **WHEN** HTTP(S) 请求超时、连接失败、返回非成功状态，或后续 DNS 查询超时
- **THEN** Rove MUST 分别提供对应阶段的稳定错误代码与双语说明，MUST NOT 将它们全部描述为公共目录未配置或最终节点连接失败；错误不得暴露 URL 查询、请求头或密钥，TLS 校验 MUST 保持开启

#### Scenario: Test connectivity through a discovery URI
- **WHEN** 用户对已填入初始节点的 HTTP(S)、TXT 或 SRV 发现 URI 执行“测试连通”
- **THEN** 所选目标 agent MUST 先解析完整发现链，再只对本次最终连接 URI 应用既有的有界传输探测；GUI MUST 展示发现链和实际被探测的最终 URI，网络配置仍保留原始发现 URI，且 UDP、WG、QUIC 或 FakeTCP 最终节点 MUST 显示为不支持探测而不是无效发现地址

### Requirement: Final EasyTier connectors are complete
Rove MUST 接受并规范化 `tcp`、`udp`、`ws`、`wss`、`wg`、`quic`、`faketcp` 七种最终节点协议，要求合法主机和协议所需的非零端口，拒绝凭据、fragment、未知协议和无主机地址。直接节点与发现 URI MUST 使用同一去重和信任边界，但发现 URI不得被当成最终传输探测。

#### Scenario: Save every supported connector family
- **WHEN** 用户分别输入七种受支持的合法初始节点 URI
- **THEN** Rove 均可保存和分享，且不会把 UDP、WG、QUIC 或 FakeTCP 的普通端口探测描述为 EasyTier 连接成功

### Requirement: Independent discovery service exposes a bounded contract
独立 `rove-discovery` 服务 MUST 提供健康检查、结构化节点列表和 EasyTier 兼容的 HTTP 发现端点；MUST 从可配置上游读取 ruixuan 的公开状态页 JSON与 heartbeat JSON，按 monitor id 合并状态，只接受社区公共节点分组中未打码且协议受支持的 URI。服务 MUST 明确标注社区/人工元数据，MUST NOT 把 Uptime Kuma `type`、名称中的“可中转”或 heartbeat 成功提升为已验证的 EasyTier 中继保证；MUST 有超时、正文/条目上限和缓存降级。

#### Scenario: Community upstream is available
- **WHEN** ruixuan 返回公开 monitor 列表和 heartbeat 数据
- **THEN** 服务输出去重后的结构化节点，保留来源、监控时间和有限健康状态，并可用纯文本逐行候选响应 EasyTier HTTP 发现请求

#### Scenario: Community upstream fails
- **WHEN** 上游超时、响应过大或结构变化
- **THEN** 服务在缓存有效时返回带 stale 标记的最近成功快照，否则返回明确 503，且不伪造空列表为成功

### Requirement: Discovery health is represented honestly
目录的在线状态或连接探测 MUST 仅表示目录最近的健康检查或特定传输的有限可达性；Rove MUST NOT 将其描述为 EasyTier 身份认证、加入指定 overlay 网络或端到端 Rove 组网成功。公共节点选择 MUST 由用户确认，获取结果本身 MUST NOT 自动保存、创建或启动网络。

#### Scenario: Healthy public node is selected
- **WHEN** 用户选择目录中显示在线的公共节点
- **THEN** 页面将其加入待创建配置并说明状态来源，直到网络实际启动前不显示已加入或已组网

### Requirement: Public-node discovery has client parity
GUI MUST 提供公共节点获取与 TXT 解析入口；CLI MUST 提供等价的结构化获取和解析操作。两者 MUST 通过同一 agent 契约执行并返回稳定错误代码，使远端目标、Windows 与 Android 不依赖 GUI WebView 的跨域或 DNS 能力。

#### Scenario: Resolve from a remote target
- **WHEN** GUI 当前操作另一台 Rove 设备并请求解析 TXT 发现地址
- **THEN** DNS 查询由该目标设备的 agent 发起，结果明确归属目标设备，不静默改由 GUI 所在设备解析
