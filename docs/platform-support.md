# 平台支持与未完成边界

更新于 2026-09-24；这是一份开发验收矩阵，不是所有目标平台已正式支持的声明。Linux/Android 制品见 [交付记录](ubuntu-android-delivery.md)，Windows 安装器见 [Windows 安装包与 240 实机测试](windows-installer-delivery.md)。

| 平台 / 架构 | 本机运行与通信 | 网络与服务入口 | 打包 / 生命周期 |
| --- | --- | --- | --- |
| Ubuntu 24.04 / x86_64 | CLI、独立 agent、Unix socket 与真实 GTK/WebKit 窗口已验证 | 原版 EasyTier 自动/手动模式、不重叠多网络及 API 同步；双网实际 TUN/API/服务独立停止测试通过 | 最新 runtime / GUI deb、安装升级保留卸载、真实 systemd 临时系统/用户服务与关窗继续/重开恢复通过；未测试整机重启/登录退出/所有桌面环境 |
| Windows 11 / x86_64 | 受保护 named pipe、同账号客户端、匿名拒绝；r16 普通用户 GUI 自动启动 agent，原身份保留 | 应用内 EasyTier/Wintun、独立提权 helper、动态 WFP 隔离；与 Android 双向发现和 Rove 操作、停止失效及物理网负向通过 | r16 NSIS 覆盖安装于 240，驱动/二进制哈希匹配，交互桌面 GUI 已验；不注册 SCM、不写 PATH，仍为开发测试版 |
| Android 14 / arm64 | 实际 WebView → SDK → Unix socket；身份/配置持久化，仍不提供本机 shell | 应用内 EasyTier + VpnService，双网络独立启停/撤销恢复；与 Windows 双向 Rove 操作通过 | r16 APK 覆盖安装于 Redroid；前台 VPN 服务已验，后台仍受系统限制；未把容器/响应式结果外推为实体手机、软键盘、物理相机或 Android 15+ 验收 |
| macOS | 源码有 Unix/桌面分支，**未验证构建和原生运行** | 原生接口隔离和系统网络服务未完成 | 无本轮可验收 bundle、launchd 或升级记录；需要 Mac / Xcode |
| iOS | 同一核心的目标设计，**未完成 Xcode 构建及原生验收** | native VPN / 后台扩展不在首版壳的交付声明中 | 需要 Mac / Xcode 和设备或模拟器；不能由 Android APK 成功推断 |

## 进程与权限

桌面 GUI 与 CLI 是客户端，不拥有独立 agent 的作业寿命。Linux 安装资源分别注册用户 agent 与系统 EasyTier；安装包生成不自动启用主机服务。Windows 每用户安装器包含独立网络 helper，不注册 SCM 服务、不写 PATH，不保证开机无人值守运行。移动端 agent 寿命依赖应用；Android VPN 使用前台服务，但离开应用后仍受系统限制。

Windows 本机管道使用仅当前进程用户 SID 的 protected DACL 和拒绝远程客户端选项。匿名身份拒绝测试不替代远端安全验证；r16 另以原生接口事实、动态 WFP 和 overlay 源地址绑定验证入口隔离，实际正向/负向证据见 [原生运行时记录](native-overlay-implementation.md)。

## 测试环境

- 构建宿主 systemd 系统/用户 manager 已用独立临时单元完成网络服务与原生窗口联合验证，测试后停止；未安装宿主全局包、启用开机服务或改变 lingering。隔离包安装与真实运行分开记录。
- Windows 测试机（240）安装到 `%LOCALAPPDATA%`，未启用系统服务、未改 PATH/Mihomo/全局防火墙策略。网络 helper 创建限定 overlay 的规则，上游 EasyTier 另创建自身程序规则；普通端口探针仍不能证明组网，r16 以实际双端身份、设备发现、Rove 操作和停止/物理网负向为证据。
- ARM Android 宿主可连接，既有纯 64 位 Redroid 容器保持 2 核 / 4 GiB 限制，未改动其他容器。ADB 通过 SSH 转发访问宿主 loopback，不开放公网 ADB。
- 没有已提供的 Mac / Xcode / iOS 测试环境。相关任务保持未完成；不得把缺失平台移出清单来制造 80/80。
- 两台独立 Linux 宿主（x86_64 / ARM）已完成真实 overlay 装配与服务恢复闭环，见 [跨宿主验收](two-host-acceptance.md)，不外推为其他平台网络支持。

用户澄清 DHCP 与手动网段是独立模式后，不再需要 DHCP 补丁；见 [地址管理边界](easytier-addressing.md)。macOS 环境按用户要求暂缓。原型网页、浏览器 IPC fixture、MockRuntime、软件包生成、真实 WebView 与跨设备 overlay 是不同层次证据，验收记录分别标明。
