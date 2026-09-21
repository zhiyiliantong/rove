# 平台支持与未完成边界

2026-09-18；这是一份开发验收矩阵，不是所有目标平台已正式支持的声明。最新制品见 [交付记录](ubuntu-android-delivery.md)。

| 平台 / 架构 | 本机运行与通信 | 网络与服务入口 | 打包 / 生命周期 |
| --- | --- | --- | --- |
| Ubuntu 24.04 / x86_64 | CLI、独立 agent、Unix socket 与真实 GTK/WebKit 窗口已验证 | 原版 EasyTier 自动/手动模式、不重叠多网络及 API 同步；双网实际 TUN/API/服务独立停止测试通过 | 最新 runtime / GUI deb、安装升级保留卸载、真实 systemd 临时系统/用户服务与关窗继续/重开恢复通过；未测试整机重启/登录退出/所有桌面环境 |
| Windows 11 / x86_64 | 本轮 21 项原生 agent 测试通过；受保护 named pipe、同账号客户端、匿名拒绝；早期 GUI 实机链路已验证 | **尚无正式 Windows overlay runtime**；不能用 Linux SO_BINDTODEVICE 的实现外推 | GNU 交叉构建与早期便携包；系统网络服务与安装器未完成 |
| Android 14 / arm64 | 新 APK 实际 WebView → SDK → Unix socket；真实提交、Rig unsupported、归档恢复、语言与重启持久化通过 | **没有 native VPN / 后台常驻**；不等于已能管理 Windows | 最新 Debug APK 覆盖升级保留身份/设置/历史；原生安全区域与横竖屏/软键盘已验。物理相机、Android 15+ 和物理缺口设备未测 |
| macOS | 源码有 Unix/桌面分支，**未验证构建和原生运行** | 原生接口隔离和系统网络服务未完成 | 无本轮可验收 bundle、launchd 或升级记录；需要 Mac / Xcode |
| iOS | 同一核心的目标设计，**未完成 Xcode 构建及原生验收** | native VPN / 后台扩展不在首版壳的交付声明中 | 需要 Mac / Xcode 和设备或模拟器；不能由 Android APK 成功推断 |

## 进程与权限

桌面 GUI 与 CLI 是客户端，不拥有独立 agent 的作业寿命。Linux 安装资源分别注册用户 agent 与系统 EasyTier；安装包生成不自动启用主机服务。Windows 目前实际验证独立进程与 named pipe，不假装已有 SCM 安装器。移动端 agent 寿命依赖应用，离开应用后后台执行没有保证。

Windows 本机管道使用仅当前进程用户 SID 的 protected DACL 和拒绝远程客户端选项。新增测试在独立线程临时模拟匿名身份，得到真实拒绝；未创建账号或改变系统权限。它不是 Windows 远端 API 的安全验证。Windows 远端入口仍需可验证的虚拟接口入站边界、失败关闭、匹配驱动及后台网络服务，不能只绑定一个虚拟 IP 就开放设备控制。

## 测试环境

- 构建宿主 systemd 系统/用户 manager 已用独立临时单元完成网络服务与原生窗口联合验证，测试后停止；未安装宿主全局包、启用开机服务或改变 lingering。隔离包安装与真实运行分开记录。
- Windows 测试机可连接，当前原生测试使用单独测试目录，无系统服务或防火墙变更。
- ARM Android 宿主可连接，既有纯 64 位 Redroid 容器保持 2 核 / 4 GiB 限制，未改动其他容器。ADB 通过 SSH 转发访问宿主 loopback，不开放公网 ADB。
- 没有已提供的 Mac / Xcode / iOS 测试环境。相关任务保持未完成；不得把缺失平台移出清单来制造 80/80。
- 两台独立 Linux 宿主（x86_64 / ARM）已完成真实 overlay 装配与服务恢复闭环，见 [跨宿主验收](two-host-acceptance.md)，不外推为其他平台网络支持。

用户澄清 DHCP 与手动网段是独立模式后，不再需要 DHCP 补丁；见 [地址管理边界](easytier-addressing.md)。macOS 环境按用户要求暂缓。原型网页、浏览器 IPC fixture、MockRuntime、软件包生成、真实 WebView 与跨设备 overlay 是不同层次证据，验收记录分别标明。
