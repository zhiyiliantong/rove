# bootstrap-rove 实现记录

更新日期：2026-09-21。任务复选框以 OpenSpec tasks.md 为准；规划产出物 complete 不表示应用完成。

`bootstrap-rove` 已按用户要求阶段归档，位置为 `openspec/changes/archive/2026-09-21-bootstrap-rove/`。以下 84/93 是归档快照，原 9 项仍未完成；后续不再向本提案追加任务，按照 [小提案计划](next-iterations.md) 分别立项。现行接口固定在 `api/`，现行能力契约在 `openspec/specs/`。

当前任务进度 **84/93**（原 1–12 组 63/71，原型迁移第 13 组 9/9，追加 API 同步/独立账号授权第 14 组 1/2，四步导航及反馈第 15 组 4/4，即时会话与 Android HTTPS 第 16 组 4/4，紧凑会话及入口反馈第 17 组 1/1，图标导航与会话命名第 18 组 1/1，消息复制第 19 组 1/1）。原型迁移进入核心功能用户验收，不等于全平台或官方会员登录已完成。下方较早的测试数、产物及环境阻碍是历史记录，不代表最终状态。

## 2026-09-21：用户消息与失败信息复制

按图片标注增加两类常显复制按钮，支持中文/英文和手动复制降级。9 项浏览器、5 项国际化测试及生产/Android 构建通过。最终签名 APK 已覆盖安装 242，12 条会话、4 个型号、草稿和身份保留；原生触屏复制成功、44×44px 按钮、无横向溢出。详见 [交付记录](message-copy-delivery.md)。

## 2026-09-20：全宽紧凑底栏、会话命名与归档操作

底栏保持全宽、只显示图标并降低高度；标题原地编辑，新会话首次已接受消息生成标题并保护手动名称。加号移除归档当前会话项，归档列表新增逐条恢复/确认删除及活动作业检查。24 项单元、65 项浏览器、10 项 agent 会话及 7 项 CLI 测试通过。最终 APK 覆盖升级 242，保留 12 条会话、4 个型号、2 份草稿、身份及引导偏好；原生实测底栏 360×53px、归档行操作与原地编辑。未删除用户历史、未使用真实模型。详见 [交付记录](conversation-title-delivery.md)。

## 2026-09-20：紧凑会话与网络内 API 同步入口

完成六项反馈：移除执行详情/成功标签、会话倒序分页、手机对话隐藏底栏、压缩顶栏、引导直接添加、网络内 API 同步。24 项单元、55 项浏览器、8 项会话和 7 项 CLI 测试通过。Android 签名验收包已覆盖安装 242，保留最新 11 条会话、4 个型号、草稿与设备身份；按用户已有设置重播导航并打开添加表单。240 原 scrcpy 仍在。没有调用真实模型或同步密钥。详见 [交付记录](compact-chat-delivery.md)。

## 2026-09-20：即时对话与 Android 模型连接修正

完成 16.1–16.4：单一固定标题栏、连续滚动会话、点击标题改名、加号归档操作、默认开启的建议开关；合并型号与编辑名称、自动获取目录及失败回退。定位并修复 Android HTTPS 平台验证器未初始化造成的任务 panic/SDK 超时，未禁用证书校验。24 项单元、49 项浏览器回归、契约与严格校验通过；真实 Android 公开 HTTPS 成功、自签名证书拒绝。最终 APK 已覆盖安装 242，身份、10 条会话、2 个型号与草稿保留；真实密钥/额度待用户点击测试确认。详见 [本轮交付](chat-feedback-delivery.md)。

## 2026-09-18：导航与模型测试验收修订

完成 15.1–15.4。四步导航、精简会话列表、下次启动重播及持久草稿迁入正式 GUI；新增 OpenAPI/CLI/GUI 模型推理测试，未测试成功不能完成/跳过引导，密钥输入显隐、DeepSeek OpenAI/Anthropic 协议切换和短屏纵向滚动。24 项前端单元、42 项浏览器回归及最后 10 项相关复测通过，Rust/Clippy/52 操作契约校验通过。Android arm64 最终验收 APK 已覆盖升级 242，保留身份、10 条会话、2 个型号及中文偏好；240 的原 scrcpy 窗口已激活。未使用用户真实 API 密钥测试，不代表真实供应商授权验收通过。制品校验和、范围和证据见 `docs/onboarding-delivery.md`。

## 2026-09-18：核心功能验收包与原生生命周期

完成 11.1、13.8。最新 Ubuntu runtime/GUI deb 和 Android arm64 APK 已包含多网络、API 配置同步、会话归档、多型号、国际化及对话管理增量。验收入口 `http://10.1.2.237:4174/` 只提供指定制品/截图/说明；`4173` 仍是已归档独立原型，没有伪装为正式浏览器应用或开放 agent。

- 真实 Linux GTK/WebKit + systemd 系统网络临时服务/用户 agent：界面会话与 CLI 身份一致，正常关闭窗口后作业继续、PID 不变、重开可读结果，重启保留状态。linux-desktop-v5/result.json 通过。使用独立目录与 loopback 模型夹具，不改现有宿主网络、启动项或 lingering。Ubuntu 两包安装、同版本升级、数据保留卸载再次通过。
- Android 14 arm64 覆盖升级保留原身份、两条旧会话、并发设置和中文偏好。真实页面提交经 Rig 执行 `system_exec` 得到实际 `unsupported`，作业完成；页面归档/归档管理恢复通过。仅本轮的临时模型连接被移除，历史保留。未使用供应商付费 API。
- 实际截图发现系统栏遮挡，MainActivity 按系统栏/屏幕缺口/IME insets 调整整个 WebView 并消费一次；无固定机型 padding，不依赖不可靠的 CSS env()。重包覆盖安装后竖屏 360×568、横屏 592×336 无横向溢出、底栏保留；具体校验和与证据见交付记录。
- 修复 Vite Node 类型依赖缺失和英文侧栏标题拆词。95 项 Rust、20 项前端单元、32 项浏览器、2 项原生 IPC 测试通过，布局改动后 6 项相关浏览器测试再通过。OpenAPI 51 操作/功能对等矩阵/OpenSpec strict/fmt 通过。包测试改为有界就绪探测，避免固定等待一秒在高负载下误报。

当前缓存：GUI node_modules 链接到 `/mnt/data/rove-ui-deps-20260918-types-gBCsCM/node_modules`；Android app/build 链接恢复到此前迁出的专用缓存。npm 安装会把 node_modules 链接替换为真实目录，因此新增类型后再次完整迁回数据盘，未删除原缓存或用户源码。仓库分区仍接近满额。

剩余 **9 项**：1.4、10.2、10.6、11.2–11.5、12.4、14.2。官方账号登录/执行尚未适配，Windows overlay 与安装器、物理相机、Apple 环境及全平台矩阵未完成；Android native VPN/后台仍不在首版壳能力声明中。现请求的是已交付核心功能验收，不是全部 82 项验收通过。详见 [本次验收](core-acceptance.md)。

## 2026-09-18：API 配置同步，账号逐设备授权

完成 14.1：正式 GUI 模型页新增“同步本机 API 配置”，CLI model sync 对等。来源 agent 从私有目录读取 API 连接及全部型号，经 SDK 直接发送给目标；界面请求不含密钥。相同来源快照幂等，变更/替换须确认，保留仍存在型号的目标 ID 和默认选择；原子失败不写半份配置，来源删除不删除目标副本。拒绝额外账号令牌字段，会员账号每设备独立授权，14.2 官方登录与执行适配仍未完成。详见 [API 同步](model-sync.md)。

95 项 Linux Rust 回归、20 项前端单元、32 项浏览器测试、生产构建、Clippy 通过；首轮并行 CLI 测试有四项触发原有 10 秒时限，降低测试并发后全套通过，未放宽断言。新增 2 个 API，总计 51 操作；OpenAPI/功能映射和 OpenSpec strict 校验通过。两个隔离 Linux 容器以真实 EasyTier overlay 完成配置同步、重试、密钥变更确认、来源删除独立性及原有多网络/服务回归，api-sync-overlay-v1.json 为 PASS；不是 Android → Windows 实机或付费提供方验证。

Windows GNU agent/CLI 交叉检查与 2 项 Tauri MockRuntime/真实 socket 测试通过；不等于 Windows 网络运行时已实现或实机通过。

源码增量尚未重新打入 Ubuntu/Android 安装包，4173 原型不变。仓库分区耗尽后，将未跟踪的 GUI node_modules 完整迁到 /mnt/data/rove-ui-deps-20260918-r5lb9c/node_modules，并在原位置保留链接；依赖、源码和旧制品没有删除，迁移后单元/生产构建通过。Vite 开发服务仅额外允许真实依赖目录以加载图标字体，不开放数据盘父目录。macOS 继续暂缓；剩余 11 项包括原有 10 项平台/验收与新追加的 14.2。

## 2026-09-17：自动/手动地址模式与真实多网络

完成 13.6。用户澄清后撤销“DHCP 指定池需补丁”的假设，使用未修改 EasyTier。自动模式读实际网段，手动模式保存网段和不随分享的本机 IP；正式 GUI 新网络表单、密钥显隐、四字段名片、多个初始节点和逐条 TCP 探测，CLI 同步支持。新增 test_network_peer，总计 49 个操作。

Linux driver 按网络管理监听、SDK 路由、发现与服务，停止单网络不再全局关闭。两个隔离容器各运行两个网段，双网身份发现/远端调用、停一网保另一网认证服务、恢复监听端口、包含关系拒绝及原版 DHCP 实际重叠拒绝通过。最后运行结果为 multinetwork-v3.json，先前失败脚本报告保留。前端 20 项单元与 30 项浏览器测试、生产构建通过。详见 [多网络实现](multinetwork.md)。

最终 Linux Rust 回归 91 项通过（包含 EasyTier feature、CLI 新命令、SDK/socket/HTTP 全操作覆盖），另有 2 项 Tauri MockRuntime/真实 socket 测试通过；EasyTier all-targets Clippy 和 Windows GNU agent/CLI 交叉检查通过。49 操作 OpenAPI 契约、映射、OpenSpec strict、fmt/diff 校验通过。Windows 交叉检查不代表多网络已在 Windows 原生实现或验收。

用户要求暂不考虑 macOS 环境，本轮未搭建 Mac/Xcode/虚拟机或改变 Apple 任务验收条件。剩余 **10 项**：1.4、10.2、10.6、11.1–11.5、12.4、13.8；不再有 DHCP 补丁决策阻塞。Windows 网络服务/安装器、平台生命周期和整体交付仍需继续。此次源码/正式 GUI 增量尚未进入下节历史安装包，4173 原型不变。

## 2026-09-17：核心管理、跨机闭环与平台复验

本轮完成 3.1、8.6、10.5、12.1、13.7 五项。新增有界只读存储 API / CLI / AI 工具、服务目录按网络选择、服务管理新会话与影响确认对话约定；桌面文件导入、移动扫码生命周期及降级处理有回归。OpenAPI 当前为 agent 45 + 配置服务 3 个操作。存储统计不读取文件内容、不跟随符号链接、不承诺可回收字节；未实现在线搬迁数据目录或硬审批系统。

两台不同架构的 Linux 宿主完成真实 EasyTier TUN 组网、分享导入、身份发现、远端模型配置、本机归属会话的远端 Rig 工具执行、SSE 和带认证服务访问。停网、恢复原端口、取消发布后目标应用继续运行均通过。使用确定性本地模型与隔离应用，不使用收费凭据；详见 [跨宿主验收](two-host-acceptance.md)。

Windows 11 原生 agent 测试 21 项通过，含同账号 named pipe 和匿名身份拒绝；Android 14 arm64 覆盖安装新 APK，实际 WebView 操作、SQLite v2→v5 迁移备份、身份/设置/旧历史保留、语言切换重启持久化以及真实 Rig 工具返回 unsupported 均通过。临时模型配置已清除，不把 Android 本机运行当作 Android → Windows overlay。Ubuntu 两个开发 deb 的隔离安装、同版本升级与保留数据卸载通过。交付与哈希见 [Ubuntu / Android 安装包](ubuntu-android-delivery.md)。

本轮通过：87 项 Linux Rust 测试（含 SDK）、19 项前端单元测试、29 项正式 GUI 浏览器测试、2 项 Tauri MockRuntime/真实 socket 测试，以及类型/构建、Clippy/fmt、OpenAPI 和 OpenSpec 校验。新增跨宿主夹具另经两个架构编译与实际运行验证。浏览器测试用真实 QR 像素流经过 jsQR，但 Redroid 无可用相机，物理相机尚未验收。

### 上一增量时剩余 11 项（13.6 已在上节完成）

| 任务 | 尚未完成的条件 |
| --- | --- |
| 1.4、10.6、11.3、11.5 | Apple 本机构建/运行、macOS 网络与后台服务、iOS 壳；未提供 Mac/Xcode 环境 |
| 10.2 | 真实移动相机扫码；已有文件/URL/手动导入和浏览器真实像素解码测试 |
| 11.1 | Linux 实际桌面与 systemd 用户生命周期联合验收；包构建/隔离安装不替代此项 |
| 11.2 | Windows 正式网络运行时、系统服务及安装器；named pipe 通过不等于网络已交付 |
| 11.4 | 声明支持平台的完整安装/升级/保留数据卸载矩阵 |
| 12.4、13.8 | 剩余功能迁移后的整体 GUI/CLI、平台和真实用户验收 |
| 13.6 | 四字段网络名片、节点逐条测试与不重叠多网络；需确定 EasyTier DHCP 管理接口修正方案 |

本轮复查宿主 PID 1 为 systemd；早期“PID 1 是 codex”的环境描述不再适用，尚未安装/启用宿主 Rove 服务或修改账号 lingering。仓库分区耗尽后，将未跟踪的 Android `app/build` 缓存完整迁至 `/mnt/data/rove-android-build-backup-20260917-QCoFzi/build`，可恢复或重新构建；源码、原 target 链接、已交付 APK 和用户数据保留。

## 2026-09-17：补齐本机会话关联远端执行

完成 13.5，当前 **64/80**。会话元数据保存在发起端，远端执行事实独立保留；SQLite v5 升级前备份，保存不可变目标、原请求/wire 请求、远端 run 与快照/水位。重启不重放未确认请求、不标记远端 interrupted；丢响应可用原 ID 显式重试或只读查询恢复关联。离线缓存明确标记，归档删除有未确认/非终态保护。GUI 切换设备不再重建会话，CLI 增加关联创建与 pending 查询。详细协议与边界见 [远端会话](remote-conversations.md)。

本轮 85 项 Rust 回归、18 项前端单元测试、24 项浏览器测试、2 项 Tauri MockRuntime/真实 socket 测试通过，类型/生产构建、Clippy/fmt 和 47 操作契约检查通过。测试以隔离双 agent、真实 HTTP/SSE/socket、本地可控模型及 GUI IPC fixture 验证，不是 Android → Windows overlay 实机验收。未打新安装包，4173 原型不变；13.6–13.8 及其他平台任务继续保留。

## 2026-09-17：追加中英文国际化

新增并完成 13.9，当前合计 **63/80**。正式 GUI 支持简体中文、英文与跟随系统，可在设置中即时切换并持久保存；导航、表单、组件提示、日期/数字与常见错误同步切换，用户原文、草稿和运行中订阅保持不变。详见 [国际化说明](internationalization.md)。CLI/API 数据和 agent 日志不自动翻译。

本轮最终检查：18 项 GUI 单元测试、22 项浏览器测试、80 项 Rust 回归、2 项 Tauri MockRuntime/真实 socket 测试通过；Vue 类型检查/生产构建、Rust Clippy/fmt、46 个 OpenAPI 操作及 OpenSpec strict 校验通过。浏览器验证包含英文 320/1440px 布局与动态切换，不冒充 Android 原生 WebView 实机验收。4173 原型与 2026-09-16 安装包未更新；13.5 远端关联及其他未完成项继续保留。

## 2026-09-17：优先实现模型核心链路

按用户要求暂停扩展平台打包，继续实现 13.5。新增 7 个模型目录操作（agent 43 / 配置服务 3），SQLite v4 备份迁移、共享连接凭据、多型号原子导入、可编辑自动名称、默认模型与持久会话选择、真实型号枚举、首次现代配置的唯一网络引导会话。GUI/CLI 使用同一 API；会话加号进入型号选择。创建网络后自动导出完整入网配置，漂亮的四字段名片仍待 13.6。

执行接入 Rig 的 12 家原生提供方，保留旧 OpenAI 兼容配置。适配器请求格式、Anthropic/Gemini/Ollama 原生流式回复、脱敏、枚举分页/重定向拒绝/大小限制、删除已选择模型不回退、运行中快照、升级身份与配置保留都有本地测试。没有使用真实收费模型密钥，没有将会员登录模拟成可用。型号参考目录沿用已确认原型，不是账号权限列表。

**该增量时任务仍为 62/79**（后续国际化见上节）：13.5 中的本机会话关联远端执行尚未完成，不因模型子项完成而勾选整项。多网络、网络表单和节点测试、管理存储的安全影响确认及平台验收继续保留。此增量不更新 2026-09-16 的安装包，不改变 4173 原型服务。详细接口及验证入口见 [核心模型链路](core-model-workflow.md)。

## 2026-09-16：真实归档、对话管理工具与重新打包

使用 `openspec-apply-change` 继续 `bootstrap-rove`，完成 13.4，合计 **62/79**。新增 3 个归档 API（agent 共 36、配置服务 3），SQLite v3 备份迁移、活跃作业保护、删除历史后的去重墓碑、SDK/CLI/GUI 接入和跨客户端测试。`rove_api` 复用设备/设置/网络/服务 API，“管理 Rove”准备独立本机只读草稿；安全确认目前依赖对话提示，不据此勾选完整 13.7。

73 项 Rust 回归、1 项 EasyTier 契约、2 项 Tauri 原生 MockRuntime/真实 socket 测试、11 项前端单元测试、15 项浏览器测试、类型/构建和 OpenAPI/OpenSpec strict 校验通过。工具错误状态丢失已修复。EasyTier DHCP 契约再次确认指定网段被管理接口丢弃，不将解除单网络保护当作多网络实现。

重新建立外置 Rust 缓存并恢复 JDK/Android SDK/NDK，已构建 Ubuntu 24.04 amd64 runtime/GUI `.deb` 和 Android arm64 Debug APK。原 `target` 链接、旧缓存及 4173 原型预览保持不变。Ubuntu 隔离安装/升级/保留数据卸载、Android v2 开发签名和 ABI/资源检查通过；最终文件与哈希见 [Ubuntu / Android 交付记录](ubuntu-android-delivery.md)。这些不是完整原型迁移、真实桌面系统服务或 Android → Windows overlay 验收；13.5–13.8 未完成项继续保留。

## 2026-09-16：原型归档与正式 GUI 首批迁移

已同步并归档 conversation-archive-management（10/10）。用户授权回到 bootstrap-rove 按已确认原型实现；新增第 13 组 8 项任务，首批完成迁移边界文档、PrimeVue 响应式导航和真实会话输出展示（13.1–13.3），合计 61/79；原 71 项不因新页面完成而补勾。

正式 GUI 11 项单元测试、11 项浏览器 IPC 适配器测试、类型/生产构建通过。切页不丢同一目标的草稿和订阅，富文本/复制/本地语音降级、流式恢复与不确定提交去重均有回归；服务管理表单移除，CLI/API 保持。详细边界与复现见 [正式 GUI 原型迁移记录](gui-prototype-migration.md)。原型 4173 保留，不是正式 GUI 的新部署地址。

Rust 原生回归在编译前受阻：仓库 `target` 指向已不存在的临时构建缓存，仓库分区仅约 392 MB 可用；未清理或替换用户文件。本轮不声明新原生包通过。真实归档、多模型、本机会话关联远端、网络和对话管理工具仍在 13.4–13.8 待实现，不能把本阶段界面构建当作完整迁移。

## 2026-09-10：Android / Windows 打包与原生测试

使用 openspec-apply-change 继续 bootstrap-rove，平台任务按完整验收条件保留未勾选。

ARM 主机恢复后的增量：Android 14 纯 64 位容器已成功启动并安装 Rove；真实 WebView 通过嵌入 agent SDK/socket 读取 Android 身份、保存并发设置。强制关闭/重启应用后身份和设置保持不变。可控本机 SSE 模型实际触发 Rig `system_exec` 并得到 `unsupported` 工具结果，事件持久可读，测试模型配置已清除。修复真实窄屏 Grid 横向溢出。该证据不替代 iOS 或 Android native VPN/Windows overlay 验收，任务仍为 58/71。

- Windows x86_64 agent、CLI 和 Tauri GUI 交叉构建成功；便携开发包补齐 `WebView2Loader.dll`。Windows 11 实际 named pipe、配置写入、重启身份/配置持久化测试通过。
- Windows GUI 在已登录用户桌面启动，UI Automation 验证页面渲染的 device_id 与 CLI 一致；这是真实 GUI → SDK → named pipe → agent 链路，不是 MockRuntime。未验收扫码、浏览器、跨账号拒绝、Windows 系统服务与 overlay。
- Android 工程与嵌入 agent 已交付，复用 SDK/socket，不让 GUI 直接管理数据库；应用私有目录禁用备份。Tauri/Gradle 已成功构建 arm64 Debug APK（约 60 MB）。本轮 agent 单元测试 28 项通过，包含嵌入启动/重复启动拒绝/重启身份测试。无 native VPN、后台常驻，也不声明 iOS 验收。
- ARM Android 容器第一次启动因宿主不支持 32 位自检程序失败，已下载纯 64 位镜像；随后宿主从开发机和 Windows 均不可达，镜像导入/停止测试容器最终状态无法确认。未执行宿主重启或业务容器变更，Android 实机安装与跨设备控制仍待完成。

复现入口与当前限制见 [Android / Windows 测试说明](android-windows-testing.md)。以下环境清单是 2026-09-09 历史基线，不覆盖上述最新结果。

最终完整回归：`cargo test --offline --locked --features rove-agent/easytier` **69 项通过**（包括 28 项 agent 单元测试和上游 DHCP 契约测试）。EasyTier all-targets Clippy、fmt、OpenAPI/映射与 OpenSpec 严格校验通过。Windows GNU agent/CLI 交叉检查通过，仍不代表实际 Windows 运行。

## 剩余 13 项与环境缺口

| 任务 | 尚需交付/验证 | 当前不能替代的环境 |
| --- | --- | --- |
| 1.4、3.1 | Windows 同账号/异账号访问、各平台后台接入支持矩阵 | Windows/macOS 原生运行环境；交叉编译不是 ACL 实测 |
| 8.6、12.1 | 物理两设备服务与完整装配闭环 | 两台可安装 Rove 的受控设备与明确部署授权 |
| 10.2、10.5 | 已有 UI 的真实扫码、浏览器打开及 CLI 对等交互验收 | 桌面 WebView、相机与浏览器会话 |
| 10.6、11.5 | 移动 agent library 嵌入、壳构建和 unsupported 验证 | Android SDK/NDK、Xcode/iOS 构建环境及设备/模拟器；核心嵌入代码尚未交付 |
| 11.1 | 已有 Linux 包的 systemd 实际启用、账号/登录生命周期、关闭 GUI 后仍运行 | 以 systemd 管理服务的测试机与桌面会话 |
| 11.2、11.3 | Windows/macOS 网络入口、系统服务集成及安装包 | 对应原生构建/运行环境；这些仍有实现工作，不仅是补测试 |
| 11.4、12.4 | 全平台安装/升级/卸载与最终验收矩阵 | 上述平台结果齐备后才能完成 |

当前进程 1 是 codex，不是 systemd；`adb`、`xcodebuild`、`wine` 未找到，`/dev/kvm` 不存在。没有擅自安装宿主服务、启用账号 lingering、使用用户模型凭据或将容器冒充物理设备。未完成项保持未勾选。

完成 4.4、4.7：新增 Linux 主路由表中非默认、非自身 TUN 的特定前缀冲突拒绝，配合直连子网检查；不承诺解析所有自定义策略路由。实际容器故障测试停止其专用 EasyTier 进程后，网络显示 failed/error，另一份 stopped 配置保留；重启网络进程后自动恢复。设备 hello 分类单元测试覆盖普通节点拒绝、不兼容状态、网络上下文拒绝和 peer_id/IP 改变仍使用同一身份键；真实 overlay 测试覆盖在线发现、地址刷新及重连。

7.6 的可控提供方持久化/失败测试已完成，真实 Rig 流式 system_exec / publish_service 在隔离 overlay 中运行通过。该任务的外部真实模型验收是“有可用模型配置时”的条件项；本次未提供授权模型配置，未调用外部模型或计费接口，不声称完成了该条件项。

进一步完成 4.5、4.8、10.3、12.2、12.3：独立测试探针仅在一次性容器内修改上游实例地址（产品仍用 DHCP），实际验证新地址的发现/API/原端口服务恢复、旧地址 API/代理拒绝及最终停止确认。结合真实 Socket 的客户端切换/取消/重启中断、SDK 断线游标恢复、并发网络启动 202/409 与分享过期测试完成对应场景。前端四组测试、类型检查/生产构建和两项原生 Tauri MockRuntime 桥接测试再次通过；10.3 不包含真实相机或移动平台验收。

Linux 开发 runtime `.deb` 和 Tauri GUI `.deb` 已构建，并在无网络一次性 Ubuntu 24.04 容器共同安装成功；动态库完整，CLI/agent 可运行，同版本重装保留 device_id，卸载保留数据。网络 systemd unit 静态校验通过，但真实 systemd 启用、登录退出、桌面 WebView 及版本迁移仍未据此勾选。详见 [Linux 打包说明](linux-packaging.md)。

HTTP 新增最多 128 个活动连接（包含空闲连接和 SSE），额外连接等待内核队列；持有连接释放后容量可复用。带 EasyTier feature 的 26 项 agent 单元测试通过，包括容量释放测试。此前 65 项默认回归不包含此新增测试。

## 最新增量：真实 Linux overlay 与接口对照

已勾选 4.1、4.2、4.6、8.1、8.3、8.4、9.1、9.3、12.5。Linux 可选 `easytier` feature 已正式接通，不再只是空适配器。默认构建未配置该运行时时仍明确拒绝组网；尚未声明 Windows/macOS 网络运行时支持。

- `network_runtime.rs` 从专属 loopback RPC 创建/停止实例、读取 DHCP 地址并发现设备；单设备加入互斥，停止确认前不允许另一网络加入。后台调和不随客户端断开而取消。身份使用持久 device_id，peer_id 只用于当前路由。
- HTTP 与服务 TCP 监听同时绑定实际 TUN 和 IP，检查实际接口地址与直连物理子网冲突；接口索引变化也重建监听。远端 SDK 同样绑定该 TUN，无代理环境变量或重定向旁路。
- `scripts/check-overlay.py` 两个专用容器实测通过：真实 EasyTier DHCP/TUN、稳定身份发现、CLI Socket → SDK → 远端 HTTP、Rig 接受可控提供方的 system_exec / publish_service、远端 SSE、应用认证、取消发布保留应用。测试不调用收费模型。
- 同一测试通过物理管理端口拒绝、物理入口绕路到 overlay API 拒绝，以及容器重启后身份/网络/原服务端口恢复。目标应用由测试显式重启，不宣称任意应用自动恢复。最新接口索引/地址检查修改后已重新跑通。
- 回归 65 项默认 Rust 测试通过；新增矩阵遍历全部 33 个 agent 操作，经 SDK 对照真实 Socket 与 HTTP 状态及错误。结合独立 SSE 游标/204/410、分页、共享示例和三个密文服务操作测试完成 12.5。矩阵包含缺失资源的错误路径，不把它称为每项业务的成功端到端验收。
- 两份 OpenAPI 检查通过：36 操作、313 引用、21 示例、31 正反例；FR/NFR/用户故事/规格映射检查通过。

容器不等于物理两设备；任意策略路由冲突、跨平台安装、真实桌面相机及移动壳仍未验收。生成包不会自动安装或启动主机服务。

## 当前实现

| 模块 | 已实现 | 未实现 |
| --- | --- | --- |
| rove-core | 稳定 UUID 值类型、作业状态、UTC 时间 | 其余领域行为 |
| rove-protocol | OpenAPI 构建生成 DTO、参数/body 校验、8 MiB 分帧；全部操作 Socket/HTTP 对照及 SSE | 后续版本兼容演进 |
| rove-sdk | 本机/远端连接、订阅/恢复、上下文/身份校验、QR、TUN 限定 transport | 跨平台网络适配验收 |
| rove-agent | 私有 SQLite、身份/分享、Rig、调度、Linux EasyTier 启停/发现、限定 TUN 的 HTTP/SSE/服务代理、地址变化及故障恢复 | 其他平台网络运行时、任意策略路由处理 |
| rove-cli | 全部命令组、通用 call、交互菜单、QR、JSON/stdin、远端 watch 和服务目录 | 平台服务管理 |
| rove-gui | Vue 全部面板、SDK 事件桥、分页、浏览器入口；Linux Tauri 开发 deb | 真实 WebView/相机/多窗口/浏览器验收、其他平台安装包、移动核心 |
| rove-config-server | SQLite 密文存取、7 天 TTL、小时清理、body 与存储上限 | 实际公网运维验收 |

未实现操作返回明确错误；不提供未经 overlay 限定的 agent HTTP 执行入口。GUI 和 CLI 设置的并发值已驱动同一本机 AI 调度器。

## 平台验证范围

| 环境 | 当前证据 | 声明 |
| --- | --- | --- |
| Linux x86_64 | Rust 本机/协议/分享测试；CLI+agent 开发运行；Vue 类型与构建检查 | 开发基础可运行，不是完整 Linux 发行版 |
| Linux 原生 GUI | 原生开发构建、Tauri deb、隔离安装/动态库检查；MockRuntime + 真实 socket agent 测试 | 不等于真实 WebView、桌面运行或 systemd 生命周期验收 |
| Windows | 当前账号 DACL 的 named pipe 服务端及客户端；agent/CLI 与测试交叉编译、Clippy 通过 | 未实际运行，未验证多账号拒绝、服务安装与数据目录 ACL |
| macOS | Unix socket 公共代码 | 未编译、未运行、未打包 |
| Android / iOS | Tauri 壳入口 | 无 native VPN、无后台运行、无嵌入 agent 验证 |

当前测试包含同机私有数据目录及两个独立网络命名空间的真实 overlay 容器，不等同于物理多设备或跨平台验收。

## 依赖验证入口

- Rust 1.96.0、Node 22.22.2、npm 10.9.7 已核实本机版本。
- Vue 3.5.42、Vite 8.2.2、Tauri JS API 2.11.1 来自官方 npm registry；实际解析依赖由 lockfile 固定。
- Rig 0.42.0 已固定为运行依赖。使用低层 CompletionModel 流接口，Rove 控制自己的历史、持久化、取消和顺序工具执行。可控本地 HTTP 模型测试已通过，不等同于真实付费模型验收。
- 本机 EasyTier core/cli 为 `2.6.4-8428a89d`，对应发行 tag `v2.6.4`、commit `8428a89d2dabc94c97d370ec607c6ca142473626`。固定发行归档中的 core/cli 已纳入 Linux 开发包；未修改现有服务。

EasyTier 2.6.4 的 `core.rs` 创建一个 NetworkInstanceManager，`rpc_service/api.rs` 在同一本机 RPC server 上注册 WebClientService 的实例管理服务。创建/停止/枚举接口可从源码确定，但 CLI 没有直接的实例 CRUD 命令，需要对接上游专用 protobuf RPC 客户端，不能把该端口当 HTTP/gRPC。

特别注意：该版本的 RPC portal 可绑定物理网卡；Rove 适配显式要求 `127.0.0.1:<port>` 并限制本机管理访问。专用临时进程已验证两个实例的创建、运行状态、空 peer 列表与删除；两实例均无虚拟地址，不把实例运行当作跨设备网络已连通。

已添加可选 `easytier` feature 和独立 `easytier_probe`，使用精确 git revision；已解决与 Tauri 的传递依赖解析，适配器编译检查通过。构建环境安装了 `protobuf-compiler`（3.21.12）和 `libprotobuf-dev`，用于上游 protobuf 代码生成，未改动现有 EasyTier 服务。

实际 RPC 验证发现必须启用上游 `zstd` feature：只验证短响应会漏掉较大运行状态响应的压缩兼容问题。已加入该选项，未引入 TUN 等网络数据面 feature 到管理客户端依赖。早期完整构建受磁盘 I/O 阻碍，后来最终配置的构建和进程验证均已通过，详见下方时间顺序记录。

可重跑管理入口检查（脚本校验 core 版本，启动专用临时进程，始终停止该进程，不管理已有服务；不依赖地址设计决策）：

```sh
cargo build --locked -p rove-agent --features easytier --example easytier_probe
python3 scripts/check-easytier.py
cargo test --locked -p rove-agent --features easytier --test easytier_contract
cargo clippy --locked -p rove-agent --features easytier --example easytier_probe --test easytier_contract -- -D warnings
```

此前地址管理阻塞已于 2026-09-09 撤销：用户明确每设备同时只加入一个网络，原版 EasyTier DHCP 符合地址管理方向，不需要指定地址池或上游补丁。正式 Linux 网络启动已接线；见 [地址管理边界](easytier-addressing.md)。以下历史记录中的“等待地址决策”或“尚未接通”描述不代表当前状态。

官方参考：

- [EasyTier v2.6.4 core](https://github.com/EasyTier/EasyTier/blob/v2.6.4/easytier/src/core.rs)
- [EasyTier v2.6.4 RPC server](https://github.com/EasyTier/EasyTier/blob/v2.6.4/easytier/src/rpc_service/api.rs)
- [EasyTier 实例管理客户端](https://github.com/EasyTier/EasyTier/blob/v2.6.4/easytier/src/rpc_service/remote_client.rs)
- [Rig 0.42.0 流接口](https://docs.rs/rig-core/0.42.0/rig_core/streaming/index.html)

## 检查与后续

开发检查纳入 `.github/workflows/check.yml`：Cargo fmt/test/clippy、两份 OpenAPI 合规与示例检查、Vue 类型检查与 Vite 构建。CI 文件已创建，不把未执行的远端 CI 标为通过。

此前基线 Rust 测试通过 18 项（包括 5 项真实 Rig 接可控 HTTP 模型的 AI 集成测试）；Vue 类型检查与生产构建通过。不包含 EasyTier 可选测试、原生 GUI 或真实外部模型验收。已覆盖模型失败后推进下一作业、调低并发不取消正在运行的作业。

后续本机通信与容量回归增加 8 项测试，当前默认成员合计 26 项通过：

- `FrameReader` 持久保存半帧读取进度；SDK 在等待被取消后继续读取，不丢失前缀、正文或 UTF-8 边界。另有 SDK 集成测试覆盖半帧等待被取消后立即 unsubscribe，并正确读取后续确认。
- SDK 按最后已处理序号重连，过滤重复事件，拒绝序号跳跃与错误作业身份；终态 204 不重提作业。
- unsubscribe 必须收到确认；断线不再被误报为取消订阅成功，也不取消 run。
- 同一 socket 可同时订阅两个 run；取消一个或从其他连接发送订阅 ID 不影响另一个订阅或作业。重复取消保持幂等。
- 慢请求期间重复活动 correlation_id 返回 409，原请求仍保留；无效 operation/body 不关闭正常连接，其他请求可以先完成。
- 1,024 条队列满时新提交返回 429，同一 request_id 重试仍返回原作业，冲突内容仍拒绝；取消等待项释放容量，失败提交不留下半条记录。
- 队列容量改为数据库计数，不再为每次提交将全部历史作业解析到内存。

默认 Clippy、Cargo fmt、OpenSpec 严格校验、两份 OpenAPI 校验均已通过；OpenAPI 覆盖 36 个操作、313 个引用、21 个内嵌示例和 26 个正反例。任务进度为 25/71，未勾选完整 EasyTier 多实例运行验收。本轮再次尝试 EasyTier 完整构建，仍长时间等待磁盘；已停止该次构建以完成本机回归，没有修改底层源码或已批准的地址契约。

后续先确认 EasyTier 地址管理方案，再按 tasks.md 完成真实 RPC 验证、剩余队列/协议边界测试、远端 SSE、overlay 入口约束和 TCP 代理，最后补完整 GUI/CLI 与各平台包。开发数据包含明文入网凭据和模型配置，须保持私有目录；稳定 device_id 本身不是密钥。

## 本轮增量验收

默认 Rust 测试现为 **41 项通过**，Clippy 和 fmt 通过；另有 2 项前端事件 reducer 测试、Vue 类型检查/生产构建，以及真实 PTY 交互测试通过。OpenAPI 检查新增 [验收映射](acceptance-map.json)：全部 11 FR、9 NFR、16 个故事、7 份规格、36 个操作和证据文件引用受到一致性检查。映射完整不等于需求全部完成。

- QR 使用固定 `qrcodegen 1.8.0`，独立 `quircs 0.10.3` 解码实际 HTTP 分享结果后导入两个设备数据目录；过期返回 410，既有配置保持不变。GUI `jsQR 1.4.0` 相机流程尚未实机验收。
- CLI 子命令验证网络分页、stdin 手动导入、显式更新、密钥脱敏、设置和服务调用；目标不可达不修改本机配置。PTY 测试验证菜单、隐藏密钥和退出后 agent 仍运行；无 TTY 不等待。
- `system_exec` 验证当前 UID、默认/显式目录、stdout/stderr、退出码、超时、预取消、UTF-8 分块、输出截断、不可执行文件、包锁和端口竞争。后两者是测试专属资源冲突，不执行真实包安装。
- 服务目录/发布/PUT/取消发布与 Rig 共用同一实现。5 项测试使用测试专属回环入口验证真实 TCP 双向转发、端口/数据库失败资源释放、失败 PUT 保留旧发布、服务身份/网络保留、幂等取消、目标应用继续运行，以及重启不展示失效入口。
- 服务生产地址表仍为空，只能由未来的真实网络适配填充；没有通过 JSON 配置、UI 或公网接口设置监听 IP 的旁路。正式发布仍报 `overlay_unavailable`，不将回环测试当作 overlay 隔离或自动恢复验收。每代理最多 128 条连接、连接目标超时 5 秒，目标健康状态目前明确为 unknown。
- GUI 批次读取真实 socket 事件，最大 64 条/批、750ms 等待；握手最多 5 秒，无持久桥接后台任务。切换会话/目标使旧响应失效；序号缺口或断线重新获取快照，不重提 run。GUI 每次最多观察 8 个作业，保留有界 Unicode 输出尾部。
- SDK 远端 JSON 客户端拒绝上下文冲突、重定向、不兼容协议和错误设备身份；hello 限制 64 KiB，业务响应限制 8 MiB。仅回环 HTTP 假服务验收，不代表实际远端入口已开放。
- 新增 Unix SIGTERM 正常退出路径，退出会关闭代理并取消受管理作业；平台系统服务注册仍未交付。

再次运行 `cargo check --locked -p rove-gui` 停在 GLib/GObject/GIO 的系统库检查；尚未完成原生壳 Rust 编译。没有为此升级主机图形驱动，也未修改现有 EasyTier 服务。CLI/GUI 代码扩展与原生支持矩阵分开记录。

后续 SDK 远端 SSE 增加 5 项检查，默认 Rust 合计 **46 项通过**；Clippy、fmt、Vue 构建及 2 项前端测试再次通过。远端订阅不再套用 JSON 请求的 30 秒总超时；握手有限时，事件帧上限 8 MiB，分块/UTF-8/CRLF 状态在取消等待后保留。恢复保持网络/设备/run 目标，过滤重放、拒绝序号缺口或 id 不匹配；410 明确要求快照，终态/204 停止观察。所有 HTTP/SSE 验收仍是受控回环服务。

EasyTier 最终 `zstd` 配置的完整 probe 二进制随后在 1 分 59 秒内构建成功。独立脚本实际创建了两个实例，均返回 `running=true`、`peers=0`、`virtual_ipv4=null`，完成查询后删除实例并停止专用进程；没有管理既有服务。最终配置的 `easytier_contract` 测试也通过，再次确认 DHCP 丢弃指定 CIDR。任务 1.2 已完成，但未证明分配地址或跨设备互通，正式启动仍等待地址设计决策。

服务网络生命周期 hook 已补齐，并用回环测试验证原端口恢复、端口冲突、地址变化和旧入口关闭。新地址建立前先退役全部过时监听器，避免中途持久化失败留下旧入口；现有正式网络适配仅调用离线分支。EasyTier 在线/地址变化回调的集成和真实自动恢复仍待完成。

Linux 原生编译阻碍已通过开发专用容器绕开，主机系统库没有升级。Tauri dev/custom-protocol 检查通过，GUI 测试二进制实际链接成功；1 项 MockRuntime 集成测试使用生产 Tauri 配置、两个窗口及真实临时 socket agent，验证配置共享、目标不可达不本机执行、事件参数/序号、取消、QR 和关窗后 agent 持续运行。远端页面来源、文件打开、脚本 URL 和指定任意 OS 程序均被原生权限边界拒绝。它不替代真实 WebView、多窗口交互或浏览器运行验收。

前端现有 6 项测试通过，并完成 Vue 类型检查/生产构建：新增分页合并和服务地址校验。定时刷新只更新最后加载页，保留此前页和新提交，按创建时间/ID 去重排序；快照获取最多 8 个同时请求。服务“浏览器打开”接入固定 Opener 插件，纯 TCP 只提供地址复制。Tauri 桥接持有 SDK client 状态，不持有 agent 或数据库。

## 2026-09-09：取消隔离与 CLI 收尾

- `system_exec` 不再忽略直接子进程终止失败；只有确认已经退出时才忽略终止请求的错误，否则追加到 `residual_processes`。
- 新增真实进程测试：分别取消和超时终止 shell 的受管理后代，确认它们的 TCP 监听关闭；另一独立进程组的服务仍能收发数据，测试应用数据保持原样，agent 关闭后该服务也继续运行。所有服务仅监听测试专属回环端口，测试显式清理自己的子进程，不操作系统已安装服务。
- 结合已有 Rig 模型取消、跨作业隔离与后续会话推进测试，任务 7.4 的 Linux 验收完成。Windows 仍明确报告后代停止需要 Job Object 集成，未把它标为 Windows 进程树终止已实现；跨平台构建、安装与运行验收仍未完成。
- CLI 新增 `service open`：读取所选设备的服务目录，只将当前发布的 HTTP/HTTPS 入口交给客户端本机默认浏览器；无图形会话、非法协议、内嵌凭据和无效入口明确拒绝。结构化输出区分 OS 打开请求与真实连接成功；地址校验和既有 CLI 集成检查通过。
- 当前环境的沙箱禁止 socket 监听，首次未提权测试因此无法就绪；获得测试执行权限后，同一新增测试及 CLI 回归通过。没有修改网络策略或系统服务配置。
- 最终默认成员 Rust 测试 **50 项通过**（含 2 个测试子进程入口），Clippy、fmt、OpenAPI/验收映射及 OpenSpec 严格校验通过；前端测试通过。此前原生 MockRuntime 验证记录保留，本日未将其算入默认 Rust 测试数。

## 2026-09-09：本机客户端等待期限

SDK 普通调用将连接/hello 限为 5 秒，发送业务请求与响应读取限为 30 秒。订阅的连接、hello 与打开确认共用 5 秒，包括 CLI `run watch` 和 GUI；成功订阅后的事件等待不套用整个作业的总超时。无效订阅参数在连接前校验。

超时只结束客户端等待，不自动重试、不发送取消。握手超时明确表示尚未发送业务请求；业务超时说明结果未知，提示提交作业时继续使用原 `request_id`。不在错误中回显模型密钥或输入内容。SDK 验证程序覆盖无响应 hello、业务响应丢失、订阅确认丢失及握手之后长时间空闲仍可收到事件，检查断线前没有额外的重提或取消帧。

本次属于任务 3.3/3.5/9.4/10.1 的通信健壮性补充，不将 EasyTier、Windows named pipe 服务端或实际远端功能标为完成；任务进度仍为 35/71。

当前工作区默认成员 Rust 回归 **55 项通过**；最后调整超时提示与半帧测试后，SDK 的 17 项测试及默认成员 Clippy 再次通过。fmt、OpenSpec 严格校验和 OpenAPI/验收映射检查通过；契约检查覆盖 36 个操作、313 个引用、21 个内嵌示例及 31 个正反例。开发容器中的原生 GUI MockRuntime/socket 集成测试另有 1 项通过，不计入默认成员测试数，也不代表真实 WebView 或安装包验收。本轮未改动前端，沿用此前前端验证记录。

## 2026-09-09：工具平台与资源边界

任务 7.5 完成，进度 36/71。能力声明和 `system_exec` 使用同一个内部平台判断，仅 Linux/macOS/Windows 提供系统命令工具；Android/iOS 及未接入平台返回契约定义的 `unsupported`。平台选择来自编译目标，不能由请求覆盖。

新增测试经实际执行入口模拟不支持的平台，验证不创建命令标记文件、不生成输出事件、不改变作业快照，且返回值符合 OpenAPI。连同已有的输入长度/超时上限、输出截断、真实不可执行文件、测试专属包锁与端口竞争、取消隔离测试，工具模块 9 项测试通过（含 2 个子进程入口）。这是主机上的平台分支验证，不代表 Android/iOS 壳或 Windows/macOS 实机验收；相关平台任务继续保留未完成状态。

本轮最终默认成员 Rust 测试 56 项通过，Clippy（warnings denied）、fmt、OpenAPI/验收映射及 OpenSpec 严格校验通过。没有修改对外接口、EasyTier 源码或现有系统服务。

## 2026-09-09：结构化协议与客户端续接

- 任务 2.2 完成：`rove-protocol::dto::{agent,config_server}` 在构建时由两份 OpenAPI 生成全部组件模型、枚举、联合与嵌套结构；业务 UUID 字段引用 `rove-core` 的稳定 ID 类型。字段缺省和显式 null 分开，必填可空字段不能缺省。模型不派生 Debug，避免凭据被随手记录。`WireDto::from_value/to_value` 保留 JSON Schema 对范围、格式和条件分支的最终校验；直接 serde 转换只负责结构，不替代契约检查。`system_exec` 已使用生成输入模型。
- DTO 测试覆盖两份契约内嵌示例及共享正例无损往返、负例拒绝、必填/null/资源限制；不通过修改 wire schema 迁就生成器。新 schema 中未支持的结构会在生成阶段明确失败。
- 任务 6.1 完成：真实 Unix socket 上第一个 SDK 客户端创建会话并提交作业，断开后作业完成；另一个客户端查询同一会话与历史、继续提交，模型收到按顺序保留的先前输入/回答/新输入，重启后两次输入仍可查询。使用受控模型 HTTP 服务；不冒充跨设备 overlay 验收。
- 本阶段默认 Rust 测试 59 项与 Clippy 通过。实际跨设备入口、平台安装与移动端验收保持独立未完成项。

任务 9.2 随后完成：增加实际 CLI 子进程→SDK→socket agent→Rig→受控 HTTP 模型的整段验收，覆盖模型设置、会话创建/更名/查询/列表、stdin 提交、事件 JSON 行、快照、原 request_id 重试、历史读取、另一 CLI 进程续接、取消和作业过滤。结合已有并发设置和目标不可达隔离测试，CLI 交互能力验收完成；不代表实际远端转发已交付。

## 2026-09-09：发行来源、Windows 接入与设备选择

- 任务 1.1 完成：固定六个平台/架构的 EasyTier 官方发行资产、大小、SHA-256 和源 revision；重新下载并校验 Linux x86_64 ZIP，core/cli 成员哈希与现有测试二进制一致。校验脚本纳入 CI，未安装或执行下载内容。详情见 dependencies.md。
- Windows named pipe 服务端设置当前账号 SID 的受保护 DACL、拒绝远程客户端、首次实例独占名称，每个连接复用同一 socket 协议处理器。SDK 管道忙时重试打开但受握手期限约束。Rust Windows GNU target 与现有 MinGW 完成交叉编译，agent/CLI 全目标 Clippy 通过；Windows 原生 CI 已定义但尚未执行，任务 3.1 保持未完成。
- 工作盘空间紧张，将本轮新生成的 Windows 交叉编译缓存迁到 `/tmp/rove-cross-build-OkL4iR/x86_64-pc-windows-gnu`，原 target 子目录保留符号链接。没有删除源码、用户数据或其他构建目录；该临时缓存不属于发行产物。
- Vue 新增本机网络→设备列表选择，支持两级分页与显式刷新；切换网络清空旧设备列表，卸载/过期响应不更新界面。离线与不兼容分别展示，缺少 system_exec 只表示该能力 unsupported，在线设备仍可选择；网络 ID 不匹配不能改变目标。后端真实发现尚未接通，任务 10.3 保持未完成。
- 新增 ui-parity.json，逐项对应全部 33 个 agent 操作的 GUI/CLI 入口及界面等价动作，契约检查要求端点集合与源码引用一致。入口映射不是完整端到端验收，任务 12.4 保持未完成。
- DTO 保留 JSON Schema 的整数语义（包括 1.0）、原始整数表示、显式 null 和缺省差异。没有通过收窄 JSON 契约迁就 Rust 基础类型。
- 原生 MockRuntime 回归发现取消测试的时序假设：合法响应可为 200（已终止）或 202（正在取消）。按原 OpenAPI 修正断言，增加有限时的终态查询，不改变取消实现。

本轮最终验证：默认 Rust 60 项通过；Linux Clippy、Windows GNU agent/CLI 全目标 Clippy、fmt、8 项前端测试、Vue 类型检查/生产构建、原生 MockRuntime/socket 测试 1 项通过。OpenAPI/验收映射、发行资产清单及 OpenSpec 校验另行运行。未运行真实 Windows/macOS、移动壳、真实外部模型或两设备 overlay；进度 40/71，不宣称完成剩余 31 项。

随后完成任务 10.4，进度 **41/71**：新增 Tauri MockRuntime 两窗口→SDK→真实 socket→Rig→受控模型测试。两个独立会话必须在任何模型响应放行之前都进入模型执行，证明窗口间不串行阻塞；首个窗口取消自己的作业，两个窗口关闭后，第二个作业仍成功完成并可从 SDK 查询输出。结合 Vue 的排队状态、分页与输出恢复测试，完成交互/桥接逻辑验收；这不是实际 WebView 视觉或桌面窗口管理验收。原生测试现为 2 项通过。

因新原生测试的依赖组合扩大构建缓存，工作盘一度满。将完整 8.9 GiB `target` 缓存移至 `/tmp/rove-build-cache-qel3Ol/target`，工作区原路径改为符号链接；所有构建产物保留，没有删除代码或用户数据。原生检查脚本只额外挂载这个专用 target 目录，不挂载其父目录。六份 EasyTier 固定归档随后均下载并通过大小/SHA-256 校验；仍未执行或安装其他平台程序。

## 2026-09-09：HTTP/SSE 服务端协议适配（部分完成）

`rove-agent::http` 根据 OpenAPI 的全部 agent 路径/方法建立路由，复用 agent 请求校验和业务逻辑。每个入口带固定网络上下文，校验网络 UUID 和设备 UUID，hello 返回该入口的 network_id；错误目标不会执行或转发。JSON 请求体限 8 MiB、读取限 5 秒；重复查询参数、错误内容类型、非法 JSON、缺省/null 按各自语义处理。普通响应 no-store。

SSE 发送持久化 RunEvent，支持 after_seq/Last-Event-ID、一致性检查、终态 204、游标超前 409、历史过期 410；流式响应 no-cache。断线不取消 AI 作业，网络停止令牌结束事件观察。SDK JSON 和 SSE 都已对接这个真实业务处理器完成回环测试，不再只用伪响应服务器验证协议。

Linux 的唯一公开监听函数要求 TUN 类型、网卡实际拥有该 IPv4，并同时绑定 IP 和网卡；校验或绑定失败直接返回，不回退任何物理/通配入口。[Linux SO_BINDTODEVICE 语义](https://man7.org/linux/man-pages/man7/socket.7.html)说明该选项约束接收网卡。此函数尚未被生产网络启动流程调用；调用方必须在移除/重新配置网卡前取消旧入口。其他系统未开放这个实现。测试用回环 listener 仅存在于私有测试模块。

新增 3 项测试覆盖 SDK 会话往返、错误上下文无副作用、参数错误、真实 SDK SSE 续读、终态/过期/超前游标，以及生产函数拒绝回环和非 TUN。尚缺 EasyTier 生命周期接线、真实 TUN 成功绑定/物理入口拒绝、连接资源配额与其他平台适配；任务 4.5/4.8/12.5 保持未完成，总进度仍为 **41/71**。

最终默认成员 Rust 测试 **63 项通过**，Linux 全目标 Clippy、Windows GNU agent/CLI 全目标 Clippy、fmt、发行资产清单与 OpenSpec 严格校验通过。前端 8 项及原生 GUI 2 项测试沿用本轮前述成功记录；HTTP 模块只在 Linux 编译。契约脚本须使用 README 指定的 `.venv/bin/python`，系统 Python 未安装校验依赖。

## 2026-09-09：纠正个人多网络与单设备单网络

用户再次明确每台设备同时只加入一个网络。已同步 proposal、design（进程/关系/风险模型）、requirements、US-04、overlay-network 规格及任务 4.4/12.3；不再以同机多网络地址重叠阻塞实现，不维护 EasyTier 补丁。既有多实例探针是上游能力检查，不是产品行为。

OpenAPI 将旧 ipv4_cidr 改为可选、已弃用的开发版兼容元数据；新建配置不生成它，CLI 去掉指定地址池选项，EasyTier 探针不传 virtual_ipv4/network_length。旧载荷及加密向量保持原值，未修改已有数据库或分享凭据。GUI/CLI 明示保存与加入的区别，以及先停止再切换。

网络生命周期增加互斥，启动检查其他记录的 enabled/state；另一网络未确认退出时返回 409 network_already_joined，包括 starting/running/stopping/failed。未接入适配器时不假称退出成功或释放加入资格。新增测试覆盖无地址池的新配置、重复导入、稳定身份、多份保存与重复启动拒绝。测试内注入状态仅验证保护逻辑，不算真实网络切换验收。

默认 Rust 回归 64 项通过；首次沙箱内测试因禁止 socket 失败，获准执行测试自有本机监听后全部通过。EasyTier 可选兼容测试 1 项通过，OpenAPI/映射校验、OpenSpec 严格校验及前端测试（4 个测试文件）和生产构建通过。任务 4.2/4.4 的实际启停、地址与入口验证仍未完成，进度保持 41/71。当前没有待用户确认的地址架构决策。
