# 独立地址模式与多网络

2026-09-17，按用户澄清实现，不修改 EasyTier 源码、固定版本或制品。

## 用户入口

正式 GUI 创建/编辑网络提供名称、随机十六进制密钥及显隐/重新生成、地址模式、多条初始节点和单项测试。自动模式由 EasyTier 分配；手动模式填写网段及此设备 IP，可先保存再补地址。创建后导出四字段名片，完整 JSON 仍提供稳定 ID 和版本；URL/扫码/桌面文件/手动 JSON 继续复用同一导入入口。机密只在显式名片/导出显示，不进入普通列表。

```sh
rove network create home
rove network create work --subnet 192.168.100.0/24 --local-ipv4 192.168.100.2
rove network test-peer tcp://example.invalid:11010
rove network export <network_id>
rove network update <network_id> --body -
```

`update` 输入仍为 `display_name`、`easytier` 和可选 `local_ipv4`。省略本机地址保留原值；切回自动模式则清除。导入手动配置后没有本机 IP，启动返回 `local_address_required`，不会复用分享设备 IP。手动地址仍需用户保证网络内每台设备不同；不引入 Rove 中央地址分配器。

## OpenAPI 与运行方式

- 新增 `POST /v1/network-probes` / `test_network_peer`；总计 agent 46 + 配置服务 3 个操作。socket、HTTP、SDK、GUI 和 CLI 走同一实现。
- 探测最多并行 4 个，每个 3 秒。TCP/WS/WSS 仅建立 TCP 连接，不发送网络密钥、不验证 TLS/应用握手，也不创建网络；UDP/QUIC/WG 返回 `unsupported`，不把发送成功当作可达。
- `EasyTierJoinConfig.dhcp=false` 使用指定 `ipv4_cidr`；`NetworkCreate` / `NetworkUpdate` 的 `local_ipv4` 只属于本机，不进入分享。Network 增加可选实际 `overlay_cidr` 和本机 `listener_url`。旧数据库记录无需重置，旧自动模式 CIDR 仍是兼容元数据。
- Linux driver 从单一 endpoint 改为按 network_id 管理。启停串行协调，每个网络独立保有接口索引/IP/实际网段、HTTP/SSE 和服务代理。其他应用的 EasyTier 实例不被接管或删除。
- 实际网段包含关系也拒绝。静态冲突在启动前返回 409；另一自动网段未知返回 `network_address_pending`。新 DHCP 网络产生冲突时停止其实例、保存 `network_subnet_conflict`，由用户显式停止/修改/重试，不自动换池。仍绑定 TUN 并检查主机具体路由；完全信任成员不是跨网络设备权限隔离。
- 实例不能共用一个 TCP 监听端口：首个实例可使用部署端口，其余申请空闲端口，记录实际 listener_url 供重启。端口被外部进程占用时报告失败，不接管。监听通配地址不是可分享的初始节点，需要配置接收设备可达的地址。

## 验证与边界

`scripts/check-multinetwork.py` 在两个一次性 Linux 容器中使用真实 EasyTier TUN。每端两个手动网段同时运行，双网各自发现同一稳定 device_id、SDK 远端调用成功；停止一个网络不影响另一个网络的 API 和带认证的测试服务；恢复保留对应监听端口。另验证 `/16` 包含 `/24` 在启动前拒绝，以及原版 DHCP 根据 peer 分配出重叠网段时新入口拒绝并停实例。无宿主路由、系统服务或用户数据改动；运行后仅删除脚本自己的容器。

证据：`/mnt/data/rove-deliveries/2026-09-17/multinetwork-v2.json` 与最终代码 `multinetwork-v3.json` 完整通过。v1 业务链路通过，但测试脚本错误地从 stdout 读取 CLI 错误（实际在 stderr），因此保留为失败报告，修正后重跑，不覆盖先前报告。

浏览器 30 项测试通过，含真实 QR 像素解码、地址模式、密钥显隐、逐节点测试、名片和 320px 布局；单元测试包含子网规范化与旧分享保留。原生物理相机未新增验证。Linux 多网络测试不等于 Windows 网络运行时或 Android native VPN；此前 Android 安装包没有本次网络增量，未重新打包时不得把它当作新版验收。

macOS/Xcode 环境按用户要求暂不搭建，相关任务仍保留未完成。
