# Linux 开发包与后台生命周期

当前以 Ubuntu 24.04 / Linux GNU 为开发打包基线，非跨发行版兼容承诺。运行时包包含 `rove`、带 `easytier` feature 的 `rove-agent`、清单验哈希的 EasyTier core/cli，以及两个独立 systemd unit。没有上游 web 服务，没有修改现有 EasyTier 服务。开发 profile 必须在验收记录标明，不称为优化发行构建。

```sh
python3 scripts/package-linux.py --archive /下载路径/easytier-linux-x86_64-v2.6.4.zip
```

脚本只构建包，拒绝覆盖同名包；不会安装、启用服务或删除用户数据。默认无网络构建，需要缓存依赖。架构与 ELF 必须一致。`--profile release` 构建优化版；aarch64 需要对应 GNU 编译工具链。

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

当前必须分别记录包构建、隔离安装、真实 systemd 启停、GUI 退出保持运行、升级和保留数据卸载结果；仅生成 `.deb` 不代表任务 11.1 或平台验收完成。

## 当前开发产物

- 最新 runtime：`target/packages/latest/rove-runtime_0.1.0_amd64.deb`，SHA-256 `55cf9329bab0b45ea7d4f6575947b0b68569f132222251afa569806357278304`。
- Tauri GUI：`target/debug/bundle/deb/Rove_0.1.0_amd64.deb`。
- 都是 dev profile，未签名，不是完整发行版。构建缓存目录是临时存储，不替代正式制品仓库。

可复现隔离包测试：

```sh
python3 scripts/check-linux-package.py target/packages/latest/rove-runtime_0.1.0_amd64.deb --gui target/debug/bundle/deb/Rove_0.1.0_amd64.deb
```
