# 固定依赖与发行来源

开发基线日期：2026-09-09。下面是已经解析/检查的版本，不宣称是最新版本。依赖升级须同步 lockfile、契约检查和受影响的平台验收。

| 部件 | 固定版本 | 来源与约束 |
| --- | --- | --- |
| Rust | 1.96.0 | rust-toolchain.toml；edition 2024；本机 Linux x86_64 已运行 |
| Node | 22.22.2 | 根目录 .nvmrc；前端 CI 同版本 |
| Rig | rig-core 0.42.0 | Cargo 精确版本；Chat Completions 流接口，由 Rove 保存会话和调度工具 |
| EasyTier | 2.6.4 / 8428a89d2dabc94c97d370ec607c6ca142473626 | Git revision 固定；管理客户端仅启用 zstd，不编入 TUN 数据面；配套 core 独立运行 |
| Tauri | Rust 2.11.5 / build 2.6.3 | Cargo 精确版本；原生平台依赖另行验证 |
| Tauri JS / CLI | 2.11.1 / 2.11.4 | npm 精确版本，package-lock.json 固定平台包 |
| Tauri Opener | Rust / JS 2.5.5 | 精确版本；仅授权 HTTP/HTTPS 默认浏览器打开，不授权文件或指定程序 |
| CLI 浏览器交接 | open 5.4.3 | 精确版本；仅在显式 service open 时调用；无图形会话明确失败 |
| Vue / Vite | 3.5.42 / 8.2.2 | npm 精确版本 |
| TypeScript / vue-tsc | 5.9.3 / 3.3.11 | npm 精确版本；Vue 编译插件 6.0.8 |
| 二维码 | qrcodegen 1.8.0 / jsQR 1.4.0 | SDK 编码，GUI 本机解码；测试独立使用 quircs 0.10.3 |
| 隐藏终端输入 | rpassword 7.5.4 | CLI 私密提示；PTY 已验证不回显 |
| Windows API | windows-sys 0.61.2 | 当前账号 SID、受保护 DACL 和本机 named pipe；不增加网络成员 RBAC |

所有 Rust 传递依赖、registry 校验和与 git revision 由根 Cargo.lock 固定。不得单独替换 EasyTier core、RPC 依赖或禁用 zstd 后宣称兼容；较大管理响应实际使用压缩。Tauri 的版本号在 Rust crate、JS API 和 CLI 间并不完全相同，必须作为此处的已检查组合更新。

## EasyTier 二进制证据

现有 `/usr/local/bin/easytier-core` 与 `/usr/local/bin/easytier-cli` 的版本输出是 `2.6.4-8428a89d`，文件类型为静态链接 ELF Linux x86_64。2026-09-09 从官方发行重新下载 Linux x86_64 归档，校验整个 ZIP 的 SHA-256/大小后，在不解压安装或执行的情况下读取两个成员哈希，与现有文件完全一致。本轮没有替换或重新安装它们，也尚未将它们纳入 Rove 安装包。哈希一致不是上游数字签名或可复现构建证明：

| 文件 | SHA-256 |
| --- | --- |
| easytier-core | 35b854e0c5853c02733870488c977200f6aa06d09c08d5dc7f9f9e150e7be8d5 |
| easytier-cli | 8da66644ec50dcf618fccf76780991f52dfbe2939797abc0efe0ef4d4c4d3383 |

发行包应从 [EasyTier v2.6.4 官方发行](https://github.com/EasyTier/EasyTier/releases/tag/v2.6.4) 获取匹配目标架构的产物，或从固定 revision 构建并记录构建参数、许可证和产物校验和。Windows/macOS/ARM 的配套二进制、签名和安装行为尚未验收，不能由 Linux 的成功外推。

两个隔离实例的实际管理 RPC 已通过（创建、运行状态、peer 列表、删除）；它们没有虚拟地址，也没有进行外部连接。Rove 每设备同时只加入一个网络，使用原版 DHCP，不再要求指定 CIDR；上述多实例探针仅验证上游接口，不代表产品支持同机多网络。见 [地址管理边界](easytier-addressing.md)。

### 发行资产锁

[资产清单](../packaging/easytier-assets.json) 记录官方 release ID、固定 revision，以及 Linux/macOS/Windows 的 x86_64/aarch64 共六项归档名称、大小和 SHA-256；Windows 上游将 aarch64 命名为 arm64。数据来自 [官方 release API](https://api.github.com/repos/EasyTier/EasyTier/releases/tags/v2.6.4)，不是自动跟随 latest 的下载入口。六项归档均已下载并核对大小与哈希；没有执行或安装其他平台程序，不代表支持矩阵已经验收。

`python3 scripts/check-release-assets.py` 检查清单与 Cargo revision 一致；加 `--archive <本机归档> --asset <清单文件名>` 可离线校验大小和哈希。脚本不下载、解压、安装或执行程序。Linux 归档含 core/cli/web/web-embed，Rove 后续组装仅选择所需 core/cli，不启动额外 Web 管理服务；随包许可证与目标平台驱动仍属于安装器交付项。

与 Rove 相关的 [v2.6.4 发行说明](https://github.com/EasyTier/EasyTier/releases/tag/v2.6.4)：修复 Machine ID 持久化和长期运行资源泄漏，改善重连错误区分；Windows UDP 广播中继默认关闭且需系统权限。Rove 仍保存独立 device_id，不以这些修复替代业务身份或自动开启广播转发。

### Windows 本机 transport 检查

`x86_64-pc-windows-gnu` 的 agent/CLI 交叉检查与 agent 全目标 Clippy 已通过；依赖 C 编译使用现有 MinGW 工具链。管道服务端依据 [Microsoft named pipe 安全模型](https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights) 设置当前账号 SID 的显式受保护 DACL，通过 Tokio 拒绝远程客户端；客户端的 PIPE_BUSY 等待受原有握手期限约束。

`.github/workflows/check.yml` 新增 Windows 本机测试任务，但本环境无 Windows/Wine，未实际执行该 CI。仍需 Windows 多账号拒绝验证、用户数据目录 ACL、后台服务注册和安装/卸载验收；不能把交叉编译成功当成完整 Windows 支持。

## Linux 原生检查环境

`packaging/linux/Dockerfile.check` 固定 Ubuntu 24.04 基础镜像 digest，并在开发专用镜像中安装 GLib/GTK3/WebKitGTK4.1 和 C 构建依赖。apt 安装解析不是产品依赖锁；该镜像仅用于原生编译验证，不代替安装包或桌面运行验收，也不升级主机图形库。

应用窗口图标以仓库中的 SVG 为源，经固定 Tauri CLI 的 `tauri icon` 生成；生成移动平台图标不表示已经实现移动核心或 VPN 能力。

服务卡片使用 [Tauri 官方 Opener 插件](https://v2.tauri.app/plugin/opener/)，关闭自动链接拦截，仅在用户点击按钮时调用。Vue 拒绝非 Web 协议和 URL 内嵌账号密码；原生 capability 只允许主窗口的 HTTP/HTTPS 默认应用，不授权 `open_path` 或任意应用程序选择。此处是 UI/OS 调用边界，不引入 Rove 网络成员权限角色。
