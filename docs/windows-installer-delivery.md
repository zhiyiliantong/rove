# Windows 安装包与 240 实机安装、Android 网络测试

## 2026-10-08 r27：运行时观察重试与页面资源打包修复

Windows 240 已覆盖安装并启动 r27。修复共享生命周期将临时状态观察失败永久禁用的问题：保留 enabled，撤下未确认入口，按 1/2/4/8/16/30 秒退避重查；显式停止取消恢复，权限/隔离等安全校验不跳过。旧 enabled=false 状态不擅自迁移，不自动重放作业。旧跨设备 target_unreachable / Mihomo 调查按用户要求暂停。

r26 Windows 安装后用户报告 `asset not found: index.html`，确认其二进制未嵌入当前前端资源。此前进程及 agent 正常不代表 GUI 可用。疑似 Windows 资源嵌入与 Android 前置构建清空/重建共享 dist 的竞争；串行重编译恢复。r27 增加构建入口检查和打包前当前资源键校验；不再把前端重建与原生嵌入并行。r26 不作为有效 Windows 交付。

| 制品 | 字节 | SHA-256 |
| --- | ---: | --- |
| `target/packages/rove-windows-x86_64-setup-20261008-r27.exe` | 79,795,756 | `0add09d617500d1ecb5f69e4bd48fd91a12c5f9045a924d4c5651ce48b5850cf` |
| `target/packages/rove-android-arm64-debug-20261008-r27.apk` | 234,293,035 | `257f27171341ed587c6303da41e822526b2875fa084adb6290684677ed1c4daa` |

Windows 安装器 exit 0，六个文件哈希一致；GUI SHA-256 `5d9a47e0f55724ee9bef6766a8944a739f10b5e839d7db3516c8cf8c8647f705`。真实安装窗口已通过 CDP 验证：组件已渲染、`index-DK9pXVfE.js`、CSS、字体及回复组件资源全部 200，原生 `get_device` 返回 200 且为原 Windows 身份。实际 1100×760 截图已检查（`/tmp/rove-installed-windows-ui.png`）。最初验收脚本的额外 fetch 被生产 CSP 正常拒绝，改为观察真实资源请求，未放宽 CSP；另修正脚本误用 status 而非 status_code 的断言。临时调试转发和监听已关闭，GUI 以正常方式重新打开。

15:41 实机验证 `stopped → running → stopped`，等待后仍 enabled=false/stopped；再显式启动恢复 running / 10.203.240.3。安装前后无活动作业，原设备 ID、12 个会话 ID、3 个网络 ID 及三份入网配置哈希一致，其他两个网络仍停止。未卸载清数据、未修改网络 key/初始节点或系统代理/路由策略。此为正常原生启停，不是注入原生状态观察故障后恢复的实机证据，也不代表本轮跨设备组网验收完成。

Android r27 构建及 v2 Debug 签名验证通过，构建检查不改变运行内容，因此与 r26 APK 字节哈希一致；当前 JS/CSS 资源键均存在。原授权 242 Redroid 当前无法接入，未覆盖安装、未验证 Android 原生恢复；240 新连接的实体手机不是原目标，未擅自安装。仍为 Windows 未正式签名 / Android Debug 测试包。

共享 agent 85/85、生命周期专项 8/8、GUI 单元 31/31、恢复提示四尺寸浏览器交互 4/4、打包资源校验测试、57-operation 契约和 OpenSpec 严格校验通过。浏览器截图不是 Android 原生验收。详细边界见 `openspec/changes/local-session-ownership/runtime-recovery-verification.md`。

## 2026-09-29 r24：编辑后重新生成网络名片

Windows 240 与 Android 14 arm64 Redroid 已覆盖安装并启动 r24。每次点击网络名片重新读取最新保存配置；编辑保存、离开视图、组件卸载时使旧 Base64、二维码和复制状态失效。异步生成以序号校验，旧请求晚返回不能覆盖新网络名片；加载期间显示加载状态，不闪现读取失败。二维码与复制文本使用同一个 Base64 快照。规则已同步 AGENTS.md 与 OpenSpec。

| 制品 | 字节 | SHA-256 |
| --- | ---: | --- |
| `target/packages/rove-windows-x86_64-setup-20260929-r24.exe` | 79,784,509 | `507971aea3d85326a6604fafb36e55187947e3700084904884234037922319b7` |
| `target/packages/rove-android-arm64-debug-20260929-r24.apk` | 234,191,051 | `4a3b89d2b19a918f407dca1be760b8049dd9d3dfc185da9e016f2e12fba7b7d4` |

Windows 安装器 exit 0，六个运行时文件哈希匹配；GUI SHA-256 `a99d39aa234ceec0921697012fa0d11abc3a3364012391dd76b5f964d9363cfb`，agent/helper/CLI 未变。Android `adb install -r` Success，冷启动 Status ok，v2 Debug 签名校验通过。两端实际前端 `index-CWpqC_cX.js`。仍为 Windows 未正式签名/Android Debug 测试包。

GUI 单元 30/30、完整浏览器 105/105（4.9m）、名片专项复测 5/5（等待入场动画完成后截图）、构建、57-operation 契约及 OpenSpec 严格校验通过。四尺寸的名片、回退、滚动、复制栏和横屏分栏截图已查看。浏览器二维码 IPC 为测试桩，不能代替真实编码验收。

真实安装版两端创建 stopped 临时网络，先打开旧名片，再编辑名称/初始节点、保存并重新打开。Base64 与二维码均变化，解码 Base64 等于后端最新 JoinConfig；将实际原生返回的二维码 SVG 栅格化后用 jsQR 解码，所得文本与最新 Base64 完全一致。两端临时网络均保持 stopped，并在验收后删除。Windows 1100×760、Android 360×568 原生截图已查看，二维码、回退和复制栏可见；这不是实体相机扫描或真实软键盘验收。

安装前无活动作业；Windows 原 12 个、Android 原 18 个会话、身份、模型/连接/默认模型、网络 ID/启停状态及本机别名与本轮基线完全一致。未卸载/清数据，未修改真实网络 key/初始节点，未调用付费模型。两端原网络恢复 running，应用保持打开。为缓解构建空间，将已有 r21 APK 移到 `/home/git-ssd/rove-build-artifacts/` 并保留原路径符号链接，原 SHA-256 不变；r24 APK 同样存放在那里。

重连调查仍独立未完成：发现 Windows helper 使用 Mihomo 的 198.18.0.1 作为 bootstrap TCP 源，同时存在 Android 宿主到 Windows 物理地址的连接。未改网络策略时，17:36、Windows 安装重启后的 17:49 双向真实目标 get_device 均恢复 200，但不能将间歇恢复当作永久修复。已向用户询问是否允许仅 Rove 组网优先物理网卡、避开第三方 VPN；尚未收到确认，未实现或应用此出口策略，任务 5.2–5.3 保持未完成。没有修改系统代理、Mihomo、路由或防火墙规则，也未宣称本轮完整组网/永久重连验收完成。

收尾 17:59 双向目标 get_device 也返回 200；这只说明当前通信恢复，不是已实施永久修复。证据：`/mnt/data/rove-deliveries/2026-09-29-r24/`，包含名片更新/二维码解码、原生截图、基线比对、临时网络清理与分阶段对端调用结果。

## 2026-09-29 r23：网络编辑与保存自动重启

Windows 240 与 Android 14 arm64 Redroid 已覆盖安装、启动 r23。移除网络设备列表“管理此设备”和设置中的全局执行目标入口，远程操作保留在会话；本机设备名称仍可原位编辑。网络详情提供“编辑网络”，复用创建表单；key 只读、可显示，服务端拒绝修改（400），修改显示名称不改变 EasyTier 身份名称。

运行中保存不再要求手动停止。按钮显示“保存并重启网络”，提前提示“保存后将自动重启此网络，连接会短暂中断”。Windows/Android 共用一个流程：重新读取状态、停止并等待 stopped、写入配置、启动并等待 running；原本 stopped 的网络只保存不启动。停止失败不写入，保存失败/结果未知不盲目启动，已保存但启动失败明确区分，不把 202 当作完成。AGENTS.md 和 OpenSpec 已按用户最新要求更新。

| 制品 | 字节 | SHA-256 |
| --- | ---: | --- |
| `target/packages/rove-windows-x86_64-setup-20260929-r23.exe` | 79,787,738 | `fd463532c08860bbed42a64201749e3a775fe62788cb9b61d5458d35ca7e6c07` |
| `target/packages/rove-android-arm64-debug-20260929-r23.apk` | 234,191,051 | `3ff29a3f042a956dce8c43601958e57d8644f65b43c57e909c4d0b1a1daceee0` |

Windows 安装器 exit 0，安装后六个运行时文件哈希匹配；GUI SHA-256 `958349b9a4b16cef19f7bceee43d72dc43ca583c66835ff9b4eea338491ed845`。Android `adb install -r` Success，启动 Status ok，v2 Debug 签名校验通过。两端实际加载 `index-aUDOSwzD.js`。仍是 Windows 未正式签名/Android Debug 开发包。r22 是中间构建：Windows 曾安装，Android 未安装；最终均为 r23。

验证：GUI 单元测试 30/30（含 6 项生命周期测试），生产构建通过，浏览器完整回归 104/104（单 worker，4.6m）；四尺寸 1100×760、320×640、390×844、844×390 截图及交互已查看。初跑缺 Chrome 路径；随后并行运行因磁盘 I/O 导致 page.goto 超时而中断；单进程专项发现测试预期未考虑刷新清空调用记录，修正断言后全量通过。没有将这些失败记为通过。共享 agent 网络专项 4/4（r22 同轮后端，r23 未改后端）、57 operations 契约检查和 OpenSpec 严格校验通过。

安装版两端实际点击保存：独立读取到 `running → stopping → stopped → starting → running`，原网络配置（含 key、初始节点、本机 IP）不变，分别恢复 10.203.240.3 / 10.203.240.2。原生 Tauri invoke 不可改写，最初拦截式测试无法记录调用、断言失败；改用实际状态采样重新验收通过。另用临时 stopped 网络验证名称/初始节点保存、重新打开持久化、key 替换返回 400、保存后仍 stopped；两端临时网络随后删除，未删除用户网络。原生截图 Windows 1100×760、Android 360×568 已查看。

安装前无活动作业。Windows 原 12 个、Android 原 17 个会话全部保留，身份、模型/连接/默认模型、网络 ID/启停状态和别名与基线一致；Android 验收期间另出现 1 个非测试脚本创建的新会话，原样保留，未为匹配基线而删除。Windows 本机别名为用户最新的 `windows`，不是历史 `aa`。未卸载/清数据或调用付费模型。

限制：截至 17:12（Asia/Shanghai）两端网络均 running，但双向目标 get_device 仍返回 503 target_unreachable。因此本轮只确认自动重启与编辑通过，**尚未完成本轮 Windows–Android 组网及永久重连修复验收**，任务 5.2–5.3 不勾选。未修改 Mihomo、系统代理、路由或防火墙。Redroid 不是实体手机，未重新验收真实软键盘/全部协议。

证据：`/mnt/data/rove-deliveries/2026-09-29-r23/`，含前后数据、实际重启状态、编辑/提示截图、临时网络清理及目标不可达记录。为缓解构建盘空间，仅把本轮 r20/r22/r23 APK 制品存于 `/home/git-ssd/rove-build-artifacts/`，原 target/packages 路径保留符号链接、哈希不变。

## 2026-09-29 r21：本机设备别名、名称原位编辑

Windows 240 与 Android 14 arm64 Redroid 已覆盖安装并启动 r21。点击设备名称即在原位置编辑，没有单独表单/弹窗/保存取消按钮；回车或失焦保存、Esc 取消，输入法组合键不提交。AGENTS.md 已记录此交互及“修改后默认继续打包测试”的要求。

| 制品 | 字节 | SHA-256 |
| --- | ---: | --- |
| `target/packages/rove-windows-x86_64-setup-20260929-r21.exe` | 79,764,895 | `dcf6c77dd083dd2f10c0e0483c5abde40e90a621b083f320145f713dcf0bd9fe` |
| `target/packages/rove-android-arm64-debug-20260929-r21.apk` | 234,186,131 | `864c9a9247f0818a79ffcb673a324a36e98e5a4e8d13f6824e24c5c9ce389e15` |

Windows 安装器 exit 0，四个程序与两份驱动哈希一致；GUI SHA-256 `c2cd6e0c1c33898a2a99a45b8ec4f8a7cecf001c600473c52ded4f2a70f8ac5c`。启动脚本第一次枚举进程时 agent 尚未启动，脚本整体返回 1；后续真实 GUI/API 已确认 agent 正常启动，不把脚本返回码描述为全部成功。Android `adb install -r` Success，冷启动 Status ok，v2 Debug 签名验证通过。两端加载同一前端 `index-D8sClFN4.js`。仍是 Windows 未正式签名/Android Debug 开发包。

浏览器原位编辑及同步配置专项 6/6；完整回归 100/100（1.9m），覆盖 1100×760、320×640、390×844、844×390 并查看截图。全量第一次因未指定本机 Chrome 路径而未启动浏览器；使用 `PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH=/usr/bin/google-chrome` 后完整通过。契约 57 operations 和 OpenSpec 严格校验通过；共享 agent 无 native 66/66、有 native 81/81、协议 7/7、GUI 单测 24/24 为同轮此前结果。

安装版：Windows 1100×760 验证点击名称原位聚焦、Esc 取消及用户 `aa` 不变；Android 360×568 验证回车保存、IME 组合键不提交、Esc 取消、实际 ADB 点击编辑区外保存，重新进入页面名称保留。初次未聚焦 WebView 的程序化 blur 没有触发事件；前台触摸聚焦后外部点击保存已确认。两端本机别名解析返回正确 device_id/network_id/当前地址，重名返回 409。Android 临时验收别名已清除，Windows 用户 `aa` 完整保留；没有改写远端名字。未做实体手机或真实中文软键盘完整验收、模型驱动按别名远程部署验收。

安装前确认无活动作业；正常停止测试网络再安装、随后显式启动。Windows 12 个/Android 17 个原会话、两端身份、模型/连接 ID 与默认模型、网络 ID 和启停状态、别名均与基线一致。未卸载/清数据，没有调用付费模型。两端应用保持打开。

网络限制：r20 中曾复现 Windows 自动启动 `overlay_unavailable`，控制重启后显式启动恢复；Android 覆盖安装后需显式启动 VPN。本次 r21 两端启动成功，地址分别为 10.203.240.3/.2，但 15:43（Asia/Shanghai）仅发现自身，双向目标 get_device 仍为 503 target_unreachable。**尚未完成本轮 Windows–Android 组网验收，永久重连修复未完成**；未修改代理、Mihomo、系统路由、防火墙或 bootstrap 策略。原任务 5.2–5.3 保持未完成。

证据：`/mnt/data/rove-deliveries/2026-09-29-r21/` 的基线/最终数据、网络启停/目标错误、原位编辑截图和清理记录。

## 2026-09-29 r18：Windows 中文命令输出；重连路径诊断

`target/packages/rove-windows-x86_64-setup-20260929-r18.exe`：79,750,827 字节，SHA-256 `20654e3537e80821367263ef6583d559a691a054c6499f40b80f8000d3aa79ec`。240 覆盖安装 exit 0，安装器、agent、CLI、helper、GUI 与驱动哈希核对通过。随后为重连取证，仅把 agent 替换为包含同一乱码修复及默认关闭 trace 的 r19 诊断构建（SHA-256 `ff7471054f4266064ba424f29b4b3ec281a4604d56bacb5ef22f7a37a8f1b11d`）；其余安装文件保持 r18，r18 agent 备份为 `C:\Users\haobo\rove-agent-r18-recovery.exe`。这不是已交付的永久重连策略修复。

乱码根因：共享 system_exec 原来无条件按 UTF-8 有损读取 Windows OEM/本地代码页字节。现在共享 stdout/stderr 管线保留正常 UTF-8，Windows 薄适配按 console/OEM 代码页转换，逐行处理并保护多字节分块，长行有界；不改系统语言/代码页。旧损坏历史不重写。

真实 GUI 会话 `ee0769fc-6488-43a3-9deb-3740c6ce88f5`、作业 `85d3608c-f16b-477c-ac6d-c239bd97a893` 的 ping 输出已正常显示“来自 10.203.240.2 的回复”，2/2 回复、3–4 ms；“中文标准输出”和“中文标准错误”在原始工具事件、结果、重新查询的持久记录及截图中均正确，没有替换字符。第一次安装后立即 ping 的 2 次超时也原样保存为失败连通性证据，没有用后续成功掩盖；cmd 复合命令的最后 echo 返回 0 不表示前面的 ping 成功。两份辅助验收会话已归档（可恢复），成功会话保留。3 个临时无密钥模型连接全部删除，原身份、会话、默认型号及模型目录、网络配置保留；没有使用付费模型。

验证：主机 agent 无 native 64/64、有 easytier-native 78/78；Windows 原生解码专项 3/3；新增 trace 脱敏/条目上限测试 1/1；OpenSpec 严格验证通过。Android 保持 r17，未重新构建不支持 system_exec 的 Android 包。

重连复测：未改配置的 r17 两次 Windows 停启分别在 7.044/6.379 秒恢复双向真实 get_device，Android 停启 6.724 秒；Windows 停止 45 秒后，9.481 秒恢复。但 r18/r19 系统选路又复现持续离线，不能宣称永久修复。故障时 Windows 初始节点经 Mihomo 地址 `198.18.0.1`，TCP 反复关闭；trace 记录了仅公共节点连接、Android 经公共节点的 cost=2 路由，overlay ping 与 HTTP hello 不通。指定物理网卡的 TCP 成功只作为诊断，不作为入网证据。

仅针对已验证的 Rove helper，临时阻止其使用 Mihomo 接口做对照（未关闭 Mihomo、未改系统路由/代理、未放宽任何既有防火墙限制）。该条件下三轮停启在 8.524/6.432/4.078 秒恢复双向 get_device；trace 显示物理地址连接公共节点、与 Android 的直接 TCP 和 cost=1 路由。说明出站接口竞争及公共路径稳定性影响重连；不能把一次 TCP 成功当作完整原因证明。所有临时防火墙规则均已删除；删除后 14:32 双向 get_device 仍返回 200。永久默认出站策略须先确认，不静默替用户改变是否使用系统 VPN 的行为。

当前诊断进程通过 `ROVE_NETWORK_TRACE=1` 在私有目录记录最多 4 MiB 脱敏成员/路由事实，正常启动默认关闭。trace、安装前后、乱码前后及 A/B 数据和截图存于 `/mnt/data/rove-deliveries/2026-09-29-r18/`。这是 Redroid 验证，不是实体手机、全部传输协议或全面 VPN 共存验收。

## 2026-09-29 r17：本机会话归属与会话内远端执行

Windows 240 覆盖安装返回 0，安装后四个主程序及两份驱动哈希匹配。Android 14 arm64 Redroid 覆盖安装成功，APK v2 签名验证通过。两端均已启动，原 device_id、全部既有会话、模型目录、默认型号和网络配置保留。

| 制品 | 字节 | SHA-256 |
| --- | ---: | --- |
| `target/packages/rove-windows-x86_64-setup-20260929-r17.exe` | 79,773,201 | `f0ae5cbea869f54c7352276efc615f79193fff1a88058e92504d7a9d9b73df76` |
| `target/packages/rove-android-arm64-debug-20260929-r17.apk` | 234,079,867 | `4dd5f719b85fa7a6e44bd77c82699c2893e12a0e2cfebe16227490e296554cbc` |

真实安装版 GUI 上分别选择自己和对端后新建会话，均由本机保存且没有 execution_target；对端暂不可达也不影响本机创建。Windows 1100×760、Android WebView 360×568，无横向溢出，创建后显示输入框。原始数据未删除；测试辅助会话归档可恢复，成功执行会话保留供查看。

使用临时、无密钥的确定性模型验证真实执行链：Android 会话 `abb116e3-0b1e-4590-9c12-6c7bb38a3d5a` 的本机 agent 通过 EasyTier 调用 Windows agent，创建远端执行记录、提交及查询作业。Windows 作业 `d64319db-c14f-4b7a-bdba-da1c6de8a628` 真实执行 `echo ROVE_R17_REMOTE_EXECUTION`，exit_code=0；Windows 原生工具事件和 stdout 已核对，结果返回 Android 原会话。Android 原作业 `b274a8c6-a89c-4d49-846c-556e698f9ff1` 成功，会话 device_id 仍属于 Android，没有整段会话迁移。

停止 Windows 指定网络后，新 Android 测试会话的 rove_api 返回 target_unreachable；工具事件中没有本机 system_exec，也没有替代远端执行。Windows 物理地址 `10.1.2.240:43190` TCP 探测失败（这是物理 API 不可达的负向证据，不是组网成功证据）。测试后恢复原网络启停状态，并删除两端仅本次新增的临时模型连接，原目录和默认型号完全一致。

限制和诊断：安装后首次对端发现曾延迟数分钟，最终双向真实 get_device 返回 200 后才测试远端任务；没有通过修改 bootstrap 绕过。断网负向测试后重新启动 Windows 网络，截至 2026-09-29 10:31（Asia/Shanghai），两端网络虽为 running、具有 overlay 地址，双方对端仍为 offline，目标 get_device 返回 503 target_unreachable。故重连恢复尚未通过，不能将恢复启停状态描述为恢复通信。Windows 临时模型地址用 `127.0.0.1` 时请求失败，改为 `localhost` 后成功；未声称一般代理兼容问题已解决。确定性模型只验证工具编排、协议和执行链，不验证真实供应商的智能决策。ADB/SSH 转发只承载调试及临时模型端点，agent 间调用使用应用真实 overlay。Android 为 Redroid，不是实体手机；本轮未重新验收实体键盘、相机或所有传输协议。

证据：`/mnt/data/rove-deliveries/2026-09-29-r17/` 的 before/installed/final、UI、execution-final、peer-proof、negative、cleanup JSON 和安装版截图。开发包仍为 Android Debug / Windows 未正式签名版本。

## 2026-09-24 r16：原生 EasyTier 组网交付

本节为 r16 历史结果，当前 r17 结果见上节。Windows r16 已在 240 覆盖安装（exit 0），Android r16 已覆盖安装并启动；两端原 device_id、网络及其他用户配置保留。

| 制品 | 字节 | SHA-256 |
| --- | ---: | --- |
| `target/packages/rove-windows-x86_64-setup-20260924-r16.exe` | 78,471,047 | `984b8706eabed8b00ff56186bab0faa81ba2aa7849b9634e1730b1cc74cececd` |
| `target/packages/rove-android-arm64-debug-20260924-r16.apk` | 234,065,795 | `7a22dc6c94fab868767dafde53f25b1a454399e0a4e846646db06bb409a2dbc9` |

Windows 安装目录仍为 `%LOCALAPPDATA%\Programs\Rove`；GUI 普通权限启动同账户 agent，启动网络时由独立 helper 处理系统授权、Wintun 和受限 WFP/防火墙适配。无需另装 EasyTier 或依赖 Mihomo。包内 Wintun/WinDivert 为校验后的签名组件，Rove 本身仍是未正式签名的开发测试版。

安装后的六个运行时文件哈希均匹配。两端实际加入 `VPN 接入测试`，共享 network_id，Windows overlay 地址 `10.203.240.3`、Android `10.203.240.2`。双方设备列表识别对端，双向目标 `get_device` 返回 200；分别停止任一端后调用返回 `target_unreachable`，清除对应原生接口/监听，不转本机执行。Windows 物理 IP 和经物理接口路由到 overlay API 的负向均失败；临时路由及测试放行规则已清理。最终通信使用两端主动连接公共初始节点，不依赖临时开放 Windows 物理 API 或关闭防火墙。

两端公共目录均返回 200/8 项，TXT 解析到最终 TCP。修复共享原生启动等待、公共中继发现总预算和私网开发目录被系统代理误转发。原生创建后名片/二维码及桌面、移动布局已检查；完整 agent 75/75、SDK 隔离传输 2/2、GUI 单元 24/24、浏览器 92/92、OpenSpec 严格校验通过。

Android 验收设备是 Android 14 arm64 Redroid（240 上由 scrcpy 查看），不是实体手机；软键盘实机、任意 VPN 共存及七种协议逐一原生互通未声称完成。具体事实与边界见 [native-overlay-implementation.md](native-overlay-implementation.md)。

交付收尾：两端测试网络重新启动后双向操作再次返回 200。240 的普通用户 Rove 窗口保持打开；旧投屏进程无窗口且 ADB 已断开，已重建只绑定 240 loopback 的 `15555` 调试转发并重开 scrcpy（实际识别 Redroid、Direct3D11、720×1280 画面）。该转发只用于投屏，不承载已验收的 Rove overlay 业务。临时 WebView 检查转发已关闭。

## 2026-09-23 发现功能增量制品

本次重新构建了包含公共节点目录、HTTP(S)/TXT/SRV 发现配置和创建后网络名片界面的开发包：

| 文件 | 字节 | SHA-256 |
| --- | ---: | --- |
| `target/packages/rove-windows-x86_64-setup-20260923-r9.exe`（已安装到 240） | 33,763,956 | `3712cb4148838b2017fbd730758417c779707d6ef9a4557d688ac11651430548` |
| `target/packages/rove-windows-x86_64-setup-20260923-r8.exe` | 33,781,261 | `21a7b53b22f6789786c7caa69662a2f246255be3530f5417304d137c3882f76c` |
| `target/packages/rove-windows-x86_64-setup-20260923-r7.exe` | 33,770,875 | `055d10f85634881a398fd0b5601f1e45138d998b085a9f13e9a7b7606651093c` |
| `target/packages/rove-windows-x86_64-setup-20260923-r6.exe` | 33,766,444 | `f8bf560a16dffebe943c491001b044681427d3872ae395afc4e2e45947876cec` |
| `target/packages/rove-windows-x86_64-setup-20260923-r5.exe` | 33,765,985 | `a90fc7cc8112109c0e09649db247d1baf6ab19a27a3af8164f8b40078c95e5d5` |
| `target/packages/rove-windows-x86_64-setup-20260923-r4.exe` | 33,770,788 | `451c11b409e12f5d37c5f562e506a033ce020eca2a3bee3c7c18593671df615b` |
| `target/packages/rove-windows-x86_64-setup-20260923-r3.exe` | 33,760,258 | `0a6f738a2560d54cc125653bab7ec4d8b5e47d2f0a607556593fcb7eebb06b91` |
| `target/packages/rove-android-arm64-debug-20260923.apk` | 133,324,763 | `170ee146de6a5520ec741ace2198bdc19493bd42922d8baf6464075ea56d49ce` |
| `target/packages/rove-android-arm64-debug-20260923-r9.apk` | 133,358,203 | `764f74c8f39f4188c44a0424ccb9960c816e5e576ba82c835d0396a199e1380a` |
| `target/packages/rove-android-arm64-debug-20260923-r10.apk` | 133,346,819 | `d3c16f3e9988019e4ee7dd80d6f634b33e320557f42dfba34215d05062c797ef` |
| `target/packages/rove-android-arm64-debug-20260923-r11.apk` | 133,351,339 | `1df7d6e3bf0dad66f80486d7bffeb3d1fbb6f94408a2c2bf95b41a1dcd525ddf` |

Android APK 已在 `127.0.0.1:15555` 的 Android 14 arm64 Redroid 上覆盖安装成功，包名 `app.rove.desktop`、版本 `0.1.0`、targetSdk 36；显式启动 `MainActivity` 后保持 resumed，未见 fatal/崩溃日志，真实 WebView 正常显示。

Windows r2 安装器已传到 `10.1.2.240` 的 `C:\Users\haobo`，远端大小和 SHA-256 与构建机一致，并逐用户静默覆盖安装到 `%LOCALAPPDATA%\Programs\Rove`。首次增量覆盖揭示旧 NSIS 在 agent 运行时会跳过被锁定的 `rove-agent.exe` 却返回 0，造成新 CLI/GUI 与旧 agent 混装；r2 因此增加安装前进程保护。240 实测 agent 运行时 `/S` 返回 32 且不终止 agent，停止 agent 后 `/S` 返回 0；三个主二进制哈希均与打包输入一致。`Test-Rove.ps1` 最终返回 PASS，覆盖 native named pipe、设置写入、重启身份和设置持久化；默认数据目录的既有 `device_id` 仍为 `8f534803-8bb3-4940-a7e2-c35c50709632`。验收结束后已启动新版 agent 供 GUI/CLI 测试。

r3 增加安装包级 `rove-defaults.json`，当前默认发现服务为 `http://10.1.2.237:8090`；`scripts/package-windows-setup.py --discovery-service-url <URL>` 可在以后打包时替换为正式 HTTPS 域名。240 上先有 GUI 运行时安装按预期返回 32，正常关闭 GUI 后覆盖安装返回 0。正式数据目录的 `config show` 返回该默认值；独立测试数据目录通过默认值从本机发现服务取得 8 个过滤后候选（上游来源 `ruixuan-uptime-kuma`、`stale=false`），显式 `--clear-discovery-service` 后保持 `null`。测试结束重新启动新版 agent，未注册服务或计划任务。

r4 修复发现 URI 被直接传给最终连接器探测而产生 `Invalid bootstrap endpoint` 的问题：GUI 现在先在所选 agent 上解析 HTTP(S)/TXT/SRV 链，再对本次最终节点执行受限探测，同时保留原始发现 URI作为网络配置。安装包已在 240 静默覆盖安装成功（exit 0），远端安装器哈希匹配；安装后的 agent 实测把 `https://et-public.lctn.site/` 解析为 `https → txt → tcp://et.lctn.cc.cd:4107`，随后 TCP 探测可达。该结果只验证发现格式和有限端口可达性，不验证 EasyTier 身份、密钥或入网。

r5 将 HTTP(S)/TXT/SRV 发现 URI统一到初始节点输入，不再显示单独的“发现地址/解析”区域；“测试连通”自动解析并展示最终节点，创建与分享仍保留原始 URI。引导页连接设备步骤的“下一步”也移动到原“随时可以稍后设置”的底部位置。r5 已在 240 静默覆盖安装成功（exit 0），安装器哈希匹配；持久 agent PID 为 `25968`，断开 SSH 后仍运行，真实 HTTPS → TXT → TCP 解析再次通过。

r6 把网络标签拆成列表、创建、加入、名片和编辑视图，但创建/加入错误地放在列表内，未严格符合原型。GUI 改为自包含 Base64URL 配置后，二维码仍调用只接受 URL 的 SDK 方法并吞掉错误，因此实际名片缺少二维码；此前浏览器替身未覆盖这个原生限制。r6 已在 240 静默覆盖安装成功（exit 0），该安装结果不能证明二维码正常。

r7 修正上述问题：网络标签显示列表，创建/加入只由右上角全局「＋」菜单打开弹窗，创建成功弹窗转名片；名片保留名称、网段、Base64 二维码/文本和唯一的「复制配置」分享操作，删除导出按钮。新增 `network_config_qr` 原生调用，不再复用 URL 校验，失败有提示和重试。21 项相关浏览器测试、最终布局调整后的 4 项针对性复测、SDK 二维码生成/真实解码 2 项测试、Linux Tauri 原生 IPC 2 项测试及生产构建通过；桌面和 390px 名片截图已检查。Windows 交叉构建包含最终前端资源，r7 在 240 静默安装 exit 0，安装器及 GUI 哈希匹配（GUI：`75ffed4aceab4509cc495f6fed6edbfbe58a7697e0062e6bc755ff55cbf88441`），agent PID `22736`，默认发现服务仍为 `http://10.1.2.237:8090`。Windows 交互桌面验收由用户进行；Linux IPC 与浏览器测试不替代该验收。

r8 精简网络列表：删除重复“我的网络”标题及静态长说明，刷新改为页头图标。创建、加入、编辑及名片共用左上角 chevron 回退；桌面使用弹窗，手机使用全屏布局，名片顶部回退和底部复制保持可见，内容独立滚动，紧凑横屏采用二维码/配置分栏。引导中的网络操作复用相同布局，相关兼容与验收要求已写入 `AGENTS.md`。生产构建、4 项相关浏览器用例及扩展后的移动布局用例通过；人工检查 1100×760、320×568、390×844、844×390 截图，断言二维码完整、回退/复制在滚动时固定且可达。r8 已在 240 安装 exit 0，安装器和 GUI 哈希匹配（GUI：`78c9a217237833ccc5e6e5d5ac7280daf1911aa1f2b0cf0fd168a6d30ec7547a`），agent PID `3096`。本轮移动验证为浏览器响应式验证，未重新安装 Android，也未做 Android 软键盘实机验收；Windows 交互验收由用户进行。

r9 将设备置于网络下一级：点击网络条目进入 `#/networks/<network_id>`，读取当前网络状态、实际地址和设备，名片及网络管理操作移至详情；移除网络列表的全局设备入口。设备查询显式使用当前 network_id 和目标 agent，错网行拒绝显示，迟到网络响应被丢弃；远端设备列表只读，未新增二次转发选择。16 项相关浏览器回归与最终 9 项专项复测通过，覆盖四种尺寸、导航、错网/过期响应、远端目标和模型同步；完整 79 项套件尝试中途被 SIGTERM 终止（143），未计为完整通过。GUI 构建、Windows 交叉构建、OpenSpec 严格校验通过，已检查桌面与移动平台标志下的截图。r9 已生成但未安装，GUI SHA-256 为 `32da437f897f02c0e5609ea9788807198aa8cffa30bb4512ff641f501ba09cea`。

用户要求打开现有安装版查看时，通过临时交互式任务在 240 的 haobo 桌面 Session 1 启动 r8（PID `19964`），在同一会话内验证窗口标题 `Rove`、非零句柄 `7866982`，并成功置前；临时任务已注销。用户查看期间未覆盖 r9、未关闭其窗口。Android 本轮未更新，设备 UI 测试使用 fixture，不代表发现了实际 overlay 设备。

随后用户明确要求覆盖并启动 Android：r9 在 240 覆盖安装返回 0，远端安装器及安装后的 GUI SHA-256 均匹配。新版 agent PID `8208`；通过临时交互式任务启动 GUI PID `17468`，Session 1 中窗口标题 `Rove`、句柄 `2033908`，置前成功，临时任务已注销。未卸载或清空应用数据。

同轮 Android r9 由完整 Tauri Android debug/aarch64 构建生成，在 242 的既有 Android 14 arm64 Redroid（ADB `127.0.0.1:15555`）执行 `install -r` 返回 Success。`MainActivity` 冷启动 Status ok，PID `2466913`，保持 topResumedActivity。升级前后 device_id 均为 `e6ba18ba-825f-4135-a99e-118485000963`，未卸载或清数据；真实 WebView 加载 `index-Dt41lH1U.js`，与 Windows r9 前端一致。已打开 `#/networks` 供查看，360px 宽度下 scrollWidth=360，截图确认顶部加号和底部网络标签可见；当前保存的网络列表为空，没有创建验收用假网络。启动日志有 Redroid 图形缓冲格式告警，所检查日志未见 fatal/panic；页面实际显示正常。本次只验安装、启动与网络列表显示，不代表软键盘、名片全流程或 overlay 实机通过。临时证据：`/tmp/rove-android-r9-before.json`、`/tmp/rove-android-r9-after.json`、`/tmp/rove-android-r9-launched.png`。

增量包仍保持下文的平台边界：Windows 安装元数据为 `overlay_runtime=none`，Android 没有 `VpnService`/EasyTier 原生网络通道。它们可用于界面和发现流程验收，但不能完成实际 EasyTier 启停、overlay 设备发现或 Rove overlay 通信。因此，**尚未完成 Windows–Android 组网**。

### Android r10：默认发现地址与 DNS 初始化修复

后续 r11 已修正 DNS 重试期限并完成 HTTPS 全链复验，见下节；以下保留 r10 当时的失败事实。

用户实机报错 `discovery_service_not_configured`，根因为此前仅 Windows sidecar 携带默认地址，Android 系统 app_process 旁没有该文件。r10 通过编译期 `ROVE_DISCOVERY_SERVICE_URL` 嵌入经相同规则校验的默认地址，保留用户覆盖/清除优先级。另修复 TXT/SRV 系统 DNS 配置不可读时解析器直接失败，使用所固定 EasyTier 的后备 DNS。公共获取及网络启停错误新增警告图标和双语文案；Windows runtime 没有因此实现。

r10 已 `adb install -r` 成功，MainActivity 冷启动 Status ok，PID `2469637`，原 device_id 保持不变。真实设置返回 `http://10.1.2.237:8090`，目录 API 返回 200、8 项、stale=false；实际创建表单点击获取亦显示 8 项，宽度和 scrollWidth 均为 360。240 的 `Rove-Android` scrcpy PID `26660`、Session 1 仍响应。证据：`/tmp/rove-android-r10-discovery.json`、`/tmp/rove-android-r10-resolution.json`、`/tmp/rove-android-r10-public.png`。

Android TXT 实测 `txt://et.lctn.cc.cd → tcp://et.lctn.cc.cd:4107` 成功；同机 HTTPS `https://et-public.lctn.site/` 仍请求失败，未取得完整链。构建机同期可取得三个 TXT 行，说明存在平台/访问环境差异，但此次没有取得足够证据确定 Android HTTPS 失败的具体原因。不得把目录显示 up、TXT 解析或构建机 HTTPS 成功外推为 Android HTTPS 可用或入网成功。Windows 保持 r9 安装，本轮新增桌面错误文案仅源码和浏览器验证，实际 EasyTier 启停仍未实现，需后续 `windows-overlay-runtime` 设计与实现。

### Android r11：HTTPS 全链复验与有界 DNS 重试

继续复验发现 r10 也可间歇成功，另一次明确报 `discovery_dns_timeout`。Hickory 默认单次等待 5 秒，而 Rove 外层 3 秒结束，导致丢包时内部重试来不及发生。r11 将单次等待设为 900ms、两次尝试，保持外层 3 秒；本地 UDP fixture 丢弃第一包后重试成功。HTTP 发现现在区分请求超时、连接失败、非成功 HTTP 状态和其他失败；DNS 错误独立显示，界面保留原始 URI和错误图标。错误不返回 URL 查询、头或正文，TLS 验证未关闭。此前泛化的 HTTP 错误没有保留底层原因，因此不声称已追溯其全部原因或外部站点永久可用。

r11 覆盖安装 Success、冷启动 Status ok（PID 2470783），device_id 保持原值。真实 HTTPS → TXT → TCP/UDP API 连续三次成功；GUI 点击“测试连通”得到 `https://et-public.lctn.site/ → txt://et.lctn.cc.cd → tcp://et.lctn.cc.cd:4107`，TCP 可达 315ms，输入仍是 HTTPS，360px 无横向溢出；目录返回 8 项。无效自签 TLS fixture 被拒绝（`discovery_http_connect_failed`），服务统计 https_requests=0、tls_rejections=1。没有安装证书、关闭验证或降级 HTTPS；测试反向端口及 fixture 已停止。240 的 Rove-Android 窗口重新启动并置前，PID 11792、Session 1、标题和非零句柄已确认。

验证：Rust 发现模块 8/8，GUI 单元 24/24，错误/重试浏览器 9/9，生产及 Android 构建、OpenSpec 严格校验通过；人工检查四种尺寸截图。临时证据位于 `/tmp/rove-android-r11-https.json`、`rove-android-r11-tls-negative.json`、`rove-android-r11-state.json`、`rove-android-r11-ui-probe-final.json`、`rove-android-r11-https-ui.png`。该成功只证明发现链与有限 TCP 探测，不是 EasyTier 入网；Windows r9 未升级、Windows runtime 与 Android native VPN 仍未实现。

2026-09-21。本轮交付一个每用户 Windows 安装器并在 10.1.2.240 完成安装/升级/卸载与原生回归，随后与 Android 14 arm64 容器做双向网络测试。这是开发测试制品，不是正式签名发行版，也不是 Android → Windows 组网验收。

## 制品

| 文件 | 字节 | SHA-256 |
| --- | --- | --- |
| `rove-windows-x86_64-setup.exe` | 32,885,454 | `084ff8f662ad30ac0ab5dcc961724aa26e178dd975fcd429fd1a41af52c43bf0` |

NSIS 安装器 `packaging/windows/rove.nsi`，由 `scripts/package-windows-setup.py` 调用 makensis 生成。`RequestExecutionLevel user`，不请求管理员权限，不写 PATH、不注册 Windows 服务、不改防火墙、不装网络驱动。GUI/CLI/agent 与两份测试脚本安装在同一目录，脚本用 `$PSScriptRoot` 定位二进制。

安装 9 个文件：`rove.exe`、`rove-gui.exe`、`rove-agent.exe`、`WebView2Loader.dll`、`uninstall.exe`、`install-info.ini`、`README.txt`、`Test-Rove.ps1`、`Test-RoveGui.ps1`。`install-info.ini` 记录 `version=0.1.0`、`layout=per-user`、`webview2_runtime=1`、**`overlay_runtime=none`**、`data_dir=%LOCALAPPDATA%\rove`。

## 240 上的安装与生命周期

测试机 `DESKTOP-CO37UIM` / 10.1.2.240（Windows 11，`WORKGROUP\haobo`），逐用户安装到 `%LOCALAPPDATA%\Programs\Rove`，数据在 `%LOCALAPPDATA%\rove`。

| 动作 | 结果 |
| --- | --- |
| 干净安装 | 安装前无安装目录、无注册表项；`/S` 静默安装 exit 0，7.4 s；注册表 `HKCU\...\Uninstall\Rove` 与开始菜单 `Rove.lnk`、`卸载 Rove.lnk` 建立；用户/机器 PATH 均未被写入 |
| 同版本升级 | 覆盖安装 exit 0，7.6 s，仍为 9 个文件，`upgrade.json` 记录 `installed_before=true` |
| 原生回归 | `Test-Rove.ps1` 返回 PASS；checks 为 native named pipe / settings write / restart identity / settings persistence；`device_id` `8f534803-8bb3-4940-a7e2-c35c50709632` 重启后不变；`overlay_tested=false` |
| 卸载 | 安装目录、注册表项、开始菜单全部移除；`%LOCALAPPDATA%\rove` 与卸载前写入的标记文件保留 |

卸载的一处细节：NSIS 只删除已知产品文件后用 `RMDir` 收尾，目录非空时 `RMDir` 静默失败。第一次卸载记录 `install_dir_removed=false`，残留文件全部是测试脚本启动 agent 时写进安装目录的 `agent.stdout.log`/`agent.stderr.log`；清掉这两个测试日志后重跑，目录被完整移除（`uninstall-clean2.json`）。这是测试产物的影响，不是产品缺陷，但也说明第三方文件会让安装目录留在磁盘上。

生命周期结束后 240 保持已安装状态（`status-final.json`：9 个文件、注册表与开始菜单存在、数据目录存在、无残留 rove 进程）。

## 与 Android 平台的网络测试

拓扑：Windows 240（10.1.2.240/22）与 Android 14 arm64 Redroid 容器（`172.17.0.2/16`，`adbd` 监听 5555，agent Unix socket）分属两台宿主，Android 宿主机为 10.1.2.242，构建机 10.1.2.237。两端都用各自 agent 的公开接口取数，不新增产品能力。

两端身份与能力：Windows agent `8f534803-8bb3-4940-a7e2-c35c50709632`，`os=windows`、`arch=x86_64`，7 项 capability（含 `system_exec`）；Android agent `e6ba18ba-825f-4135-a99e-118485000963`，`os=android`，6 项 capability（无 `system_exec`）。两端的 `network list` 都为空，即当前没有已建立的 Rove 网络。

### Android → Windows（直接可达）

- 容器内实际 TCP 连接 `10.1.2.240:22` 成功，收到 `SSH-2.0-OpenSSH_for_Windows_10.0 Win32-OpenSSH-GitHub`；`10.1.2.242:22`、`10.1.2.237:22` 同样成功。
- Android agent `test_network_peer`：`tcp://10.1.2.240:22` reachable 2 ms；`tcp://10.1.2.242:22` reachable 1 ms；`tcp://10.1.2.237:22` reachable 2 ms。
- ICMP 环境差异：容器 ping 242 丢包 0%，ping 240 丢包 100%（240 拦 ICMP），所以 ping 不能当作 240 是否可达的判据，TCP 可以。

### Windows → Android

- Windows agent `test_network_peer` `tcp://10.1.2.242:22` reachable 33 ms（同 /22 网段，结果可信）；`tcp://10.1.2.242:59999` unreachable，说明探针能区分真实服务与死端口。
- 到 Android 容器网段没有路由：240 的路由表没有 172.17.0.0/16 条目，容器在 242 的 docker 桥上。因此 240 不能直连容器 IP，Windows 探针对 `172.17.0.2:5555` 报 reachable 属于下面记录的假阳性。
- 为了真正验证「Windows 能访问 Android 平台内部服务」，用测试链路中转后探测：237 上 `adb forward tcp:16002 tcp:5555` 指向容器内 `adbd`，237 再用 socat 监听 `0.0.0.0:16003` 转发到该端口；Windows agent 探测 `tcp://10.1.2.237:16003` → reachable 32 ms。对照项 `tcp://10.1.2.237:16099`（同主机未监听）→ unreachable，`tcp://127.0.0.1:59999` → unreachable。该路径由测试链路构成，Rove 自身没有任何跨平台通道。

### Windows 作为服务端（入站）

- 240 绑定 `0.0.0.0:15888`，本机自连 3/3 成功，证明端口确实在监听并接受连接。
- 同一时间窗口：Android 容器 `nc` 到 240:15888 超时；237 到 240:15888 超时（rc=124）；而 237 到 240:22 成功（rc=0）。即失败发生在入站链路上，不是端口或服务缺失。
- 240 三个防火墙 profile 均 Enabled，Rove 相关规则 0 条；`rove-agent.exe` 启动后 `listeners_added_by_rove=[]`、`rove_owned_listeners=[]`，每进程只有 named pipe。
- 结论：Windows 侧目前没有可被其他平台访问的入站入口，本轮也没有改防火墙。

### 可达性结论的可信边界（重要证伪）

240 上装有 Mihomo TUN（`198.18.0.1/30`，接口别名 Mihomo / Meta Tunnel）。它代答非同网段的 TCP 握手，导致 Windows 侧探针出现假阳性：`tcp://203.0.113.7:59999`（TEST-NET-3，必然不可路由）reachable 11 ms、`tcp://192.0.2.1:59999`（TEST-NET-1）reachable 0 ms、无路由的 `172.17.0.2:5555` reachable 0 ms。

因此本报告只采信同 /22 网段与 loopback 的 Windows 结果，并统一配死端口对照；Android 容器侧没有这类 TUN，其结论直接可用。后续在这个测试机上不要用 `Test-NetConnection` 或单一探针结果判断外网可达性。

## 明确未覆盖

- **没有 Android ↔ Windows overlay/组网验收。** Windows 与 Android 的 `start_network`/`stop_network`/设备发现都未实现（非 Linux 返回 `No verified overlay interface binding`，设备发现返回空列表），Android 也没有 native VPN。本轮全部结论都是宿主 IP 层面的 TCP 可达性与 agent 只读探测。
- 安装包不含网络程序（EasyTier），不注册 Windows 服务（SCM），不写 CLI PATH，未做「关窗后后台继续」验证。
- GUI 未在交互式桌面会话验证：`Test-RoveGui.ps1` 需要已登录会话，本轮只跑了原生 agent/CLI 回归。
- 未测物理网卡、整机重启/登录退出、多用户与域环境；制品是 GNU 交叉构建的 dev profile，未做代码签名。

## 与后续计划的关系

本轮交付的是 `docs/next-iterations.md` 第 2 项 `windows-installer-lifecycle` 的一部分：安装、同版本升级、保留数据卸载、原生回归。该项依赖的第 1 项 `windows-overlay-runtime` 未完成，所以网络程序、服务注册、CLI PATH 与后台继续不在本轮范围；第 3 项 `android-overlay-network`、第 5 项 `android-windows-acceptance` 未开始。本文件不把上述任一项标为完成。

## 复现

打包（需要 makensis；输出路径必须不存在，脚本拒绝覆盖既有包）：

```sh
python3 scripts/package-windows-setup.py --output target/packages/rove-windows-x86_64-setup-20260921.exe
```

在目标机（脚本自定目录，只操作 `%LOCALAPPDATA%` 与 HKCU）：

```powershell
# 安装（新建或同版本升级）/ 状态 / 原生回归 / 卸载，输出单个 JSON
scripts\check-windows-setup.ps1 -Action install   -Root <暂存目录>
scripts\check-windows-setup.ps1 -Action status    -Root <暂存目录>
scripts\check-windows-setup.ps1 -Action native    -Root <暂存目录>
scripts\check-windows-setup.ps1 -Action uninstall -Root <暂存目录>

# 启动随包 agent，记录监听/防火墙/只读探测结果
scripts\check-windows-network.ps1 -Endpoints tcp://10.1.2.242:22,tcp://10.1.2.242:59999

# 绑定一个端口并保持，供另一平台探测（先自连，证明端口确实在监听）
scripts\check-windows-inbound.ps1 -Port 15888 -HoldSeconds 40
```

Android 侧在设备内运行（只读）：

```sh
adb push scripts/check-android-agent-network.sh /data/local/tmp/agent-net.sh
adb shell sh /data/local/tmp/agent-net.sh tcp://10.1.2.240:22 tcp://10.1.2.240:15888 tcp://10.1.2.242:22
```

证据在 `/mnt/data/rove-deliveries/2026-09-21-windows/`：`rove-windows-x86_64-setup.exe`、`install-final.json`、`upgrade.json`、`native.json`、`uninstall-clean.json`、`uninstall-clean2.json`、`status-final.json`、`status-before.json`、`status-after-uninstall.json`、`network.json`、`network-control.json`、`network-android-reverse.json`、`android-agent-network.txt`、`android-raw-tcp.txt`。
