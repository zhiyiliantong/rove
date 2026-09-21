# Linux 开发包与后台生命周期

当前以 Ubuntu 24.04 / Linux GNU 为开发打包基线，非跨发行版兼容承诺。运行时包包含 `rove`、带 `easytier` feature 的 `rove-agent`、清单验哈希的 EasyTier core/cli，以及两个独立 systemd unit。没有上游 web 服务，没有修改现有 EasyTier 服务。开发 profile 必须在验收记录标明，不称为优化发行构建。

```sh
python3 scripts/package-linux.py --archive /下载路径/easytier-linux-x86_64-v2.6.4.zip
```

脚本只构建包，拒绝覆盖同名包；不会安装、启用服务或删除用户数据。默认无网络构建，需要缓存依赖。架构与 ELF 必须一致。`--profile release` 构建优化版；aarch64 需要对应 GNU 编译工具链。

仓库分区空间不足时可设置 `CARGO_TARGET_DIR` 为有空间的专用 `target` 目录；运行时打包、原生 GUI 容器检查和 Tauri 打包需使用同一个环境变量。本轮实际缓存位置见交付记录，不替换用户原有 `target` 链接。

桌面 GUI 通过 Tauri 打包，合并 `packaging/linux/tauri.deb.json`，依赖精确版本的 `rove-runtime`。GUI 关闭不管理后台进程。无界面设备只安装 runtime 包，CLI 与 GUI 使用同一 SDK 和协议。

```sh
npm run build --prefix apps/rove-gui
python3 scripts/check-native-gui.py --build
cd apps/rove-gui
node_modules/.bin/tauri bundle --debug --config ../../packaging/linux/tauri.deb.json --ci
```

上述开发流程先在带 GTK/WebKit 的专用容器内编译，再用 Tauri 生成 `target/debug/bundle/deb/Rove_0.1.0_amd64.deb`。正式发行应使用匹配 profile 的 `tauri build`。安装桌面包时同时提供 runtime 包，不覆盖 `/usr/bin/rove`；GUI 二进制是 `/usr/bin/rove-gui`。

## 安装后的显式启用

以下命令属于部署操作，由安装者在目标设备执行，不由构建脚本自动执行：

```sh
sudo dpkg -i rove-runtime_0.1.0_amd64.deb
sudo systemctl daemon-reload
sudo systemctl enable --now rove-easytier.service
systemctl --user daemon-reload
systemctl --user enable --now rove-agent.service
rove status
```

仅为一个选定 OS 账号启用 agent，不能让多个账号共用此专用管理端口。需要退出登录后继续运行时，由管理员显式配置该账号的 systemd lingering。无界面设备也使用指定账号的 user manager，不以 root 身份运行 AI 作业。管理 portal 仅本机可达，但不提供本机不同账号之间的强隔离；设备应由受信任账号使用。

EasyTier 持有 TUN 网络权限，数据位于 `/var/lib/rove-easytier`。Agent 使用当前用户默认私有数据目录，可通过 systemd user override 设置 `ROVE_DATA_DIR`，GUI 和 CLI 必须一致。服务自动失败重启，不自动重提 AI 作业。缺少 TUN、权限或端口被占用时查看 `systemctl status` / `journalctl`；不能退化为物理网卡执行入口。

## 升级与保留数据卸载

升级前停止用户 agent，备份其私有数据目录（包含明文凭据），再更新包并启动。SQLite 迁移有自身备份机制，但不能替代用户备份。网络二进制升级须与 adapter 固定版本同步。

卸载前先停止/禁用用户 agent 和专用网络 unit，再移除包。包没有删除数据的 maintainer script，保留 agent 数据与 `/var/lib/rove-easytier`。不要对用户目录做递归删除，也不要停止不属于 Rove 的网络服务。

必须分别记录包构建、隔离安装、真实 systemd 启停、GUI 退出保持运行、升级和保留数据卸载结果；仅生成 `.deb` 不代表平台验收完成。

2026-09-18 已补齐 Ubuntu 本机生命周期验收：`scripts/check-linux-desktop.py` 启动独立临时 systemd 用户 agent 和系统 EasyTier 单元，用 Xvfb 打开真实 GTK/WebKit 窗口；AT-SPI 校验界面的会话和 device_id 与 CLI 一致，发送正常关窗事件后后台 PID 不变、作业继续完成，重开窗口读取实际输出，重启 agent 后历史保留。模型是只在 loopback 监听的可控 SSE 测试服务，不使用收费凭据。最终报告 `2026-09-18/linux-desktop-v5/result.json`。

测试不安装宿主全局软件、不启用开机服务、不改变 lingering 或现有 overlay；系统网络单元采用包内相同的关键隔离属性、独立端口/目录，结束停止。首次错误脚本缺少 config-dir 的情况已修正；原生 GUI 在构建高负载下曾超时，最终条件不放宽、独立重跑通过。原生 Xvfb 无硬件加速时使用软件渲染，不能据此承诺所有显卡/桌面环境兼容。

```sh
/usr/bin/python3 scripts/check-linux-desktop.py \
  --gui /绝对路径/rove-gui --cli /绝对路径/rove --agent /绝对路径/rove-agent \
  --output /新建的验收目录
```

可选 `--easytier-core /绝对路径/easytier-core` 验证独立系统网络服务（需已授权的系统服务操作身份）。依赖 Xvfb、DBus、AT-SPI/GI、xdotool、ImageMagick；脚本保留测试目录，拒绝覆盖已有输出目录。Ubuntu 包安装/升级/保留数据卸载仍用 `check-linux-package.py`，两层证据不能互相替代。

## 当前开发产物

- 2026-09-18 交付目录：`/mnt/data/rove-deliveries/2026-09-18/`，包含 `rove-runtime_0.1.0_amd64.deb` 和 `Rove_0.1.0_amd64.deb`。此前历史制品保留，最新下载与验收说明在 `http://10.1.2.237:4174/`。
- 哈希、功能范围、安装验证及未完成项见 [Ubuntu / Android 交付记录](ubuntu-android-delivery.md)。不要使用失效 `target` 链接下的历史包当作本次产物。
- 都是 dev profile，未签名，不是完整发行版。交付目录在本机构建机上，仍需正式制品存储和备份策略。

可复现隔离包测试：

```sh
python3 scripts/check-linux-package.py /mnt/data/rove-deliveries/2026-09-18/rove-runtime_0.1.0_amd64.deb --gui /mnt/data/rove-deliveries/2026-09-18/Rove_0.1.0_amd64.deb
```
