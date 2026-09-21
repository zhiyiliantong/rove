# Ubuntu / Android 原型迁移开发包

当前进度 **73/82**，最新包已包含自动/手动地址模式、Linux 多网络、节点探测和 [API 配置同步](model-sync.md)，不是仅有原型的演示包。官方账号适配及未完成平台仍保留任务。

## 最新交付：2026-09-18

目录 `/mnt/data/rove-deliveries/2026-09-18/`；[验收下载页](http://10.1.2.237:4174/) 提供三个包、SHA256SUMS 与原生截图，没有开放 agent 或数据库。[验收步骤与范围](core-acceptance.md)。`http://10.1.2.237:4173/` 仍是旧独立原型，不连接正式 agent。

| 文件 | SHA-256 |
| --- | --- |
| `Rove_0.1.0_amd64.deb` | `f3b757af3c5af1b9a7ef1f20153c06a996ff2c30aa6124778b99d146e625ec9e` |
| `rove-runtime_0.1.0_amd64.deb` | `373ee02bffd5a2ce9a6a7fc875dafcdf87b9b3d7eff9e5aadd542f7e5f8be7b9` |
| `rove-android-arm64-debug.apk` | `b420574f840f0f045ee3072634bacdb47a5777361f4cade0f586baf4099f9a6c` |

三包校验通过。都是 dev/Debug 开发制品，不是正式签名发行版；Ubuntu 24.04 amd64 / Android arm64。APK 的开发 v2 签名通过，最终 `librove_gui.so` SHA-256 为 `54680b5b0a218250a183e318666cb51d79f67b3831ae207895f3a372fccac0ae`，前端为 `index-zHsUSwOF.js`。最终只修改 Android 原生 insets 和 manifest，Gradle 复用同一已构建 Rust library 后打包，包内 JNI 哈希一致。

### 实际验证

- Ubuntu：新 runtime/GUI 包隔离安装、同版本升级、身份和归档保留、恢复/删除、保留数据卸载通过。包测试由固定一秒等待改为有界就绪探测，不再将负载下的正常启动误报失败。
- 真实 Linux 原生窗口/systemd：新脚本 `scripts/check-linux-desktop.py` 用独立目录、临时系统网络单元、用户 agent、Xvfb/GTK/WebKit 和可控 loopback 模型验证。界面 device_id 与 CLI 匹配；正常关窗后后台 PID 不变、作业继续完成；重开窗口读取结果，重启 agent 后历史保留。`linux-desktop-v5/result.json` 为 PASS。没有安装宿主全局软件、启用开机服务、修改 lingering 或现有网络；不是所有 Linux 桌面/显卡认证。
- Android：10.1.2.242 的既有 Android 14 arm64 Redroid 已覆盖升级两次。旧 device_id `e6ba18ba-825f-4135-a99e-118485000963`、两条旧会话、并发 3 和中文偏好不变。新页面输入→Rig→工具实际 unsupported→成功作业；真实归档按钮及归档管理恢复通过。临时模型连接的两条型号已清除，留下 `Android 核心验收` 会话与作业供查看，未清空用户数据。
- 实机发现并修复 edge-to-edge 下系统栏遮挡。按 [Android 官方 insets 方式](https://developer.android.com/develop/ui/views/layout/edge-to-edge) 由原生根容器消费系统栏/缺口/IME 空间，整个 WebView 使用安全区域。最终竖屏 360×568、横屏 592×336 无横向溢出，移动底栏仍可见；软键盘弹出高度变为 305，底栏保持在可视区域，收起恢复 568。旋转设置恢复为原值。Android 15+、物理缺口设备与真实相机仍未逐台验收。
- 回归：95 项 Rust、20 项前端单元、32 项浏览器测试；英文标题修复后另有 6 项布局测试；2 项 Tauri IPC 测试通过。51 操作 OpenAPI/GUI-CLI 对等映射、7 规格/故事映射、OpenSpec strict/fmt 通过。API 同步真实 overlay 报告 `api-sync-overlay-v1.json` 保留。

证据：`android-core-acceptance.json`、`android-archive-acceptance.json`、`android-final-upgrade.json`、`android-landscape.json`、`android-keyboard.json`、`linux-desktop-v5/`。修复前包/截图保留用于对照，但不在下载白名单中。下载页仅允许三份最终制品、动态校验清单、两张指定截图；未知路径、数据目录、API 路径和路径穿越均返回 404。

仍未交付：官方账号登录/会话适配、Windows overlay/安装器、Apple 环境及完整平台矩阵；Android native VPN/后台常驻/本机 shell 仍不可用。**不是 Android → Windows 控制验收**。旧版本及旧校验和在下文保留，不用于本轮安装。

## 历史交付：2026-09-17

打包时 `bootstrap-rove` **69/80**，不是全部完成。包目录 `/mnt/data/rove-deliveries/2026-09-17/`，原型预览 `http://10.1.2.237:4173/` 不变，预览不连接正式 agent。

本轮包增加真实多型号配置、默认/可编辑名称、首次网络引导、本机归属的远端会话、简体中文/英文、服务按网络查询及管理草稿、只读存储检查和桌面文件导入。扫码仅移动本机显示，离开页面释放相机；API 当前为 agent 45 / 配置服务 3，SQLite v5。会员登录仍未开放，不使用模拟登录冒充正式认证。在线存储搬迁、多网络与节点测试、Android native VPN/后台常驻仍未交付。

| 文件 | SHA-256 |
| --- | --- |
| `Rove_0.1.0_amd64.deb` | `5d78972f2c6b4ec38f32a201cef367427055f0ad50f9bd16954d31065b1cb229` |
| `rove-runtime_0.1.0_amd64.deb` | `8f92258dedec43226feedb0aedfbf6d4d99e4625c41bdc8c4e0fd5d6d9dcf65f` |
| `rove-android-arm64-debug.apk` | `20adbb14b3aa33b053faf51294ca2edcbc8ec23d5331efa71d932a1323f69c0b` |

三份制品 `SHA256SUMS` 复核通过。Ubuntu 24.04 amd64 dev/Debug 包和 Android arm64 Debug 包都不是正式发行签名。APK v2 签名验证通过，最低 API 24 / 目标 API 36 / versionCode 1000；相机与自动对焦全部声明为可选硬件。最后一次仅修改硬件清单，Gradle 复用已构建 Rust library 重新打包。前端资源为 `index-hlGjRACf.js`；较早日期的包和哈希在下节保留为历史记录。

### 本轮实际验证

- Ubuntu：runtime 与 GUI 两包在一次性 Ubuntu 容器安装、动态库检查、同版本升级、身份和归档保留、CLI 恢复/删除及保留数据卸载通过。未替代真实桌面/systemd 生命周期验收。
- Android：在 10.1.2.242 的既有 Android 14 arm64 Redroid 上 `adb install -r` 成功，没有卸载清数据。旧 device_id、并发设置和成功作业保留，产生三份数据库迁移备份；真实 WebView 切换中文后重启保持，360px 无整页横向溢出。真实页面提交作业，经嵌入 Rig 调用工具得到 `unsupported` 并正常完成；临时模型清除后旧/新作业仍可读取。相机 API 存在，但硬件/权限不可用，已验证中文 URL 降级及视频释放，不声称物理相机扫码通过。结构化结果为同目录 `android-acceptance.json`。
- 跨机：两台不同 Linux 宿主经 EasyTier 完成远端作业、认证服务访问、停网恢复与取消发布保留应用。结果为 `two-host-acceptance-v5.json`；详见 [跨宿主验收](two-host-acceptance.md)。不是 Android → Windows overlay。
- 回归：87 项 Linux Rust 测试、19 项前端单元测试、29 项浏览器测试、2 项 Tauri MockRuntime/真实 socket 测试；Windows 11 另有 21 项原生 agent 测试通过。编译、Clippy/fmt、契约校验与 OpenSpec 校验通过。

Android 仍无本机 shell、native VPN 或后台存活保证；macOS/iOS 缺少验证环境，Windows 网络运行时/安装器仍待完成。安装方法沿用下文，Android 签名不匹配时不要通过卸载绕过。仓库分区满后，Android `app/build` 缓存迁到 `/mnt/data/rove-android-build-backup-20260917-QCoFzi/build`，可完整恢复；源码和交付包未删除。

## 历史交付：2026-09-16

2026-09-16。使用 `openspec-apply-change` 继续 `bootstrap-rove`。这是阶段性开发交付，**不是新版原型全部功能完成，也不是正式发行版**。原型浏览器地址仍为 `http://10.1.2.237:4173/`，它不连接正式 agent。

## 本轮功能

- Vue + PrimeVue 响应式主导航、移动底栏、气泡会话、真实流式事件、Markdown/公式、分段复制和本地朗读。
- 真实会话归档、恢复和确认删除，CLI/API 与 GUI 共用数据；活跃作业阻止删除，历史删除保留请求去重标记。SQLite v3 升级前备份。
- `rove_api` 通用工具复用设备/设置/网络/服务 API，服务页只显示目录、入口和认证说明。取消发布不卸载应用。
- “管理 Rove”创建独立本机会话的只读检查草稿，没有模型时先进入配置。不自动执行，不把提示词当成强制安全机制。

## 未完成的原型差异

- 多模型连接/型号目录、默认和可编辑自动名称、模型同步、首次网络会话引导，以及官方账号登录适配。
- 本机拥有会话并持久关联远端执行；当前会话仍保存在执行设备，切换目标会重建 GUI 组件，草稿尚未跨重启保存。
- 虚拟网段名片、节点逐条测试、不重叠多网络。固定 EasyTier 管理接口开启 DHCP 时丢弃指定网段，不能仅移除单网络保护；需要先确定上游接口修正/升级方案。
- 自动存储迁移及强制影响确认流程。在线数据目录不能由模型直接搬移；系统提示明确要求先说明备份、停机和影响。
- Android native VPN、离开应用后常驻和本机 shell 均未交付；APK 不能据此宣称已能通过 overlay 管理 Windows。Windows/macOS/iOS 不在本次重新打包范围。

## 构建环境

- Ubuntu 24.04 amd64 GNU 开发基线，Rust 1.96.0，EasyTier 2.6.4 固定发行制品验哈希。运行时是 dev profile，Tauri 桌面包是 Debug，不承诺其他发行版 ABI。
- Android arm64，JDK 21、SDK 36、Build Tools 35/36、NDK 27.2.12479018、Gradle 8.14.3。复用本机构建机现有 Debug keystore，不作为正式发行签名。
- 仓库原 `target` 链接已失效，且仓库分区空间不足。本轮使用 `CARGO_TARGET_DIR=/mnt/data/rove-build-20260916-xoZ37q/target`；未替换该链接、删除缓存、修改宿主系统服务或停止 4173。

## 安装与数据保留

Ubuntu 桌面需同时安装 runtime 和 GUI 两个 `.deb`；无图形界面只安装 runtime，使用 `rove` CLI。安装后需要显式启用专用 EasyTier 和用户 agent，步骤见 [Linux 打包说明](linux-packaging.md)。升级前停止 agent 并备份私有数据目录，注意备份含凭据。

Android 在已经连通的 ADB 上使用 `adb install -r <APK>`；选择具体设备时加 `-s <serial>`。若签名不匹配，不要直接卸载以绕过错误，因为会清除应用数据；先核对原签名和备份方案。窗口调试与无界面宿主连接方式见 [Android 测试说明](android-windows-testing.md)。

## 验证边界

本轮已通过 11 项前端单元测试、15 项浏览器 IPC 适配器测试、类型检查/生产构建，以及 2 项 Tauri MockRuntime/真实 SDK socket 测试。浏览器在原生编译高负载下曾有加载超时，单 worker 全量复跑通过。OpenAPI 校验覆盖 39 个操作（agent 36、配置服务 3），OpenSpec strict 校验通过。

最终代码的 73 项 Rust 定向回归、Clippy（warnings denied）和 fmt 通过；另有 1 项 EasyTier 契约测试通过。HTTP 入口补齐 `archived` 布尔查询解析，真实 HTTP SDK 覆盖归档、筛选和恢复，非法布尔值返回 400。

最终 Ubuntu 两包在无网络一次性 Ubuntu 24.04 容器安装通过，GUI 动态库完整；同版本重装后 device_id 和归档记录保留，CLI 恢复/删除成功，卸载保留测试数据。没有启动宿主 systemd 服务，也未冒充真实桌面运行验收。

最终 APK v2 开发签名验证通过，仅包含 `arm64-v8a`；最低 API 24、目标 API 36、applicationId `app.rove.desktop`、versionCode 1000。包内 `librove_gui.so` 与最终构建文件 SHA-256 一致（`10cf2bdbfa4af319d61228e879e71991ae2bac1b55110db89906bb8dc623c9c8`），Ubuntu GUI 与 APK 使用本次前端资源 `index-CXbYGys2.js`。未在手机重新安装验收，不把浏览器 fixture、MockRuntime、包安装或签名校验替代真实 WebView、相机、语音及 Android → Windows 端到端验收。

## 最终制品

本机构建机目录：`/mnt/data/rove-deliveries/2026-09-16/`。没有将二进制、SDK、NDK 或签名私钥提交到 Git。

| 文件 | 大小 | 用途 |
| --- | --- | --- |
| `rove-runtime_0.1.0_amd64.deb` | 28.2 MiB | Ubuntu agent、CLI、EasyTier 与 systemd unit |
| `Rove_0.1.0_amd64.deb` | 11.8 MiB | Ubuntu 桌面 GUI，须同时安装 runtime |
| `rove-android-arm64-debug.apk` | 60.6 MiB | Android arm64 开发包，含嵌入 agent |

SHA-256：

```text
73e6d8a192c3fe897af16f57225131215f414e9679436b2d528326a7ea3ea99a  rove-runtime_0.1.0_amd64.deb
85ba053bfcd2ab3a7e75d3ede404545680695e77196c9370d44f2d9c94af264a  Rove_0.1.0_amd64.deb
c15f704e8bd3af490dc97d61575d3d7e75cb8020d27012c7e0a0bb1cf0ce660f  rove-android-arm64-debug.apk
```

目录内同时提供 `SHA256SUMS`。下载/复制后可运行 `sha256sum -c SHA256SUMS`；该文件用于完整性核对，不是发行者数字签名。
