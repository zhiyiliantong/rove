# Android / Windows 开发包与实机验证

2026-09-10。以下是开发测试包，不是全平台正式发行版。

## Windows x86_64

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
npm run tauri -- android build --debug --target aarch64 --apk --ci
```

Android 工程已纳入 `src-tauri/gen/android`。构建回调用 `npm run tauri`，不依赖开发机绝对路径。SDK、NDK、JNI 二进制和签名私钥不纳入源码。Debug APK 仅用于开发，正式签名、升级策略尚未验收。若依赖下载需要代理，Java 需配置自己的 HTTP/HTTPS 代理参数，不要把凭据写进仓库。

移动端在应用生命周期内启动同一个 `rove-agent` library，Vue → Tauri → SDK → Unix socket → agent；数据库仍由 agent 独占。数据位于应用私有目录，禁用 Android backup。没有 native VPN、后台常驻和本机 shell 能力，也尚未完成 iOS 构建。因此 APK 编译成功不表示 Android 已能通过 overlay 管理 Windows。

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
