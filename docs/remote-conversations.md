# 本机会话与远端执行

正式 GUI 的会话由发起端 `rove-agent` 保存。选择设备 B 后新建会话，会在本机 A 保存 B 的网络/设备关联；切换设备选择、关闭窗口或重启 A，不会把这个会话改成本机执行。B 保存实际作业、模型上下文和工具结果，独立推进已接受的任务。

## 接口与存储

接口以 `api/rove-agent.openapi.json` 为准；目前 agent 44 个操作，配置服务 3 个操作。

- `create_session` 新增可选 `execution_target: {network_id, device_id}` 与幂等 `session_id`。不指定目标时保持本机行为；指定目标时先保存本机关联，首次显式提交才在远端创建执行会话。相同创建 ID 与原输入返回原会话；修改输入冲突，删除后不能复活。
- `Session.device_id` 是拥有者 A，`execution_target.device_id` 是执行者 B，两端使用同一会话 UUID。关联不可变。模型选择引用 B 的目录，凭据仍在 B，不借用 A 的模型，也不随会话复制密钥。
- `submit_run` 发送前保存原请求、实际发送的型号/网络语境及请求 ID；接受后保存远端 run_id。原请求重试仍由 B 去重；不同消息/型号复用 ID 被拒绝。
- 新增 `list_session_submissions`：`GET /v1/sessions/{session_id}/submissions`，分页读取未确认请求。只读调用或进程重启不会重新提交。GUI 提供“重试原请求”，不使用新草稿替代原输入。
- `list_runs`、`get_run`、`list_messages`、`cancel_run` 和事件订阅按关联经 SDK 路由，不依赖 GUI 当前选择。取消失败不写本机假成功。socket 与 HTTP/SSE 使用同一逻辑，断开观察不取消作业。
- 本机保存已读取的作业目录、快照和观察水位。离线时，有缓存的目录/快照附带 `sync_error`；没有快照则报错。完整工具消息仍在 B，未实现离线完整复制。无会话过滤的 run 列表包含本机作业及已观察远端缓存；实时检查应指定会话或读取 run。

SQLite v5 升级前备份，增加 `session_creations`、`remote_submissions`、`remote_runs`，保留设备身份、模型和旧历史。远端作业不进入 A 的本机执行队列，也不占 A 的 AI 槽；A 重启只中断 A 真正执行的作业。

## CLI

```sh
rove session create --title '管理远端音乐服务' \
  --execution-network <network_uuid> --execution-device <device_uuid> \
  --session-id <new_session_uuid>
rove session send <session_uuid> '先检查设备，列出安装方案' --request-id <request_uuid>
rove session pending <session_uuid>
rove run list --session-id <session_uuid>
rove run show <run_uuid>
rove run watch <run_uuid>
rove run cancel <run_uuid>
```

这些命令不需要全局 `--network/--device`：会话已有执行关联。必要时将 `session pending` 返回的 `request` 原样交给 `rove call submit_run --path session_id=<id> --body -`，保留原请求 ID、消息及型号/网络字段。

旧的 `rove --network <id> --device <id> session list` 仍直接读取旧执行端历史，不会自动搬到本机列表。GUI 新建会话使用新版归属，旧远端记录可通过兼容 CLI/API 继续访问。

## 归档、删除与验证边界

归档/恢复在 A 生效，不取消 B 已接受的任务。只有已归档、没有未确认请求且已观察作业均为终态才允许删除。删除清理 A 的关联/缓存并保留去重墓碑，不连带删除 B 的历史或第三方文件。执行端长期离线且接受结果未知时不能宣称任务已停止，应保留请求直到恢复连接、查明结果。

测试使用隔离的两个 agent 数据目录、真实 socket/HTTP/SSE 与本地可控模型接口，覆盖发起端重启、丢响应重试/只读恢复、模型归属、缓存水位和身份保护、取消、删除保护、CLI 与 GUI。测试路由仅在 Rust `cfg(test)` 下允许回环地址；正式构建继续使用 EasyTier 发现及接口绑定，不增加公网/物理 LAN 执行入口。

这不代表 Android native VPN、移动后台常驻或 Android → Windows 真实 overlay 闭环完成。新旧应用应一起升级以使用扩展契约；不模拟会员登录。不更新 4173 原型，也不把旧安装包视为包含本次实现。
