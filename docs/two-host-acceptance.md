# 两宿主真实 overlay 验收

2026-09-17。`scripts/check-two-host-overlay.py` 第五次完整运行通过，证据：`/mnt/data/rove-deliveries/2026-09-17/two-host-acceptance-v5.json`。前四份失败报告保留，不覆盖失败历史。

## 实际范围

不是同机两个容器：一端在当前 x86_64 Linux 构建宿主，另一端在用户提供的 `10.1.2.242` ARM Linux 宿主。两端各自使用新建、限制为 2 核 / 1 GiB 的隔离容器，不更改宿主路由、系统服务、Android 容器或用户应用数据。ARM 端只临时发布 `10.1.2.242:31010` 到测试 EasyTier 的 TCP 端口；Rove API 与应用入口仅经真实 EasyTier TUN。

Linux ARM agent / CLI 已用固定 Rust 和 GNU 工具链交叉编译；EasyTier ARM 官方归档通过仓库锁定 SHA-256 校验。远端 Ubuntu 24.04 基础镜像摘要为 `sha256:33ceb71981b602c1a7443a53469e4dba065f7503eab3078a2d7a57a2ab987517`。远端下载超时后，在构建机下载同一 arm64 镜像并传输加载，未改 Docker 镜像源配置。

## 通过的闭环

1. 创建网络、导出可移植配置并在另一宿主导入；实际 DHCP 分配、TUN 建立和稳定 device_id 发现。
2. A 通过本机 socket → SDK → overlay HTTP 配置 B 的模型；远端存储检查返回 B 的设备身份。
3. A 创建本机拥有、关联 B 的会话，提交真实远端作业；B 的 Rig 调用 `system_exec` 启动隔离测试服务，再调用 `publish_service` 发布它。
4. A 的本机 CLI 经持久关联读取远端 SSE，作业完成且工具结果与服务目录一致。
5. A 访问 B 的发布入口：带应用认证返回 200；不带认证返回 401。
6. 停止 B 的网络后，A 无法访问旧入口，B 本机应用继续返回 200；恢复网络后沿用原服务监听端口并可重新访问。
7. A 通过远端 API 取消发布，入口不可达，但 B 的本机应用仍返回 200。

可控模型是本地确定性 SSE 测试提供方，不调用收费 API；验证的是实际 Rig/工具/持久化/跨机执行链路，不是供应商模型的推理效果或任意开源应用的安装兼容性。应用本身也是有认证的隔离测试服务。此次不替代 Android → Windows overlay、macOS/iOS、物理相机或 systemd 生命周期验收。

## 排查与安全边界

前两轮在模型请求处失败，第三轮的只读探针确认最小 Ubuntu 容器缺少系统 CA，HTTP 客户端构建失败。正式 runtime 包已依赖 `ca-certificates`。测试补入宿主公开 CA bundle 的只读文件挂载，不挂载私钥或凭据目录、不关闭 TLS 校验。第四轮链路通过至取消发布，测试脚本错误地把合法 204/null 当作字典；修正后第五轮全量通过。

测试结束已删除本次创建的两端容器及容器内测试状态，删除的是隔离夹具数据，不是用户数据。ARM 宿主 `/var/tmp/rove-acceptance-*` 中本轮生成的二进制暂存目录保留，成功轮目录为 `/var/tmp/rove-acceptance-lcq3EB8Q`；这些文件可由当前构建缓存重新生成。临时端口随容器删除而撤销，既有 Android 数据保留。

## 复现

构建 `rove-agent` / `rove-cli` 时启用 `rove-agent/easytier`，并分别构建两个架构的 `peer_acceptance_fixture` example。夹具要求显式 `ROVE_DISPOSABLE_OVERLAY_TEST=1`，仅在新测试容器使用，不进入安装包。

```sh
python3 scripts/check-two-host-overlay.py \
  --host <已授权的远端私网 IPv4> \
  --target-dir <外置 target 目录> \
  --arm-easytier <已验证的 EasyTier ARM ZIP> \
  --remote-image <已缓存的 arm64 镜像 ID> \
  --output <新的验收 JSON 路径>
```

SSH 使用现有密钥；需要密码时通过进程环境 `ROVE_TEST_SSH_PASSWORD` 交给 sshpass，不写源码或报告。脚本拒绝覆盖已有报告，每轮使用新的容器名和暂存目录。保留首次错误与重跑结果，不把启动就绪阶段的预期重试错误当作最终失败。
