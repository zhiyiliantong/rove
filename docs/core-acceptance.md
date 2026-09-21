# 核心功能验收 · 2026-09-18

验收入口：[下载与实机截图](http://10.1.2.237:4174/)。这是指定制品的只读下载页，不是正式浏览器客户端，也没有开放 agent API。

[4173 原型](http://10.1.2.237:4173/) 仍是之前验收的独立演示；正式功能请安装本轮应用体验。任务 **73/82**，原型功能迁移进入用户验收，未完成平台/会员功能仍在原任务中保留。

## 安装

Ubuntu 24.04 amd64 桌面安装两个 deb，无界面主机只安装 runtime。首次启用步骤见 [Linux 安装与生命周期](linux-packaging.md)。已有数据先停 agent 并备份再更新，不能清空目录来代替升级。

Android arm64 使用 `adb -s <设备> install -r rove-android-arm64-debug.apk`。10.1.2.242 上的既有 Android 测试环境已升级，打开 Rove 即可查看。ADB 通过 SSH 隧道，scrcpy 显示在调试电脑；见 [Android 调试说明](android-windows-testing.md)。签名不匹配时不要卸载绕过。

三包均为 dev/Debug 开发制品，非正式签名发行版。准确哈希、设备记录和平台边界见 [交付记录](ubuntu-android-delivery.md)。

## 建议你检查

1. 中文/English 切换、桌面侧栏、手机底栏、亮暗主题；Android 顶部与底部不被系统栏覆盖，横屏和键盘弹出后操作仍可见。
2. 添加自己的 API 连接，一次选择多个型号，修改自动名称、默认型号和会话选择。没有实际账号适配器的登录方式仍明确不可用。
3. 创建会话、真实流式应答、代码/公式/说明复制；归档后在归档管理恢复，确认删除只针对已归档且无活动作业的历史。
4. Ubuntu 创建/加入网络：自动分配与手动网段二选一，多个初始节点可逐条测试；多个网络实际网段不得重叠或包含。节点 TCP 可达不等于入网认证成功。
5. 已联网 Linux 设备之间，在模型页同步本机 API 配置；首次保存、重复同步和覆盖确认符合预期。API 配置与账号登录不同，后者不进行跨设备复制。
6. 对话管理 Rove、部署用户选择的第三方开源服务；服务页面查看入口和认证信息，不恢复旧的服务管理按钮。破坏性影响说明是对话约定，不是硬权限审批。
7. Ubuntu 关闭 GUI 后作业继续，CLI 与重开界面能看到同一作业。Android 仍不承诺退出后后台存活，不能把桌面行为外推到手机。

## 已完成的自动与原生检查

- 95 项 Rust 核心/CLI/SDK/协议回归，20 项前端单元，32 项正式 GUI 浏览器适配器测试；英文标题布局调整后另跑 6 项相关测试；2 项 Tauri IPC/真实 socket 测试。
- 51 操作 OpenAPI 和 GUI/CLI 映射、7 份规格/用户故事映射、严格 OpenSpec 校验。
- Ubuntu 包安装/升级/保留数据卸载、真实 WebKit/systemd 关窗继续与重开恢复。可复现脚本 `scripts/check-linux-desktop.py`，独立临时服务/目录/DBus 会话，不启用宿主开机服务。
- Android 14 实际 WebView 提交作业、Rig 工具真实返回 unsupported、归档恢复、两次覆盖升级后旧身份/配置/历史保留，横竖屏安全区域。
- 两宿主真实 EasyTier 执行/服务与双网络 API 同步的已有报告继续适用；不将其写成 Android→Windows 验收。

可控模型用于验证实际 SDK/Rig/事件链路，不代表收费供应商推理、会员授权或智能部署决策已验收。临时模型已清除，Android 保留 `Android 核心验收` 会话供查看，未删除旧用户会话。

## 明确未完成

官方账号登录与执行适配、Windows overlay 与安装器、完整跨平台安装矩阵、物理相机扫码/系统朗读/其他平台原生打开服务验收。macOS/iOS 按约定暂缓。Android native VPN、后台常驻与本机命令没有交付，因此尚不能验收 Android → Windows overlay 控制。

## 下载入口的运行边界

`scripts/serve-delivery.py` 只允许三份固定包、SHA-256 清单和显式选择的截图；任意其他路径（含数据目录与路径穿越）返回 404，没有目录索引。默认监听 loopback；本次按既定 LAN 地址显式绑定 `10.1.2.237:4174`，由临时用户服务 `rove-acceptance-downloads-20260918` 托管，未设开机自动启动。

```sh
systemctl --user status rove-acceptance-downloads-20260918
# 验收结束后可停止下载页，不影响 Rove 或 4173 原型：
systemctl --user stop rove-acceptance-downloads-20260918
```
