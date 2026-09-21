# API 配置同步与逐设备账号登录

2026-09-18。按用户确认：API 可以同步，会员账号各设备单独官方授权。API 同步已接入正式代码；账号授权/官方 Agent 会话适配仍在任务 14.2，不能用本功能冒充已支持会员登录。

## 使用

正式 GUI：本机网络列表 → 某个运行中网络的“同步本机 API 配置” → 选择来源连接 → 选择该网络内设备 → 确认同步。弹窗固定网络，不可切换到其他网络；模型页与远端网络配置页不提供此入口。只显示有 API 密钥的连接，不读取密钥到前端。此入口不是 4173 浏览器演示原型。

```sh
rove model list
rove model sync CONNECTION_UUID --to-network NETWORK_UUID --to-device DEVICE_UUID
# 已有内容变化，明确确认覆盖；可指定目标已有连接：
rove model sync CONNECTION_UUID --to-network NETWORK_UUID --to-device DEVICE_UUID --replace-connection TARGET_CONNECTION_UUID --yes
```

一次同步一个连接及其全部型号，包含提供商、接口地址、API 密钥、型号字符串和配置名称；不复制来源内部模型 UUID 或默认选择。无密钥的本地模型连接不在此 API 密钥同步入口范围。CLI 的全局 `--network/--device` 若使用，选择来源 agent；`--to-network/--to-device` 则始终选择接收设备。默认来源为本机。

同步是一次性复制，不是双向持续同步。来源改名、换密钥、删除或离线后，目标仍持有独立配置。撤销供应商密钥会影响持有该密钥的设备；Rove 删除连接不等于供应商撤销。

## 一致性、冲突与敏感信息

- 私有来源标记为来源 device_id + connection_id。相同来源和内容重复请求不产生副本；有变化则返回 409，用户显式确认才替换。
- 也可指定一个目标现有连接，但必须确认覆盖其密钥、接口地址及全部型号。同名型号位于其他连接时仍冲突，不能顺手删除无关连接。
- 替换保留仍存在型号的目标 model_id，已开始作业使用原快照；删除的型号不再可用，引用它的会话需重新选择。目标默认型号仍存在则保留，否则清空，不自动设为来源默认。
- 接收端使用既有 SQLite 事务原子提交，失败不保留半份配置。来源读取后释放数据库锁再等待网络，修改来源不影响已传出的快照。
- 新增私有 JSON 元数据，不提高数据库 schema 版本，不重置旧目录、模型、会话或身份。
- GUI/CLI 的同步请求只含连接 ID、目标和确认信息；密钥由来源 agent 直接经 SDK/overlay 发送。目录响应不返回 api_key 或私有来源标记。远端错误正文不透传，避免其中包含密钥。
- 中断可能发生在目标提交后，因此报告结果未知而非保证失败；检查目标或重试未改变的来源快照。没有本机回退或自动同步。
- 网络加入不触发同步，网络名片不包含模型配置。完全信任网络的既有设计不变；本机制不承诺对可控制设备的网络成员隐藏设备文件。

## OpenAPI

新增两项操作，总计 agent 48 + 配置服务 3 = 51：

| 操作 | HTTP | 用途 |
| --- | --- | --- |
| sync_model_connection | POST /v1/model-connections/{connection_id}/sync | 来源读取 API 快照，通过 SDK 发往明确目标 |
| receive_model_sync | POST /v1/model-sync/import | 目标原子接收，仅返回脱敏目录 |

本机 socket 与 HTTP 使用相同 operationId/schema。接收端运维等价命令为 `rove model receive-sync --body -`，敏感输入只走 stdin，不应手写密钥到 shell 历史。普通用户使用 `model sync`，不需要接触快照。

schema 拒绝 session、access_token、refresh_token 等额外字段；来源/替换连接有非 API 认证标记时拒绝操作。不能从任意字符串判断用户是否误把会员令牌填入 API 密钥字段，因此不将此校验宣传为令牌类型自动识别。

## 验证与交付范围

- 后端：真实 SDK → HTTP 的两 agent 同步、凭据不回显、覆盖保护、型号 ID/默认保留、重复请求、事务失败不变、重启保留、缺路由无本机回退。
- CLI：socket 接收、重试、远端目标参数与 `--yes` 保护。
- 浏览器：320px/1440px 同步弹窗、目标发现、冲突展示、选择替换、确认控制；请求不含密钥。32 项浏览器回归通过。
- 真实 EasyTier：两个隔离容器并行双网，经 overlay 同步、重复请求、密钥变更无确认拒绝、确认覆盖，以及来源删除后目标保留；报告 `/mnt/data/rove-deliveries/2026-09-18/api-sync-overlay-v1.json`。没有调用收费供应商，不冒充 Android → Windows 实机验证。

当前远端路由仍受平台能力约束：Linux overlay 已实现，Windows 网络运行时与 Android native VPN 尚未交付。源码增量没有重新打入旧 Ubuntu/Android 安装包，macOS 环境继续暂缓。
