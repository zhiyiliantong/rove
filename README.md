# Rove · 漫游者

对等设备漫游应用。GUI 使用 Tauri 2 + Vue，命令行是 `rove`，每台设备的运行时是 `rove-agent`。

当前是**开发版，不是完整的跨平台发行版**。已实现设备身份、配置持久化、OpenAPI 驱动的 socket、网络配置与二维码加密分享、Rig 流式 AI、会话并发、取消、事件订阅，以及服务登记和 TCP 代理。Linux 的可选 EasyTier 运行时已接通实际 TUN、设备发现、远端控制与服务发布；默认构建仍只启用本机运行时。不会开放物理网卡来替代 overlay。

## 本地运行（Linux）

需要 Rust 1.96.0；GUI 前端另需 Node 22.22.2 / npm。首次构建会下载依赖。

```sh
cargo build --locked
cargo run --locked -p rove-agent -- --data-dir .rove
```

保持 agent 运行，在另一个终端调用。所有客户端使用相同数据目录；关闭 CLI 不会关闭 agent。

```sh
cargo run --locked -p rove-cli -- --data-dir .rove status
cargo run --locked -p rove-cli -- --data-dir .rove --json config show
cargo run --locked -p rove-cli -- --data-dir .rove config set --max-active-runs 6
cargo run --locked -p rove-cli -- --data-dir .rove network create "家庭网络"
cargo run --locked -p rove-cli -- --data-dir .rove network list --limit 20
cargo run --locked -p rove-cli -- operations
```

不带子命令的 `rove` 打开交互菜单，密钥输入不回显；无 TTY 时明确退出。已有 `network/device/model/config/session/run/service/agent` 命令组，`--help` 查看参数。`call <operation_id>` 保留为通用入口。敏感 JSON 用 `--body -` 从 stdin 传入，分享 URL 用 `network join -`；避免密钥进入 shell history 或进程参数。失败返回非零退出码，诊断写 stderr，`--json` 输出不混入诊断。

远端目标必须同时指定 `--network <network_id> --device <device_id>`。普通输出显示目标，远端不可达不在本机执行。`service list` 需要 `--network-id <network_id>`；`service publish/update` 接受 OpenAPI `ServiceWrite` JSON。

`service open <service_id> [--endpoint 0]` 把已发布的 HTTP/HTTPS 入口交给当前设备默认浏览器，不会在服务提供设备上打开。无图形会话时用 `service show` 取得地址；`open_requested` 仅表示已请求 OS 打开，不保证浏览器已连接成功。

个人可以保存和分享多个网络，但每台设备同时只加入一个。网络创建与导入只保存配置，并不代表已联网；切换须先停止当前网络，再启动另一个。地址分配交给原版 EasyTier DHCP，新配置不指定地址池。未配置网络运行时返回 `overlay_unavailable`；已有未退出网络时返回 `network_already_joined`。

Linux 真实组网需构建 `cargo build --locked -p rove-agent --features easytier`，启动专属于 Rove 的 EasyTier 2.6.4 管理进程（TUN 所需系统权限由该进程持有），再为 agent 指定 `--easytier-portal 127.0.0.1:15888 --easytier-core /实际路径/easytier-core`。管理进程只能监听本机，不能复用含其他实例的管理入口。启动接受为 `starting`，只有实际地址和限定 TUN 的 HTTP 入口就绪才为 `running`；无 peer 时可能等待 DHCP。可复现的隔离验证见 `scripts/check-overlay.py`，它使用两个自动清理的容器，不代表物理两设备或安装包验收。

## 本机 AI

先通过 `set_model_config` 设置本设备模型。当前支持 `openai_compatible` / `openai` 的 Chat Completions 接口，既可连接本机兼容服务，也可连接用户提供的远端模型地址。不要把密钥放进命令参数；使用 `rove call set_model_config --body -` 从 stdin 读取 JSON，或使用 GUI 密码输入。

```sh
cargo run --locked -p rove-cli -- --data-dir .rove session create --title "装配任务"
cargo run --locked -p rove-cli -- --data-dir .rove session send <session_id> "查看本机系统信息"
cargo run --locked -p rove-cli -- --data-dir .rove run watch <run_id>
cargo run --locked -p rove-cli -- --data-dir .rove run show <run_id>
cargo run --locked -p rove-cli -- --data-dir .rove run cancel <run_id>
```

每个会话 FIFO，不同会话默认最多四个作业同时运行；`max_active_runs` 调低不会取消已经运行的作业。单个作业工具顺序执行，不同作业命令可并发。`system_exec` 使用 agent 当前 OS 身份执行真实 shell 命令，可能改变本机系统；`publish_service` 需要目标设备已有就绪的 overlay 入口。

关闭 `run watch` 只关闭观察，不取消作业；重启 agent 后旧队列和未完成作业标记为 `interrupted`，不会自动重跑。提交可通过 `--request-id` 保持重试幂等。

本机 SDK 握手最多等待 5 秒，普通操作响应最多等待 30 秒；事件订阅成功后不限制整个作业观察时长。响应超时表示结果未知，不代表任务没执行；不要生成新 `request_id` 重提同一作业。

运行时资源上限：1,024 条等待作业、每个 run 保留最近 4,096 个事件、65,536 字符显示尾部、每个模型循环最多 32 步；命令默认 300 秒，显式超时最大 86,400 秒。取消不回滚已经完成的安装或系统变更。

## 密文分享服务

```sh
cargo run --locked -p rove-config-server -- \
  --database .rove/blobs.db \
  --listen 127.0.0.1:43191 \
  --public-url http://127.0.0.1:43191
```

用 `config set --config-server <url>` 设置地址，再执行 `network share <network_id> --qr`。二维码与分享 URL 包含同一密钥；GUI 网络面板可展示二维码，使用本机相机解析后填入同一 URL 导入入口。真实相机权限和移动原生运行尚未验收。完整流程见 [配置服务说明](docs/config-server.md)。

## GUI

```sh
cd apps/rove-gui
npm ci
npm run build
npm run tauri dev
```

先启动同账号的 agent。默认数据目录为 `$XDG_DATA_HOME/rove` 或 `$HOME/.local/share/rove`；可为 agent 和 GUI 同时设置 `ROVE_DATA_DIR` 指向同一个绝对路径。GUI 通过 Tauri → SDK → 本机 socket 调用，不直接写数据库。

Vue 构建可独立验证；原生 Linux Tauri 还需要 GTK 3、WebKitGTK 4.1 等系统依赖。浏览器直接运行 Vite 不具有 Tauri IPC，不能替代原生客户端验收。

Linux 有 Docker 且 Rust 依赖已缓存时，可用 `python3 scripts/check-native-gui.py --build-image` 在专用容器中检查原生壳，避免安装主机图形依赖；后续复用镜像可省略 `--build-image`。加 `--test` 会链接并运行 Tauri MockRuntime IPC 测试：使用真实 socket agent 和生产配置，但不启动真实 WebView。这不是桌面运行或安装包验收。固定依赖和二进制来源见 [依赖记录](docs/dependencies.md)。

GUI 已有目标设备选择、网络分享/导入、模型设置、会话/作业、服务表单。作业输出经 SDK 有界事件批次传递（每批最多 64 条，等待 750ms）；断线从快照恢复，切换会话丢弃旧响应，关闭窗口不会取消作业。刷新保留已加载分页，新提交立即显示；服务卡片可以复制入口或将 HTTP/HTTPS 地址交给默认浏览器，纯 TCP 服务使用对应客户端。真实 WebView、多窗口、相机和浏览器打开仍待桌面验收。

## 检查

```sh
cargo fmt --all --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
python3 -m venv .venv
.venv/bin/pip install -r scripts/requirements.txt
.venv/bin/python scripts/check-contracts.py
cargo build --locked -p rove-cli -p rove-agent --bins
python3 scripts/check-cli-tty.py
npm test --prefix apps/rove-gui
```

`Cargo.lock`、GUI 的 `package-lock.json` 固定解析版本；默认 Cargo 成员不包括 GUI 原生壳，因此无界面环境可以单独构建和测试。

首版阶段需求与设计已归档，见 [bootstrap-rove](openspec/changes/archive/2026-09-21-bootstrap-rove/README.md) 和 [归档任务列表](openspec/changes/archive/2026-09-21-bootstrap-rove/tasks.md)。现行行为契约见 [主规格](openspec/specs/)，现行接口见 [api](api/README.md)。后续按 [小提案计划](docs/next-iterations.md) 迭代；实际验证范围见 [实现记录](docs/implementation-status.md)。
