## Context

## Windows / Android runtime completion

用户确认同时补齐两平台，并要求尽量共享代码。扩展既有 EasyTier 适配器，提供外部 RPC 与应用内实例管理两种后端；共享 JoinConfig → EasyTier NetworkConfig 转换，不复制配置、身份或生命周期业务。Linux 既有外部服务路径保持兼容。Android 在应用内使用固定 revision 的原版 NetworkInstanceManager，并把系统 VPN 的通道交给该实例；Windows 复用同一控制接口并由受限的系统适配层负责 Wintun 和权限。禁止为了启动而提升整个业务 agent 的默认权限或开放无鉴权的物理网管理入口。

平台接口必须提供已验证的实际网卡、地址和路由事实，以及受约束的入站/出站通道。共享观察器只在这些事实均成立后发布 running、设备与服务入口；失败必须关闭旧入口。不能简单删除 Linux cfg 后在其他平台绕过网卡验证。

Windows 权限接入细化：发行物提供独立 `rove-network-helper.exe` 与固定哈希的官方签名 `wintun.dll`，业务 agent 不提升权限。首次网络启动通过正常系统提权启动同账号 helper；无界面监督进程也可预先授权启动同一 helper，再以本机管道和预期服务端 PID 连接。管道使用当前账号专用 DACL、拒绝远程 SMB、单连接会话、双方 PID 校验（受监督预启动模式由账号 DACL 认证首个客户端）；仅暴露有界 Start/List/Info/Stop，不开放命令、文件或原版任意配置接口。断连停止 helper 自有实例；未知完成结果不重放，确认 helper 退出前不假报空实例列表。

helper 在实际 Wintun 地址和路由校验后，以独立动态 WFP 会话为本机 overlay 地址和指定网段添加入站、出站和转发隔离规则。不改动其他应用的转发开关、代理或防火墙规则；helper 存活且通过认证管道报告已安装规则时才允许与已启用转发的宿主共存，其他入口保留原严格检查。停止先移除网络实例，再释放动态规则。WFP 配置使用事务，失败不发布 running；实际 Windows 正向通信及物理网负向测试仍是交付门槛。GUI 仅负责必要的普通用户 agent 进程拉起，网络业务仍使用共享模块。

Windows 防火墙补充：helper 从认证管道客户端 PID 读取实际程序路径，只接受同目录 `rove-agent.exe`；仅为该程序、本机 overlay 地址和远端 overlay 网段添加 TCP 入站许可，正常停止移除自有规则。不删除或覆盖已有阻止规则，也不为物理地址开放 Rove API。上游 EasyTier 自身会创建 helper 程序规则，需在交付记录中明确，不能宣称运行时完全不改防火墙。

共享等待预算补充：原生实例异步初始化采用 30 秒启动期限，尚无运行信息时只报告 pending、不假报 running。Rove 设备发现包含 hello、目标身份校验和设备读取，整个候选轮次限定 8 秒，外层单网络协调限定 15 秒，退出取消等待；避免公共中继下三次合法往返被旧 2 秒总预算持续取消。身份校验和禁止物理网回退保持不变。

私网目录补充：只有已通过安装包默认值或 loopback 校验的字面量私网 HTTP 目录直接访问，不依赖系统代理通配符匹配；公网 HTTPS 目录及初始节点发现保留既有代理策略。此规则在共享目录模块实现，不为 Windows/Android 复制获取流程，也不放宽用户任意私网 HTTP 配置权限。

Android 使用 VpnService 的显式用户授权、前台服务通知与撤销回调；不通过 root、ADB 转发或宿主 TUN 代替应用 VPN。多网络需要由单个系统 VPN 承载不重叠网段并保持实例隔离，不能启动第二网络时静默替换第一网络。需要核验原版 tun fd 接口是否支持通道复用；未经实测不宣称多网络完成。启动授权拒绝、系统撤销、应用退出和停止均须回收通道且保留配置。

Android 接入细化：Kotlin 仅负责系统授权、前台服务和建立 TUN；JNI 转移拥有生命周期的 fd，实际接口名从 TUNGETIFF 读取，不使用 EasyTier 配置中的合成别名。共享 Rust VpnPlan 与数据桥按源/目的地址把一个系统 TUN 分发到各实例独立的报文通道；重配前验证整个计划，停止旧转发任务后发布新计划。只安装实际 overlay 网段的路由，不接管默认路由或 DNS。底层 connector 与 VPN 路由共存、系统权限撤销和真实报文往返仍需原生验收；纯报文通道 fixture 不作为原生成功证据。

验收顺序：共享后端与配置单测、跨平台构建、Windows 和 Android 各自真实启停、相同 network_id/独立 device_id 的互相发现、真实 Rove 操作、停止与物理网负向验证。安装更新保留已有身份与配置；任务 5.4 在全链路通过前保持未完成。

动机参见 [proposal.md](proposal.md)。正式 GUI 目前由 `NetworkForm` 生成完整 EasyTier 配置，名称初始为空；`NetworkPanel.create` 在保存后读取加入配置并设置内联名片数据，但表单、列表和名片共处一个视图，无法保证用户看到创建结果。agent 已有 `reqwest`、URL 校验、目标转发和网络操作分派；固定的 EasyTier 依赖已传递引入 Hickory DNS resolver，但 Rove 尚无公共节点目录或 TXT 解析 API。

原先假定的 `uptime.easytier.cn` 已由全部权威 DNS 返回 NXDOMAIN，不能作为产品依赖。官方 EasyTier Core 当前把 `http`、`https`、`txt`、`srv` 都视为可链式解析的 manual endpoint，并支持最多 16 跳、循环检测及连接时重新解析。ruixuan 提供的是 Uptime Kuma 公共状态页 JSON：节点 URI嵌在 monitor 名称中，heartbeat 只代表其监控探测，既不是官方目录契约也不证明 EasyTier 中继成功。

## 2026-09-22 discovery revision

本节替代下文关于固定 uptime 地址、仅 TXT 解析和“只保存最终 URL”的旧决定；未冲突的名称与名片决定继续有效。

- Rove agent 设置增加可配置、可清除的发现服务根地址。生产只允许 HTTPS，开发可显式使用 loopback HTTP；单次列表请求不能覆盖地址，避免形成通用 HTTP 代理。
- Windows 开发安装包可携带 `rove-defaults.json`，由打包参数写入默认发现服务。为支持当前局域网联调，文件中的完全匹配私网 HTTP 地址可作为未保存设置时的默认值；这不是通用的私网 HTTP 开关，用户覆盖/清除优先，正式包改用 HTTPS 域名。
- Android 没有独立 agent 可执行文件，其进程来自系统 app_process，不能沿用可执行文件旁的 sidecar。Android 包通过构建环境变量 `ROVE_DISCOVERY_SERVICE_URL` 嵌入同等校验的默认值；未提供变量时保持未配置，持久设置与显式 null 始终优先。开发构建可设置 `http://10.1.2.237:8090`，发行时改为 HTTPS 域名。公共发现失败与网络启动失败显示图标及双语说明，不以 UI 修正掩盖平台 runtime 未实现。
- TXT/SRV 优先读取系统 DNS 配置；Android 等平台无法读取时，使用固定 EasyTier revision 的后备地址 `223.5.5.5:53`、`180.184.1.1:53`。后备服务器会收到查询域名但不接收网络密钥；不因一次查询失败而切换服务器策略，仍受原有 3 秒外层超时约束。
- DNS 单次等待为 900ms、最多两次尝试，使内部重试能在既有 3 秒总期限内发生。HTTP 发现分别报告请求超时、连接失败、非成功状态与其他请求失败，不再全部伪装为公共目录不可用；返回固定分类文案和 HTTP 状态码，不返回原始异常中的 URL、查询、头或正文。TLS 证书验证保持开启，不降级 HTTPS 来绕过错误。
- `/home/git-ssd/rove-discovery` 是独立项目，默认读取 ruixuan 的 status-page 与 heartbeat JSON，只抽取“社区公共节点”分组中未打码的受支持 URI。名称中的作者、地区、“可中转”和 heartbeat 均标为未验证社区元数据。
- 独立服务提供 `/healthz`、`/v1/nodes` 和 `/v1/discover`。它设置 5 秒、512 KiB、200 项限制并维护短期快照；上游失败时只能在有效缓存内返回 `stale=true`，无快照返回 503。TLS由部署反向代理终止。
- agent 预览解析完整支持 HTTP(S)、TXT、SRV，发送 EasyTier 的 `X-Network-Name`，解释 2xx 逐行正文和 3xx Location，查询 TXT/SRV，记录发现链并限制 16 跳和循环。
- JoinConfig 接受发现协议 `http`、`https`、`txt`、`srv` 与最终协议 `tcp`、`udp`、`ws`、`wss`、`wg`、`quic`、`faketcp`。保存和分享原始发现 URI；最终预览只用于解释，不能静态替代原始 URI。
- GUI 对发现 URI 执行“测试连通”时复用目标 agent 的解析操作取得本次最终 URI，再把该 URI交给既有的有界传输探测；界面同时显示解析链和最终 URI，但不修改原始表单值。这样 `https://`/`txt://` 不会被错误当作最终 connector，同时连接/重连仍可由 EasyTier Core 动态解析原始地址。
- GUI 不再提供单独的“发现地址/解析”输入区；HTTP(S)/TXT/SRV 与最终 connector 统一填写在初始节点列表中，发现 URI只在需要测试时自动解析，减少同一节点在两个区域间复制和确认的步骤。
- 目录服务和 EasyTier发现协议是两层接口：目录供人选择多个候选，发现 URI供 Core 在连接/重连时取得一个最终节点。Rove 不把任何社区目录设为不可替换的协议依赖。

## 2026-09-23 prototype navigation and Base64 card revision

- 正式网络标签只显示列表，创建和加入只由右上角全局「＋」菜单打开独立弹窗。列表查看名片、编辑也使用弹窗；同一时间只打开一个操作弹窗，创建成功后弹窗转为名片，关闭后回到列表。引导流程继续使用自己的弹窗。
- GUI 名片不再调用 `create_network_share`，也不显示托管 URL或到期时间。它从 agent 读取经过校验的 JoinConfig JSON，以 UTF-8 JSON → RFC 4648 Base64URL（无 padding）形成自包含配置；文本和二维码使用完全相同的载荷，名片只提供「复制配置」一个分享操作，不提供导出文件或保存二维码按钮。二维码使用接受文本载荷的原生方法，不能复用仅校验 URL 的旧方法；生成失败必须明确显示错误和重试。
- GUI 加入页只接受该 Base64URL 文本、文本配置文件或同内容二维码，在 WebView 本地有界解码 JSON后调用 `import_network {source:"manual",config}`。原始 JSON和 HTTP(S) 分享 URL不再是 GUI 输入，避免多种导入方式混杂。
- agent 现有 URL分享 API 暂时保留为兼容接口，CLI 与旧客户端不在本次被强制迁移；正式 GUI不调用它。后续若确认全产品删除托管服务，再以独立契约变更移除 OpenAPI 和服务端实现。

## Goals / Non-Goals

网络界面追加整理：列表去掉重复标题及静态长说明，刷新移至网络页头图标。网络操作共用左上角 chevron 回退栏；桌面为居中弹窗，宽度不超过 640px 或紧凑横屏为全屏展示。名片复制操作固定在底部，内容区域独立滚动，兼容安全区域；引导中的网络弹窗复用相同结构。

设备层级追加：网络列表点击条目进入 `#/networks/<network_id>`，详情使用 `get_network` 取得当前网络状态与地址，并以相同目标 agent 和 network_id 查询设备。名片、启停、编辑、删除及模型同步位于网络详情；原全局设备浏览入口移除，设置中的手动执行目标诊断不作为跨网络设备列表。详情请求以代次及 network_id 丢弃迟到响应，设备行继续校验网络归属；远端 agent 的设备列表只展示，不隐式实现二次转发选择。

**Goals:**

- 让 agent 成为外部目录和 DNS 查询的唯一执行方，并使本机、远端目标、Windows 与 Android 使用同一契约。
- 把公共目录结果与 TXT 发现结果转换为最小、受限、可测试的候选模型。
- 由 agent 提供缺省名称的最终保障，GUI 同时预填相同格式以便用户看见和编辑。
- 创建成功后进入明确的名片状态，并始终从已保存配置重新读取名片。

**Non-Goals:**

- 不运营、镜像或担保 EasyTier 公共节点，不自动选择或静默加入任意公共节点。
- 不通过目录健康状态或端口探测宣称 overlay 入网成功。
- 不在本次支持任意 HTTP/SRV discovery URL、持续后台刷新、测速排名或公共节点公钥信任库。
- 不改变创建后网络仍为 `stopped` 的生命周期语义，也不把名片显示等同于启动网络。

## Decisions

### 1. 增加两个只读 agent 操作

OpenAPI 增加：

- `list_public_bootstrap_nodes`：固定请求 EasyTier uptime 目录，可带受限分页参数，返回 `BootstrapNodePage`。
- `resolve_bootstrap_discovery`：请求体只接受一个 `txt://` URL，返回原始地址、最终连接 URL 和协议。

HTTP 映射使用 `/v1/networks/bootstrap-nodes` 和 `/v1/networks/bootstrap-nodes/resolve`，本机 socket、远端 overlay 转发和 SDK 继续按 operationId 复用。固定目录地址不接受调用方覆盖，避免把该操作变成通用 HTTP 代理；TXT 操作只做 DNS TXT 查询，禁止 `http`、`file`、凭据、路径、查询参数、fragment、IP 字面量和二次 `txt/srv/http/https` 发现。

备选方案是在 WebView 直接请求 uptime 和 DNS-over-HTTPS；这会引入 CORS、平台 DNS、远端目标归属和 Android 网络栈差异，因此不采用。

### 2. 最小化并过滤 uptime 数据

agent 请求固定 URL：

`https://uptime.easytier.cn/api/nodes?page=1&per_page=200&is_active=true`

使用 rustls 系统信任、5 秒总超时、512 KiB 流式正文上限，拒绝非 2xx 与契约不符响应。只映射 `id`、`name`、`address`、`protocol`、`is_active`、`allow_relay`、`current_connections`、`max_connections`、`current_health_status`、`last_response_time` 和 `health_percentage_24h`；过滤 `!is_active`、`!allow_relay`、无效地址和不支持协议，最多返回 200 项并按规范化 URL 去重。明确丢弃目录里的 `network_name`、`network_secret`、联系方式、描述 HTML 和其他未知字段。

候选顺序先按当前健康状态、容量是否未满、响应时间排序，再保持 ID 稳定排序；GUI不自动勾选。备选方案是使用 uptime 的 `/node/{id}` HTTP discovery 地址，这仍需连接时再次依赖目录且不能满足用户查看“最终节点”的要求，因此直接采用已校验的 `address`。

### 3. TXT 解析复现固定 EasyTier 语义但返回可解释结果

将 `hickory-resolver 0.25.2` 作为 agent 的直接依赖，复用与固定 EasyTier 相同的系统 DNS + 后备 resolver 策略。查询限制为 3 秒、单一 TXT 名称、最多 16 条记录和 8 KiB 拼接文本；取 resolver 返回的第一条 TXT RR，正确拼接该 RR 的多个 character-string 后按 ASCII 空白拆分 URL。合法候选随机选择前先全部规范化和去重，然后使用随机选择，以匹配 EasyTier 的多候选负载分散语义；API同时返回合法候选列表和本次 `resolved_url`，GUI将后者加入表单。

最终协议白名单与当前 EasyTier JoinConfig 保持同源：`tcp`、`udp`、`ws`、`wss`、`quic`、`wg`；必须有主机和 1–65535 端口，不接受用户信息。`txt://` 本身只允许域名且不递归。发现结果表示“已解析的连接器”，不是连通性证明；用户如需 TCP/WS/WSS 探测仍使用既有 `test_network_peer`。

备选方案是把 `txt://` 原样存入 `bootstrap_peers` 交给 EasyTier 在每次连接时动态解析。该方案保留动态切换能力，但当前 Rove 校验拒绝 discovery scheme，也不能让用户看到最终节点；本次选择解析后保存最终 URL。后续如要保留动态发现，应单独扩展 JoinConfig 并处理重连时地址漂移。

### 4. agent 对自动名称负责，GUI只负责预览

`NetworkCreate.display_name` 改为可选。agent 在缺失或只含空白时用系统随机源从小写字母和数字生成两字符后缀，形成 `net-xx`；同一值用于 `display_name` 和省略完整 config 时的初始 `network_name`。若 GUI 提交完整 config，则 `NetworkForm` 用相同生成器预填名称并把该值写入两处。名称不是身份键，因此不为本机重名增加失败；稳定身份仍是 UUID `network_id`，成员认证仍依赖随机 secret。

CLI `network create` 的位置参数改为可选，并增加 `public-nodes`、`resolve-peer <txt-url>` 子命令及结构化输出。备选方案只在 GUI 随机生成会让 CLI/API 与失败重试产生不一致，因此不采用。

### 5. 名片使用显式 panel 状态

`NetworkPanel` 引入 `list | create | join | card | edit` 中可组合的明确视图状态；创建成功后保存返回的 network_id，刷新列表，再调用 `get_network_join_config`。只有两步都成功才切到 `card`，名片视图替换创建表单并将标题聚焦/滚入视口。名片提供“返回网络列表/继续引导”动作；父级 Introduction Dialog 不会在收到创建成功时立刻关闭。

若网络已经保存但读取名片失败，错误必须说明“网络已创建，名片读取失败”，并提供按 network_id 重试，不能重新提交创建造成第二个网络。为此 create 流程将保存和取名片错误分阶段记录，而不是用单个会回滚 UI 的 `action` 包裹。

备选方案继续在表单下方渲染名片，无法满足窄屏可见性和明确跳转语义，因此不采用。

### 6. 测试不访问生产公共服务

agent 的目录客户端和 TXT resolver 使用可注入边界：单元测试提供本地 HTTP fixture 与 DNS lookup stub，覆盖超时、超限、恶意字段、过滤、TXT 多字符串、多候选、递归拒绝和去重。浏览器测试 mock 新 operationId，验证桌面/移动、引导/普通页面、成功跳名片、保存后取卡失败和创建失败保留输入。另做一次人工只读冒烟记录实际 uptime 契约，但不把外网在线作为 CI 通过条件。

## Risks / Trade-offs

- [两字符后缀容易重名] → 名称明确仅作可编辑显示值，network_id 才是身份；列表继续同时持有稳定 ID。
- [公共目录契约或可用性变化] → 使用容错反序列化、严格输出映射和明确失败；手工创建完全保留。
- [公共节点或 DNS 被恶意控制] → 不发送网络密钥到发现服务，限制协议/大小/时间，用户显式选择；名片安全提示继续说明公共初始节点并非身份信任。
- [解析后保存会失去 TXT 动态切换] → UI保留来源用于当前会话说明，但共享配置只保存此次最终 URL；未来动态发现作为独立兼容变更。
- [“在线”被理解为可成功组网] → 文案标明来源和检查时间，继续把实际启动及 EasyTier 状态作为唯一组网证据。
- [Android DNS 与 VPN 路由相互作用] → 查询通过目标 agent 且在网络创建前执行；平台验收覆盖 VPN 未启动和已有 overlay 运行两种状态，不以桌面结果外推。

## Migration Plan

1. 先扩展 OpenAPI、协议 schema、SDK 和 agent 操作；旧客户端不调用新操作，现有网络数据无需迁移。
2. 更新 CLI 和 GUI；`display_name` 从 required 改 optional 是向后兼容放宽，旧客户端继续传值。
3. 发布时先验证本地 fixtures，再在 Windows 与 Android 分别只读获取目录、解析受控 TXT，并验证创建后名片；实际 Windows–Android 组网仍按 overlay 规格单独验收。
4. 回滚 GUI时新 agent 操作无人调用且无持久副作用；回滚 agent 前须同时回滚调用新 operationId 的客户端。已保存配置只含旧版本已支持的最终连接 URL，因此可继续使用。
