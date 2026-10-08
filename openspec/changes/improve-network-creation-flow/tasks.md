## 1. 契约与公共模型

- [x] 1.1 扩展 `api/rove-agent.openapi.json`：增加公共节点列表、TXT 发现解析的路径与 schema，把 `NetworkCreate.display_name` 改为可选，并补充成功、过滤和稳定错误示例。
- [x] 1.2 扩展统一 URI 校验为四种发现协议和七种最终协议，供 JoinConfig、公共目录和发现预览复用，并覆盖主机、端口、用户信息、重复 URI、发现链循环与跳数测试。
- [x] 1.3 更新 SDK/协议契约测试和 API 操作分派清单，确认两个只读操作可经本机 socket、HTTP 和远端目标调用且不会修改网络状态。

## 2. Agent 公共节点发现

- [x] 2.1 将目录客户端改为读取 agent 设置中的发现服务根地址，删除 `uptime.easytier.cn` 硬编码，加入未配置、HTTPS/loopback HTTP 校验、缓存来源和边界测试。
- [x] 2.2 实现 HTTP(S)、TXT、SRV 四种受限预览解析和最多 16 跳链式解析，保留原始 URI并覆盖正文、重定向、DNS 多候选、循环、超时与超限。
- [x] 2.3 将通用目录与发现操作接入 agent 网络分派、HTTP 路由及远端转发，返回稳定的未配置、上游不可用、DNS 失败、无候选、响应超限和输入无效错误。
- [x] 2.4 扩展可注入 fixture，验证目录和发现服务不会收到网络密钥、恶意字段被丢弃、所有协议去重且解析不产生持久副作用。

## 3. 网络创建默认值与 CLI

- [x] 3.1 在 agent 创建网络流程加入系统随机的 `net-[a-z0-9]{2}` 缺省名称，确保空白/缺省名称得到同一 display_name 与默认 EasyTier network_name，同时 network_id 和 secret 保持独立。
- [x] 3.2 CLI 增加发现服务设置参数，并让 `resolve-peer` 支持四种发现 URI及结构化发现链输出；创建命令接受全部直接和发现协议。
- [x] 3.3 增加 agent/CLI 回归测试，验证显式旧名称兼容、自动名称格式、重复名称不合并网络、最终节点去重及创建后仍为 stopped。

## 4. 正式 GUI 创建体验

- [x] 4.1 在 `NetworkForm` 增加可测试的随机名称生成器并于新建时预填 `net-xx`，编辑时保留原名称，用户仍可修改且提交值同步给 display_name 和初始 EasyTier network_name。
- [x] 4.2 在设置页实现发现服务地址；创建页支持公共候选，初始节点输入直接接受四种发现 URI并在测试时自动预览，保存时保留原始 URI，同时区分未配置、陈旧缓存、空结果和失败。
- [x] 4.3 重构 `NetworkPanel` 为显式名片视图：创建后按新 network_id 读取已保存配置再进入名片；普通网络页和引导弹窗均可见，返回动作正确，创建失败保留表单，创建成功但取卡失败只重试取卡而不重复创建。
- [x] 4.4 补齐中英文文案、无障碍名称和响应式样式，确保公共节点长 URL、候选列表与名片在窄屏不造成整页横向溢出。

## 5. 验证与交付

- [x] 5.1 扩展 GUI 单元与浏览器测试，覆盖发现服务设置、四种发现 URI、七种直接协议、原始 URI保留、候选确认/去重和既有名片/失败状态。
  - 2026-09-23：新增目标浏览器用例并使用系统 Chrome 执行完整 Playwright 套件，71/71 通过。
- [x] 5.2 运行独立服务与 Rust 工作区测试、GUI 类型检查/单元测试/生产构建、OpenAPI 校验和 OpenSpec 严格验证，修复全部回归。
- [x] 5.3 对生产 uptime API 和受控 TXT 域名执行一次不含凭据的只读冒烟，记录协议/字段兼容性；外网失败只记录环境事实，不替代 fixture 测试或宣称节点可组网。
  - 2026-09-22：构建机访问生产 HTTPS API 在 TLS 握手阶段返回 `SSL_ERROR_SYSCALL`，未取得或记录目录数据；DNS TXT 冒烟成功，`et2.lctn.cc.cd` 返回 `udp://et.lctn.cc.cd:7178`，`et.lctn.cc.cd` 返回 `tcp://et.lctn.cc.cd:7196`。这只确认 TXT 格式兼容，不代表节点可入网。
  - 2026-09-22：独立服务直接访问 ruixuan 的 status-page/heartbeat JSON 成功，`/v1/nodes` 返回 9 个受限候选，`/v1/discover` 返回逐行 URI；该结果只验证适配契约，不证明任一节点可加入 overlay。
- [x] 5.4 生成 Windows 安装包并安装到 240 供用户验收；生成 Android 包，分别验证公共节点获取、TXT 解析、创建后名片和实际 EasyTier 启停，且不把普通 ping/TCP 结果当作 Windows–Android 组网成功。
  - 2026-09-24：r16 Windows 安装器在 240 覆盖安装 exit 0，六个运行时文件哈希匹配；Android r16 覆盖安装成功，双端原身份/配置保留。两端公共目录均返回 200/8 项，TXT 均解析到最终 TCP；原生创建后名片、二维码及复制入口已检查。实际 Wintun/VpnService 启停、双端 Rove 发现与操作、停止失效和物理网负向均通过，详见 native-overlay-implementation.md。Android 实机环境为 Redroid，不代表实体手机/软键盘验收。
  - 2026-09-22：Windows 安装包已生成并在 `10.1.2.240` 覆盖安装成功（exit 0）；Android arm64 Debug APK 已生成。安装元数据仍为 `overlay_runtime=none`，Android 也没有 native VPN，因此无法执行“实际 EasyTier 启停/Windows–Android overlay 组网”验收，本任务保持未完成。
  - 2026-09-23：重新生成包含本变更的 Windows NSIS 安装器和 Android arm64 Debug APK；Android 14 Redroid 覆盖安装、显式冷启动和 resumed 状态通过。当前会话没有 240 可用的 SSH 认证，增量 Windows 包尚未传输；更关键的是两个平台仍缺 EasyTier runtime/native VPN，故不能验证实际网络启停、overlay 发现和 Rove overlay 通信，本任务继续保持未完成。
  - 2026-09-23：取得授权凭据后，r2 安装器已传到 240，远端哈希匹配，运行中进程保护返回预期 exit 32，停止旧 agent 后覆盖安装 exit 0，三个主二进制哈希匹配且 `Test-Rove.ps1` PASS；新版 agent 已启动供用户验收。平台 EasyTier runtime/native VPN 仍未实现，因此本任务继续保持未完成。
  - 2026-09-23：按本地联调要求增加安装包级发现服务默认值，当前为 `http://10.1.2.237:8090`；用户持久设置可覆盖/清除，正式包可通过打包参数替换成 HTTPS 域名。仅安装包中完全匹配的私网 HTTP 默认值获准使用，不放宽其他远程 HTTP 地址。
  - 2026-09-23：r4 修复 GUI 对发现 URI直接执行最终 connector 探测导致的 `Invalid bootstrap endpoint`：测试时先解析完整发现链、展示实际最终 URI并对其执行受限探测，提交仍保留原始发现 URI。GUI 单元 24/24、浏览器 71/71、生产构建和 OpenSpec 严格验证通过；r4 已在 240 覆盖安装 exit 0，真实 `https://et-public.lctn.site/` 解析为 HTTPS → TXT → TCP 且最终 TCP 探测可达。Windows overlay runtime 与 Android native VPN 仍缺失，本任务继续保持未完成。
  - 2026-09-23：r5 删除独立“发现地址/解析”区域，四种发现 URI直接填写为初始节点并在测试时自动解析；同时把引导页网络步骤“下一步”移到原“随时可以稍后设置”的底部位置。GUI 单元 24/24、目标发现用例 1/1、引导用例 9/9、生产构建和 OpenSpec 严格验证通过；r5 已在 240 覆盖安装 exit 0，真实 HTTPS → TXT → TCP 解析通过且 agent 在 SSH 断开后持续运行。实际 overlay 前置能力仍缺失，本任务继续保持未完成。

## 6. 独立发现服务与迁移

- [x] 6.1 根据官方 EasyTier HTTP(S)/TXT/SRV 发现语义和 `uptime.easytier.cn` 权威 NXDOMAIN 调查，修订 proposal、design、spec 和任务，取消失效域名假设。
- [x] 6.2 在 `/home/git-ssd/rove-discovery` 创建独立服务，提供 `/healthz`、`/v1/nodes`、`/v1/discover`，从 ruixuan 状态页与 heartbeat API生成受限、去重、带来源的候选。
- [x] 6.3 为独立服务加入上游超时/大小/条数限制、短期缓存与 stale/503 降级、配置和反向代理 HTTPS 部署文档及 fixture 测试。
- [x] 6.4 端到端验证独立服务输出可被 Rove 目录客户端读取，HTTP 发现正文可被官方 EasyTier 语义解析，且不将 Kuma 健康状态宣称为 overlay 成功。
  - 2026-09-22：本地 Rove fixture 覆盖目录读取、`X-Network-Name`、HTTP 正文、redirect、循环与 16 跳限制；独立服务对 ruixuan 的实时只读冒烟返回 `stale=false`。随后用真实 agent 设置 `http://127.0.0.1:8090`，CLI 读取到 8 个过滤后候选，并把 `/v1/discover` 链式解析为 `wss://et.chinokou.cn/`。Playwright 浏览器因本机未安装 Chromium 未执行，不影响 agent/服务契约验证。

## 7. 原型网络导航与自包含名片

- [x] 7.1 按原型把正式网络标签拆成列表、创建、加入、名片和编辑互斥视图，创建成功转名片，已有网络从列表查看名片。
- [x] 7.2 GUI 去掉 URL分享/导入入口，以有界 Base64URL JoinConfig 统一名片文本、配置文件、二维码和本地解码导入，同时保留 agent 端输入校验。
- [x] 7.3 更新双语文案、单元/浏览器测试、OpenSpec 严格验证和 Windows 240 增量安装记录，不把配置导入描述为实际入网。
  - 2026-09-23：GUI 单元测试 24/24、完整浏览器套件 72/72、生产构建及 OpenSpec 严格验证通过。r6 安装器已在 240 覆盖安装 exit 0，远端哈希匹配；agent PID `25876` 在 SSH 断开后持续运行。该验证只覆盖 UI、配置编码/解码和安装，不代表 EasyTier overlay 已启动或 Windows–Android 已组网。

## 8. 原型纠偏与真实二维码修复

- [x] 8.1 创建和加入只从右上角全局「＋」菜单打开弹窗，网络标签始终展示列表，创建成功弹窗转名片。
- [x] 8.2 名片仅保留复制配置分享操作；原生二维码接受 Base64 载荷，失败可见并可重试，覆盖真实生成与解码。
- [x] 8.3 验证桌面/窄屏入口、弹窗和名片，重新打包安装到 Windows 240 并记录结果。
  - 2026-09-23：21 项相关浏览器测试及最终布局 4 项复测通过；SDK 真实二维码解码 2/2、Linux Tauri IPC 2/2、GUI 构建和 OpenSpec 严格校验通过。r7 已在 240 安装 exit 0，安装器及 GUI 哈希匹配，agent PID `22736`。修正 r6 的列表内入口和 URL-only 二维码错误；Windows 交互桌面由用户验收，实际 overlay 任务 5.4 保持未完成。

## 9. 精简列表与移动端回退布局

- [x] 9.1 去掉列表重复标题及长说明，将刷新收至页头图标；统一网络操作和引导弹窗的左上角回退。
- [x] 9.2 手机名片/表单采用全屏、固定回退栏和复制栏及独立内容滚动，验证竖屏、横屏和桌面。
- [x] 9.3 构建并更新 Windows 240 安装包，记录本轮界面验证结果。
  - 2026-09-23：4 项相关浏览器用例及新增尺寸/滚动断言通过；检查桌面、小屏、手机竖屏与横屏截图。GUI 构建和 OpenSpec 严格校验通过。r8 已在 240 安装 exit 0，安装器与 GUI 哈希匹配，agent PID `3096`。移动端仅完成响应式验证，Android 软键盘实机未验收；实际 overlay 任务 5.4 继续未完成。
- [x] 9.4 将桌面/移动端兼容、回退导航、尺寸与交互验收要求写入仓库 `AGENTS.md`，明确响应式测试和实际设备验证的区别。

## 10. 网络详情与下属设备

- [x] 10.1 网络条目进入独立详情路由，显示该网络实际状态、地址和管理操作，移除列表页全局设备入口。
- [x] 10.2 详情页按 network_id 和目标 agent 加载设备，拒绝跨网络/过期结果，并验证桌面和移动导航。
- [x] 10.3 构建待验收 Windows 包；用户查看 r8 时保持该已打开窗口不变，记录新版本尚未安装。
  - 2026-09-23：r9 已生成，16 项相关回归及最终 9 项专项复测、GUI 构建、Windows 交叉构建和 OpenSpec 严格验证通过。全量测试尝试被 SIGTERM 中断，不计完整通过。240 的 r8 已在交互桌面启动并置前；用户查看期间没有覆盖 r9，平台真实 overlay 验收仍未完成。

## 11. Android 发现默认值与错误呈现

- [x] 11.1 Android 通过打包环境变量嵌入经校验的发现服务默认值，不从 app_process 旁读取 Windows sidecar；用户设置与显式清除优先。
- [x] 11.2 公共节点获取与网络启动错误增加状态图标和双语说明，保持手动输入及重试可用。
- [x] 11.3 验证默认值、错误 UI、四种尺寸及 Android 实际公共目录获取，更新包与交付记录；Windows runtime 缺失不得标为已修复。
- [x] 11.4 修复 Android 无系统 resolv.conf 时 TXT/SRV 解析器创建失败，使用固定 EasyTier 同源后备 DNS，保留超时和无密钥边界并验证真实 TXT。
  - 2026-09-23：Android r10 覆盖安装并启动成功，设备身份不变，真实默认地址为 `http://10.1.2.237:8090`；API 和实际创建表单均取得 8 个节点（stale=false），360px 无横向溢出。TXT 真实解析为 `tcp://et.lctn.cc.cd:4107`，但该设备访问 `https://et-public.lctn.site/` 仍返回 `public_nodes_unavailable`，不宣称 HTTPS 链已通过。默认值 2 项、显式清除 1 项、发现模块 6 项、GUI 单元 24 项、错误浏览器 5 项及协议回归 1 项通过。240 的 scrcpy 窗口保持运行；Windows 安装仍为 r9，运行时未接入，任务 5.4 保持未完成。

## 12. Android HTTPS 发现间歇失败

- [x] 12.1 复验完整 HTTPS → TXT → connector 链，将 HTTP 请求/状态错误与 DNS 错误区分，错误不得暴露 URL 查询、请求头或网络密钥。
- [x] 12.2 DNS 单次等待与重试适配既有 3 秒总期限，用本地丢包 fixture 验证可在期限内重试；保留 TLS 校验。
- [x] 12.3 验证错误图标与双语文案、重新安装 Android 并实测发现和最终节点探测，记录仍存在的外部服务/平台边界。
  - 2026-09-23：r11 已覆盖安装；HTTPS → TXT → connector API 连续 3 次成功，真实 GUI 再次解析为 TCP 并探测可达（315ms），原始 HTTPS 输入未被替换。无效自签证书被拒绝，fixture 的 https_requests=0、tls_rejections=1，临时反向端口和 fixture 已关闭。目录仍返回 8 项，原 device_id 不变；240 交互桌面已重新打开 Rove-Android（PID 11792）。Rust 发现 8 项、GUI 单元 24 项、浏览器错误/恢复 9 项通过，四种截图检查及 OpenSpec 严格校验通过。此前 r10 的笼统 HTTP 错误无法追溯具体原因；本轮确实复现并修复 DNS 内部等待长于外层期限的问题，不承诺外部服务永不失败，任务 5.4 仍未完成。

## 13. 共享 Windows / Android 原生运行时

- [x] 13.1 提取共享 EasyTier 配置转换并增加应用内实例后端，保留外部 RPC 后端，验证实例隔离与真实管理启停。
  - 应用内后端调用固定 revision 的 NetworkInstanceManager，复用 NetworkConfig 转换；实际 no-TUN 管理测试验证两个实例、原版运行信息、重复启动拒绝、单独停止和整体退出。该测试不构成实际组网证据。
  - 2026-09-23：agent 65 项回归、最终 5 项专项测试、Windows x86_64 与 Android arm64 的 easytier-native 编译检查及 OpenSpec 严格验证通过。未生成或部署新安装包；平台运行时接入与真实组网验收仍未完成。
- [x] 13.2 把网卡验证、路由与传输绑定收敛到薄平台接口，复用既有生命周期和设备发现，保持 Linux 回归。
  - 2026-09-24：Windows/Android 实际共同网络使用共享观察器、候选发现和源地址绑定；修复原生异步初始化与公共中继下三次握手总预算过短。Linux native-feature agent 75/75、SDK 隔离传输 2/2 通过；两端安装版实际通信通过。
  - 已提取 overlay_platform/linux.rs，既有内核校验与绑定行为不变；Windows/Android 的平台接入仍未完成。
  - 2026-09-24：Windows IP Helper 网卡/地址/路由检查与共享观察器、设备发现和服务绑定已接通；SDK 增加源地址绑定、显式 IP 端点限制及无物理网回退测试。尚未接入生产权限 helper。240 只读检查发现 Mihomo 网卡开启转发，当前保守隔离检查会拒绝该环境；需要解决与现有代理共存的隔离设计，未改动宿主设置，任务保持未完成。
- [x] 13.3 接入 Windows Wintun、受限权限处理和运行时打包，验证应用内启停、失败恢复和数据保留。
  - 2026-09-24：240 r16 普通权限 GUI 自动启动原身份 agent；从应用启动网络自动创建独立 helper（父 PID 为 agent、请求绑定实际 agent PID），实际 Wintun/IP/路由就绪后运行。确认旧 helper 退出后的再启动可恢复，停止清除网卡/API/自有 overlay 放行规则。未修改 Mihomo 转发、系统代理或全局防火墙策略；现有 helper 入站阻止规则保留，正向验收由两端主动连接公共初始节点完成。
  - 2026-09-24：已实现同账户/PID 校验的独立提权 helper、原生 Wintun、每网络动态 WFP 隔离、断连清理、普通用户 GUI 自动连接/启动 agent，以及驱动哈希和安装包依赖检查。Windows pnet 接口枚举改用已有 netdev 系统适配，移除静态 Npcap 依赖，FakeTCP 保留上游 WinDivert；正在构建并准备 240 实机验证，未以代码或编译检查代替验收。
- [x] 13.4 接入 Android VpnService 授权、前台生命周期、EasyTier tun fd 与撤销清理，验证多网络约束及真实启停。
  - 2026-09-24：r13 已覆盖安装并保留身份；Android 14 Redroid 原生验证两个不重叠网段同时 running，系统 TUN 持有两地址，停止 A 后 B 仍运行；系统断开后 B failed、地址和前台服务清理，错误保留；应用再次启动网络恢复 running。修复地址别名校验和失败诊断被清除，串行回归 70/70。环境仅补齐容器缺失的系统 `/dev/tun` 节点，应用仍为普通 UID、使用 VpnService；未外推为实体手机或跨设备通信通过。任务 13.5 与 5.4 仍未完成。
  - 2026-09-24：新增系统授权/前台 VpnService、JNI fd 所有权交接、实际 TUN 接口识别与共享观察器接入；实现多实例报文通道、重配与停止清理。8 项共享专项及四种尺寸的权限失败/复制测试通过；原生 APK 构建与实机验收进行中，本项尚不勾选。
  - 2026-09-24：r12 arm64 APK 构建、覆盖安装及启动成功，原 device_id 保持不变；共享专项 8/8、agent 串行回归 68/68、Windows 原生测试编译通过。独立“VPN 接入测试”启动已触发真实 Android 系统授权框，等待用户手动确认；实际 VPN 启停、多网络及撤销验收仍未完成。
  - 已增加拥有 fd 生命周期的原版 tun 接口及共享 VpnPlan：校验实际地址、网段重叠、独立实例、IPv4 包长度和跨网段注入边界。路由规划现已连接到 Android VPN 服务，但不能以编译和 fixture 宣称真实 VPN 或多网络已经可用。
- [x] 13.5 安装到 240 与 Android，验收双端 overlay 发现、真实 Rove 操作、停止失效及物理网负向边界，记录仍未完成项。
  - 2026-09-24：r16 双端同一 network_id、独立原 device_id，实际地址 Windows 10.203.240.3 / Android 10.203.240.2；双方设备列表均有对端，双向目标 get_device 均返回 200 和正确平台身份。分别停止任一端后，另一端返回 target_unreachable、不转本机执行；Windows 网卡/监听/自有放行规则归零，Android tun0/监听消失。237 访问 Windows 物理 IP API 及经物理接口路由至 overlay API 均超时，临时路由已删除。ADB/SSH/CDP 只用于控制及检查，不承载业务通信；实体手机、软键盘和全部七种 connector 的原生逐项互通未外推。
