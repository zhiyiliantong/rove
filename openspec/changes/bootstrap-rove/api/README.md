# Rove 接口契约

## 1. 文件与约束

- [rove-agent.openapi.json](rove-agent.openapi.json)：OpenAPI 3.1.0，33 个执行设备操作，包含业务 schema 与本机 socket envelope schema。
- [rove-config-server.openapi.json](rove-config-server.openapi.json)：OpenAPI 3.1.0，匿名密文上传、下载及分享地址别名。
- [local-socket.md](local-socket.md)：本机 transport 帧和 operationId 映射。
- [contract-examples.json](contract-examples.json)：请求、事件及错误的正反 schema 用例。
- [sharing-test-vector.json](sharing-test-vector.json)：固定测试密钥的分享加密互操作向量，仅供验证。

本提案选择 OpenAPI 3.1.0，schema 使用 JSON Schema 2020-12。OpenAPI描述 HTTP API，本机字节流格式由补充文档定义，但操作参数与结果仍使用同一 schema。[OpenAPI 规范](https://spec.openapis.org/oas/v3.1.0)

接口契约版本为 `0.1.0`，HTTP 路径从 `/v1` 开始；均为待实现契约。发布到公网的只有密文配置服务，agent HTTP 接口仅在已加入 overlay 中开放。本机 GUI/CLI 使用 socket，不必在物理回环 TCP 上再开放一个 HTTP 端口。

## 2. 目标与网络上下文

每个 HTTP 操作只操作接收 agent 的本机资源。来自 GUI/CLI 的远程目标先交给本机 agent，后者通过 SDK 连接目标；HTTP 服务端不继续转发。`X-Rove-Network-Id` 表示连接所在网络，`X-Rove-Target-Device-Id` 表示预期设备，目标 ID不一致返回 409 `target_mismatch`，防止地址复用导致命令送错设备。

初次 `get_hello` 不要求目标 ID，因发现时尚未知道设备身份；它仍校验网络上下文。本机 socket 省略远程 target 时上下文为本机，不必伪造上述 headers。它们是路由与一致性字段，不是认证凭据。

OpenAPI 的 `security: []` 与产品约定一致：没有二次 token 或 RBAC。运行时必须保证 agent 的远程入口仅可经 overlay 到达，不能把同一 OpenAPI 直接部署为公网无认证服务。

## 3. 操作目录

| 组 | operationId | HTTP |
| --- | --- | --- |
| 运行信息 | `get_hello`、`get_device` | GET `/v1/hello`、`/v1/device` |
| 设备设置 | `get_settings`、`update_settings` | GET/PATCH `/v1/settings` |
| 模型 | `get_model_config`、`set_model_config`、`clear_model_config` | GET/PUT/DELETE `/v1/model-config` |
| 网络集合 | `list_networks`、`create_network` | GET/POST `/v1/networks` |
| 网络记录 | `get_network`、`update_network`、`delete_network` | GET/PUT/DELETE `/v1/networks/{network_id}` |
| 网络启停 | `start_network`、`stop_network` | POST `/v1/networks/{network_id}/start`、`/stop` |
| 导入 | `import_network` | POST `/v1/networks/imports` |
| 手动导出 | `get_network_join_config` | GET `/v1/networks/{network_id}/join-config` |
| URL 分享 | `create_network_share` | POST `/v1/networks/{network_id}/shares` |
| 发现设备 | `list_network_devices` | GET `/v1/networks/{network_id}/devices` |
| 会话集合 | `list_sessions`、`create_session` | GET/POST `/v1/sessions` |
| 会话记录 | `get_session`、`update_session` | GET/PATCH `/v1/sessions/{session_id}` |
| 历史消息 | `list_messages` | GET `/v1/sessions/{session_id}/messages` |
| 提交 AI | `submit_run` | POST `/v1/sessions/{session_id}/runs` |
| 作业查询 | `list_runs`、`get_run` | GET `/v1/runs`、`/v1/runs/{run_id}` |
| 作业取消 | `cancel_run` | POST `/v1/runs/{run_id}/cancel` |
| 事件流 | `subscribe_run_events` | GET `/v1/runs/{run_id}/events` |
| 服务集合 | `list_services`、`publish_service` | GET/POST `/v1/services` |
| 服务记录 | `get_service`、`update_service`、`unpublish_service` | GET/PUT/DELETE `/v1/services/{service_id}` |
| 密文上传 | `upload_blob` | POST `/v1/blobs`（配置服务） |
| 密文下载 | `download_blob`、`download_shared_blob` | GET `/v1/blobs/{blob_id}`、`/c/{blob_id}`（配置服务） |

安装、注册 OS 服务及连接本机 agent 前的启动诊断属于 CLI/安装器本地动作，没有伪造为调用一个尚未运行的 agent 的 HTTP API。任意 shell 命令通过 AI 工具运行，本提案不另增远程 `/exec` 操作端点。

## 4. 通用字段、分页与错误

UUID 使用规范字符串，时间使用 UTC RFC 3339 字符串，业务字段和 operationId 使用 snake_case。`null`、省略和空字符串的含义由 schema 分别规定，不互换。模型 PUT 为全量替换，`api_key: null` 表示不使用密钥；普通读取只返回 `api_key_configured`。

RunSubmit 可携带 network_id作为任务语境；省略时远端使用入口网络，本机使用无网络语境。Run返回最终解析的 network_id和 input_message，排队输入因此可查看但不会提前加入模型历史。去重摘要包含解析后的 session、网络语境和输入消息，重试必须保持该语境。

列表默认 `limit=50`，最大 100，返回 `items` 和 `next_cursor`。使用 keyset 游标并固定排序：网络按 network_id、设备按 device_id、服务按 service_id；session 和 run 按 `(created_at, id)` 升序；消息按持久消息序号升序。游标编码集合、过滤条件和最后一个排序键，改变过滤条件后继续旧游标返回 400 `invalid_cursor`。列表不是跨设备一致性快照；客户端按稳定 ID合并刷新结果。单页编码后大小也受运行时输出上限约束，允许少于 limit 条且仍有 next_cursor。

错误响应统一为 `{"error":{"code":"...","message":"...","retryable":false,"details":{}}}`。`details` 可省略；只含可公开的诊断字段。HTTP 成功响应直接返回资源或分页对象，不再包裹一个重复的 `success` 字段。

| HTTP | code 示例 | 客户端行为 |
| --- | --- | --- |
| 400 | `invalid_request`、`invalid_cursor`、`invalid_event_cursor` | 修正输入 |
| 404 | `network_not_found`、`session_not_found`、`run_not_found`、`service_not_found`、`blob_not_found` | 刷新选择；不能改为在别处执行 |
| 409 | `request_conflict`、`network_config_conflict`、`network_must_be_stopped`、`target_mismatch`、`protocol_incompatible`、`event_cursor_ahead` | 查询实际状态后显式处理 |
| 410 | `share_expired`、`event_history_expired` | 重新分享或先获取作业快照 |
| 413 | `payload_too_large` | 减小输入 |
| 422 | `model_not_configured`、`invalid_network_config`、`decryption_failed`、`unsupported_config_version` | 设置模型或修正配置 |
| 429 | `queue_full`、`rate_limited` | 等待后按原请求标识重试 |
| 501 | `unsupported` | 展示平台能力限制 |
| 502/503 | `peer_unreachable`、`overlay_unavailable`、`config_server_unavailable`、`storage_unavailable` | 展示依赖故障，不假定远端未执行 |
| 500 | `internal_error` | 保留请求 ID并查询状态，避免盲目重复有副作用的创建 |

工具运行失败通常保存在 run 与 ToolResult 中，不把已经返回 202 的提交请求倒改成 HTTP 500。协议错误和已接受作业的执行错误分开处理。

## 5. 创建、状态与重试

- 网络创建/导入只保存为 stopped；GUI/CLI 的“加入”流程随后调用启动。启动/停止返回 202 表示操作已接受，最终状态通过 GET network 查询。
- 创建 session、network、share 和 service 不自动幂等。响应丢失先查询；SDK 不擅自重发全新创建。
- `submit_run` 由执行设备按 request_id 去重，持久查询先于容量和模型校验。接受后即使配置后来消失，重试仍得到原 run。
- 调用 cancel/stop/start 及删除接口具有文档规定的重复调用结果；已不存在的删除返回 204。
- 模型设置 PUT 全量替换；网络 PUT 要求已停止；服务 PUT保留服务身份与网络归属，失败不能悄悄丢失旧发布。
- 删除本机网络记录同时移除该网络发布定义与本机路由缓存，不删除会话、模型、部署软件或其他成员配置；停止网络仅关闭入口并保留定义。

## 6. 流式事件与重连

HTTP 订阅使用 `text/event-stream`。每条业务事件固定 `event: run_event`，`id` 为该 run 的十进制 seq，`data` 是 RunEvent JSON；注释心跳不写入会话历史。流式 framing 遵循 SSE 标准。[WHATWG SSE](https://html.spec.whatwg.org/multipage/server-sent-events.html)

```text
id: 7
event: run_event
data: {"run_id":"00000000-0000-4000-8000-000000000001","seq":7,"created_at":"2026-09-07T00:00:00Z","kind":"assistant_delta","data":{"text":"安装已经完成"}}

```

SDK 记录最后完整处理的 seq，重连携带 after_seq 或 Last-Event-ID。两者并存须一致；after_seq 超过当前 last_seq 返回 409。请求比保留历史更旧时返回 410 并查询快照，以 snapshot_seq 为新水位继续订阅。快照与水位必须原子读取，避免漏掉快照和新流之间的输出。

服务端先写事件再推送，终态 status 事件写入后关闭流。终态且游标后没有事件返回 204，SDK停止重连；流传输中断则以原游标重连。终态快照、会话历史和输出尾部仍可查询。慢客户端可以丢失连接，但不能阻塞执行或造成无限内存积累。

## 7. 分享加密互操作约定

明文是 JoinConfig 的 UTF-8 JSON。每次分享用新生成的 32字节密钥和12字节 nonce，AES-256-GCM固定 AAD 为 UTF-8 字符串 `rove.network_join_config.v1`。密文后拼接16字节 tag，使用不带 padding 的 base64url编码。envelope 的版本和算法只接受契约规定值，未知值在解密前拒绝。

服务器保存 CipherEnvelope，返回 share_base_url，例如 `https://config.example.com/c/<blob_id>`；agent 在本机追加 `#key=<32字节密钥的base64url>`。密钥编码固定为43字符，nonce 为16字符。URL、二维码、手动入网明文及模型凭据不进入普通日志。blob ID 是16随机字节的32字符小写十六进制。

下载不使用或消耗 token；同一密文可重复下载。到期尚未清理可返回410，清理后返回404，均代表该分享无法继续使用。分享端点不提供删除成员、轮换网络密钥或撤销访问的语义。

## 8. 契约维护与验证

OpenAPI是 wire schema 的唯一来源；Rust DTO 和前端类型按它生成或手写并以契约测试校验，不从手写 Rust 模型反向覆盖规范。暂不要求绑定一个代码生成器。

当前规划校验包括 OpenAPI 3.1 合规、所有引用解析、operationId 唯一、path 参数完整、schema 合法、示例符合 schema，以及本机 operationId映射。实施时追加服务端/SDK真实序列化与 schema 一致性测试。校验命令及本轮实际结果见 [validation.md](validation.md)。
