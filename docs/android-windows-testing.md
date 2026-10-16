# Android / Windows 开发包与实机验证

更新于 2026-10-08。以下是开发测试包，不是全平台正式发行版。

最新 r27：共享运行时状态观察失败保留启动意图并退避重试，显式停止仍有效，权限/安全失败不绕过。Windows r26 的页面资源缺失已在 r27 重编译并补充打包校验，240 覆盖安装后真实页面/资源加载、原生 get_device、正常网络启停及原数据保留通过。

Android r27 已构建，APK SHA-256 `257f27171341ed587c6303da41e822526b2875fa084adb6290684677ed1c4daa`，v2 Debug 签名通过，与 r26 字节一致。原授权 242 Redroid 无法接入，**本轮 Android 尚未安装及原生验收**；未擅自改用 240 新连接的实体手机。正常 Windows 启停与共享故障注入单元测试不能代替 Android 原生恢复，不能宣称本轮 Windows–Android 组网已验收。旧 target_unreachable / Mihomo 调查按用户要求暂停。制品与完整边界见 [Windows 交付记录](windows-installer-delivery.md)。

## 当前 r24 编辑后名片重新生成

Windows 240 与 Android 14 arm64 Redroid 已覆盖安装并启动 r24。编辑保存后再次点击网络名片，重新读取最新配置并生成 Base64/二维码；旧异步结果不能覆盖新名片。两端真实安装版验证修改前后内容变化，实际原生二维码解码为最新 Base64，并与后端配置一致。临时网络已清理，原身份、12/18 个会话、模型、网络和别名保留。

GUI 单元 30/30、完整浏览器 105/105、名片专项 5/5，四尺寸及原生截图已查看。具体制品、哈希、非实体手机/相机验收边界见 [r24 交付记录](windows-installer-delivery.md)。重连仍是独立未完成问题：间歇性恢复不等于修复，Rove 是否优先物理出口尚待用户确认；未擅自改变系统网络策略。

## r23 网络编辑与保存自动重启（历史）

Windows 240 和 Android 14 arm64 Redroid 已覆盖安装并启动 r23，移除“管理此设备”，网络编辑支持名称、初始节点和地址配置，key 只读且后端拒绝变更。运行中保存会提示短暂断连，并自动停止、保存、重新启动；两端实际观测到完整状态变化，原配置不变。停止网络只保存、不自动启动。

GUI 单测 30/30、浏览器完整回归 104/104；桌面、320px 小屏、手机竖屏和紧凑横屏截图已查看，真实安装版编辑/自动重启和原数据保留通过。原 12/17 个会话保留，Android 另新增的非测试会话也保留；Windows 当前本机别名 `windows` 未改动。两端临时测试网络已清理。

17:12 两端虽 running，双向目标 get_device 仍为 503；本轮不代表永久重连或 Windows–Android 组网验收通过。未修改系统网络策略，未做实体手机/真实软键盘完整验收。制品哈希及精确边界见 [r23 交付记录](windows-installer-delivery.md)。

## r21 本机别名与原位编辑（历史）

Windows 240 和 Android 14 arm64 Redroid 已覆盖安装并启动 r21；点击名称原位编辑，不再显示单独修改表单。完整浏览器回归 100/100，安装版原位展示/取消、Android 保存和外部点击失焦保存已验证；本机名称解析、重名拒绝、原数据保留通过。Windows 用户 `aa` 保持不变，Android 验收临时别名已清除。没有实体手机/完整中文软键盘验收。

两端网络恢复 running，但 15:43 双向目标 API 仍为 target_unreachable；本次不计为组网或永久重连修复通过。制品哈希、原生覆盖范围及限制见 [r21 交付记录](windows-installer-delivery.md)。以下 r18/r17 为历史记录，不是当前安装版本。

## Windows r18 中文输出与后续诊断

240 已安装中文命令输出修复，原生 ping/stdout/stderr 和持久历史验证正常；随后加载保留该修复的 r19 诊断 agent，Android 保持 r17。系统选路再次复现重连失败；Rove-only 物理接口对照三次恢复，临时规则已撤销，永久默认出站策略尚待确认。当前事实与制品边界见 [交付记录](windows-installer-delivery.md)，不可把下文旧轮次结果当作已彻底解决重连。

## 当前 r17 会话安装版验证

Windows 240 与 Android 14 arm64 Redroid 均已覆盖安装并启动 r17，原身份、会话和设置保留。本机新建会话归属、Android 会话经 EasyTier 请求 Windows agent 实际执行命令并返回结果、停止目标网络后拒绝调用且不转本机执行，均取得原生证据。收尾恢复网络启停状态后，两端虽为 running，但截至 2026-09-29 10:31（Asia/Shanghai）双方目标调用仍为 target_unreachable，重连恢复尚未通过。完整制品、测试记录和限制见 [交付记录](windows-installer-delivery.md)。

## r16 原生网络结果（历史）

Windows 240 与 Android 14 Redroid 的安装版已完成真实 EasyTier 组网、双向设备发现/Rove 操作、分别停止失效及 Windows 物理网负向测试；原身份和配置保留。Android 使用应用自身 VpnService，Windows 使用普通 agent 与独立受限 helper。制品与完整边界见 [交付记录](windows-installer-delivery.md) 和 [原生验收](native-overlay-implementation.md)。不代表实体手机、软键盘或全部 connector 原生逐项互通通过。

当前 Windows native 安装器构建（先完成 GUI 前端构建，使用经校验的官方驱动归档）：

```sh
cargo build --locked --target x86_64-pc-windows-gnu -p rove-agent -p rove-cli --features rove-agent/easytier-native
cargo build --locked --target x86_64-pc-windows-gnu -p rove-gui --features custom-protocol
python3 scripts/package-windows-setup.py \
  --wintun-archive /path/to/wintun-0.14.1.zip \
  --windivert-archive /path/to/WinDivert-2.2.2-A.zip \
  --windivert-source /path/to/WinDivert-2.2.2-Source.zip \
  --output target/packages/rove-windows-native-setup.exe
```

## 2026-09-23 发现功能增量包

包含公共节点目录、四种发现 URI 和创建后网络名片界面的 Android arm64 Debug APK 已重新构建并覆盖安装到 Android 14 arm64 Redroid。`app.rove.desktop/.MainActivity` 冷启动成功并保持 resumed，真实 WebView 正常显示；制品、哈希、Windows 包状态和平台能力边界见 [Windows 安装包与 240 实机测试](windows-installer-delivery.md)。Android 仍无 native VPN/EasyTier 通道，Windows 包仍无 overlay runtime，故尚未完成 Windows–Android 组网。

## 2026-09-21 Windows 安装器与 Android 双向网络实测

每用户 NSIS 安装器落在 240 并完成干净安装、同版本升级、保留数据卸载与原生回归；随后与 242 上的 Android 14 arm64 容器做双向 TCP 探测，Windows 侧入站被本机防火墙挡住。制品校验和、证据文件与明确未覆盖项见 [Windows 安装包与 240 实机测试](windows-installer-delivery.md)。仍不是 Android → Windows 组网或控制验收。

## 2026-09-18 Android 原生布局与最新升级

最新包/哈希/验收入口见 [Ubuntu / Android 交付](ubuntu-android-delivery.md)。本日已在既有 Android 14 arm64 覆盖升级，保留旧身份/两条会话/设置；真实界面提交、Rig unsupported、归档恢复通过，临时模型已移除。

MainActivity 保留 edge-to-edge，由 AndroidX WindowInsetsCompat 在根容器处理 systemBars、displayCutout、ime 并消费，避免 WebView 的顶部与固定底栏遮挡及 CSS 重复留白；manifest 声明 adjustResize。竖屏、横屏、软键盘弹出/收起均有实际 WebView 尺寸证据，未将 Android 14 Redroid 的结果扩大到所有 API/物理相机/屏幕缺口设备。

最终安全区域修复只涉及 Kotlin/manifest，复用已验证 JNI library 的 Gradle 重包；SHA-256 与包内 library 比对通过。不改变 native VPN、后台和本机命令尚未交付的边界。`scripts/android-webview-eval.mjs` 可增加一个不存在的结果文件路径参数，独占创建 JSON 证据（不会覆盖已有记录）。

## 2026-09-17 Windows 原生回归

Windows 11 测试机在新目录 `C:\Users\haobo\rove-check-20260917-2115` 运行本轮交叉编译的 `agent-tests.exe --test-threads=1`，21 项通过。包括真实 named pipe 的同账号双客户端、共享状态、首实例防抢占、退出后重新监听；增加专用线程临时匿名 impersonation 的拒绝测试，打开管道确实返回 Windows `ERROR_ACCESS_DENIED`，线程随后恢复身份。

管道使用 protected DACL，只有当前用户 SID 一条 allow ACE，不继承 Everyone/其他用户权限。匿名身份测试不等于另建普通账号登录测试；结合 DACL 检查验证本机同账号边界。不改系统账号、PATH、防火墙、网络服务或其他应用。此次同时在 Windows 验证存储检查、SQLite v2/v4 升级保留身份和内容、服务生命周期、会话归档及队列逻辑。没有据此宣称 Windows overlay 或安装器已经交付。

## 早期 Windows x86_64 便携包（不含原生组网运行时）

构建前先在 `apps/rove-gui` 执行 `npm ci && npm run build`，然后在仓库根目录执行：

```sh
cargo build --locked --target x86_64-pc-windows-gnu -p rove-agent -p rove-cli
cargo build --locked --target x86_64-pc-windows-gnu -p rove-gui --features custom-protocol
python3 scripts/package-windows.py --gui --output target/packages/rove-windows-x86_64-portable.zip
```

需要 MinGW 交叉链接器；GUI 包必须包含构建产出的 `WebView2Loader.dll`，目标机器需要 WebView2 Runtime。打包脚本拒绝覆盖既有包。解压到用户拥有的目录，在同一账号运行 agent 与 GUI。便携包不配置 PATH、系统服务、防火墙或 EasyTier 驱动。

`Test-Rove.ps1` 验证真实 named pipe、配置写入、代理重启、身份和配置持久化，只停止自己启动的进程，保留测试数据。`Test-RoveGui.ps1` 必须在已登录用户的交互式桌面运行；SSH 非交互会话不能替代桌面测试。GUI 检查使用 Windows UI Automation，比对页面渲染的 device_id 与 CLI 返回值，不读取其他应用的内容。

## Android arm64 APK

前置：JDK 21、Android SDK platform 36、Build Tools 35/36、NDK 27.2.12479018、Rust Android target。当前 Gradle wrapper 为 8.14.3。

```sh
rustup target add aarch64-linux-android
# JAVA_HOME、ANDROID_HOME、NDK_HOME 指向本机工具目录
cd apps/rove-gui
npm ci
ROVE_DISCOVERY_SERVICE_URL=http://10.1.2.237:8090 npm run tauri -- android build --debug --target aarch64 --apk --ci
```

`ROVE_DISCOVERY_SERVICE_URL` 是 Android 包的编译期发现服务默认值，正式发布时替换为 HTTPS 域名。省略该变量则不预置发现服务；它不是设备运行时环境变量。Android agent 不读取系统 `app_process` 旁的 `rove-defaults.json`。默认值通过与 Windows 包相同的地址校验，仅完全匹配的预置私网 HTTP 地址可用于开发；用户保存的地址或显式清除均优先，覆盖安装不强制重置设置。

Android 工程已纳入 `src-tauri/gen/android`。构建回调用 `npm run tauri`，不依赖开发机绝对路径。SDK、NDK、JNI 二进制和签名私钥不纳入源码。Debug APK 仅用于开发，正式签名、升级策略尚未验收。若依赖下载需要代理，Java 需配置自己的 HTTP/HTTPS 代理参数，不要把凭据写进仓库。

移动端在应用生命周期内启动同一个 `rove-agent` library，Vue → Tauri → SDK → Unix socket → agent；数据库仍由 agent 独占。数据位于应用私有目录，禁用 Android backup。Android r16 已包含 VpnService 与原生 EasyTier，后台运行受系统约束，仍不提供本机 shell；iOS 尚未完成构建。APK 编译本身不证明 overlay 成功，实际跨端验收另行记录。

## 无图形界面的 ARM Android 测试宿主

Redroid 使用宿主 Binder 与特权容器。必须在获授权的测试宿主运行，不要挂载业务数据、Docker socket 或把 ADB 暴露到公网。CPU 不支持 AArch32 时应使用官方 `14.0.0_64only-latest`，不要禁用 BoringSSL 自检来强行启动普通多 ABI 镜像。

ADB 仅绑定宿主 `127.0.0.1:15555`。有桌面的调试电脑可通过 SSH 隧道操作：

```sh
ssh -N -L 15555:127.0.0.1:15555 <user>@<android-host>
adb connect 127.0.0.1:15555
adb -s 127.0.0.1:15555 install <rove-arm64-debug.apk>
scrcpy -s 127.0.0.1:15555
```

`scrcpy` 的窗口在调试电脑显示，宿主不需要桌面。宿主和 Android 完成启动前，这些命令不能视为已验收。

## 本轮证据与限制

### ARM 主机恢复后的增量（2026-09-10）

- 主机重启后内核为 `7.0.0-31-generic`。新容器 `rove-android-test-20260910-64only` 使用纯 64 位镜像与独立 `/var/lib/rove-android-test-20260910-64only` 数据目录，旧失败容器/数据保留。
- 资源约束：2 核、4 GiB RAM、memory-swap 同为 4 GiB（不给容器额外 swap）、2048 PID/线程、日志 10 MiB × 2；ADB 只绑定宿主 loopback。未启动或改动业务容器。
- 初始 1024 PID/线程上限发生 8 次触顶，Android system_server 因无法创建线程重启；memory.events 没有 OOM。调至 2048，并通过 Android `svc bluetooth disable` 关闭此测试镜像异常的蓝牙模拟服务后继续测试。不代表 Rove 支持蓝牙或相机测试。
- Android 14 完成启动，APK 安装成功，真实 WebView 显示 agent 的 `os=android`、`arch=aarch64`。页面表单通过 SDK/socket 将并发设置从 4 保存为 3。
- 经 ADB reverse 接入本机可控 SSE 模型（`scripts/android-test-provider.py`），实际 Rig 调用 `system_exec` 返回 `unsupported`，stdout/stderr 为空；会话继续并保存完整事件。测试未调用收费模型，随后清除测试模型配置。证据为 `target/packages/android-unsupported-events.json`。
- 一次低负载采样为 CPU 0.40%、内存 781.6 MiB；不代表全过程。Rove 前台时曾达到约 200%，回到 Android 桌面后降到约 10%。高占用位于应用内多个 `Thread<00..03>` 与渲染线程，进程加载了软件 Vulkan/ANGLE 库；前后台对照支持图形显示是主要负载来源，但未做完整采样剖析。保留 2 核限制；宿主保持可连接。
- 实测发现 360 CSS px 屏幕的 main 宽度膨胀到约 442 px，补充 Grid 最小宽度与窄屏文本换行，重新打包验收。
- 强制关闭并重新启动 Rove 后，device_id 仍为 `e6ba18ba-825f-4135-a99e-118485000963`，并发设置仍为 3，模型配置为空；没有依赖旧进程内存保持身份或设置。
- 新 APK 覆盖安装后上述身份/设置仍保留，真实页面 `clientWidth=scrollWidth=main width=360`，横向溢出修复通过。证据：`target/packages/android-upgrade-test-result.json`、`android-v2-launch.png`。当前 Android 包是 `target/packages/rove-android-arm64-debug-v2.apk`，SHA-256 为 `a9eecdda694bf37f4b9f00017de704b681fd9a12a38d1809c8a8e97a6b72c234`，开发签名校验通过。旧 APK 保留用于对照。
- 本轮前端 8 项测试、类型检查/生产构建、OpenAPI/验收映射和 Cargo fmt 通过。测试结束将 Rove 切回 Android 桌面，保留容器与数据供后续调试；临时模型服务和 ADB reverse 已停止/移除。

调试 WebView 时，先将该 Rove 进程的 `webview_devtools_remote_<pid>` 用 ADB forward 映射到本机 19222，再使用 `scripts/android-webview-eval.mjs`。这是 Debug APK 的开发调试能力，不是新增产品 API，也不是 overlay 通信验收。

### 早先打包与失联记录

- Windows 11 原生 agent/CLI 测试通过；GUI 在已登录用户桌面创建 Rove 窗口，UI Automation 比对页面 device_id 与 CLI 一致（`gui_ipc_verified=true`）。没有把窗口创建或独立 CLI 检查代替 GUI IPC 验收。
- 新增 embedded-agent 单元测试通过：真实 SDK/socket、重复启动拒绝、重启保持 device_id。
- Android arm64 Debug APK 已由 Tauri/Gradle 成功构建（约 60 MB，最低 API 24、目标 API 36）；产物为 `target/packages/rove-android-arm64-debug.apk`。Windows 完整便携测试包为 `target/packages/rove-windows-x86_64-portable.zip`，包含 GUI/CLI/agent、WebView2 Loader 和两份测试脚本。
- APK 的 v2 开发签名校验通过。SHA-256：Android `b5e9e9adb29fba30ebd98d51febe961475dc1d65012dc11de99c3b17427b6d6d`；Windows `003aa4392f9350102dbdcf6ce9fe8b1eea586e3190c23273bd2ac96c5d950793`。
- 本轮 `cargo test --offline --locked -p rove-agent --lib` 28 项通过；不是此前带 EasyTier feature 的全量回归。
- ARM 普通镜像因 32 位 BoringSSL 自检程序无法执行而启动失败；纯 64 位镜像已下载。
- 随后 ARM 宿主失联，开发机和 Windows 均不可达，开发机 ARP 为 FAILED。无法据此确定宿主故障原因，也无法确认传输中的镜像导入和测试容器停止操作是否完成。没有执行宿主重启或业务容器管理操作。
- Android 安装、界面、持久化以及 Android → Windows overlay 控制仍待实际通过，不据此勾选全平台任务。

参考：[Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)、[Redroid 官方说明](https://github.com/remote-android/redroid-doc)。
