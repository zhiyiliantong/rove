# 本机 socket 协议映射

## 1. 传输与生命周期

Unix 使用用户私有 runtime 目录下的 Unix domain socket，Windows 使用按当前账号区分的 named pipe。移动嵌入核心仍暴露平台支持的本机 socket适配，不引入另一套业务方法。

帧为 `4 字节无符号大端长度 + 指定长度的 UTF-8 JSON`；长度只计 JSON字节，不含4字节前缀。单帧最大8 MiB，不允许零长帧。解析必须支持多次读取凑成一帧和一次读取包含多帧。超限、无效 UTF-8或无法解析 JSON时关闭连接；完整可解析但业务无效的请求返回对应错误。

连接关闭只释放连接及其订阅，不停止 agent、网络、代理或任何已接受 run。运行时数据目录锁与 socket相互配合；不得仅凭 socket路径存在判断进程在线。

## 2. operationId 是共同业务入口

请求 schema 为 Agent OpenAPI 的 `SocketRequest`。接收后按 operation_id查找 OpenAPI 操作，依次验证 path_parameters、query_parameters 和 body；envelope 中 body为可变类型，是因为具体类型由被调用操作决定，不能跳过第二层校验。未知 operation_id返回404 `operation_not_found`。

```json
{
  "kind": "request",
  "correlation_id": "00000000-0000-4000-8000-000000000011",
  "operation_id": "submit_run",
  "target": {
    "network_id": "00000000-0000-4000-8000-000000000021",
    "device_id": "00000000-0000-4000-8000-000000000022"
  },
  "path_parameters": {
    "session_id": "00000000-0000-4000-8000-000000000031"
  },
  "body": {
    "request_id": "00000000-0000-4000-8000-000000000041",
    "message": "在这台设备上装配音乐服务器"
  }
}
```

correlation_id标识本连接中的请求/响应关联，不承担持久去重。RunSubmit.request_id标识业务提交，网络重试保留该 ID但可换新的 correlation_id。重复的活动 correlation_id在同连接内返回409 `correlation_conflict`，不能覆盖前一个响应路由。

target省略表示本机；提供时必须同时指定 network_id和 device_id。目标等于本机时直接调用本机 handler；远端通过 SDK查询当前地址、握手并发送对应 HTTP 操作。SDK自动填入 HTTP上下文 headers，接收的远端 agent核对身份并本机执行，不能再进行第二跳转发。

HTTP 的 path/query/body定义全部复用；路径值在 socket中是名称到字符串的映射，查询值按声明类型传递。路由 headers只用于 HTTP入口，本机不要求伪造这些字段。请求 body省略和 `{}` 的区别按 OpenAPI处理：例如 create_session声明必需 body，必须发 `{}`。

## 3. 响应和错误

`SocketResponse` 包含 kind、correlation_id、status_code与可选 body。status_code沿用对应 HTTP语义；body按 OpenAPI相应响应 schema验证；204必须省略 body。错误 body统一为 ErrorResponse。某个业务请求失败不会关闭一个正常连接。

agent 允许同连接多个未完成请求，响应按 correlation_id关联，不要求响应顺序等于发送顺序。多个会话由运行时并发模型决定，socket本身不增加设备全局串行锁。

## 4. 事件订阅

客户端以 `operation_id: subscribe_run_events`、path run_id、query after_seq发起订阅。历史游标错误先返回普通错误响应；成功返回200与 `SocketStreamOpened`（subscription_id、run_id）。后续帧是 SocketEvent，通过 subscription_id关联，event内容完整复用 RunEvent。

终态且没有新事件直接返回204。成功订阅后，terminal status事件后发送 SocketStreamEnd，reason为 terminal。远端 SSE故障则 reason为 transport_error并携带错误；SDK可以重新请求同一 operationId并保留最后处理的 seq。

```json
{
  "kind": "unsubscribe",
  "correlation_id": "00000000-0000-4000-8000-000000000051",
  "subscription_id": "00000000-0000-4000-8000-000000000061"
}
```

unsubscribe只取消事件订阅，回复204；重复取消同连接已结束订阅仍为204。若订阅仍活动，再发送 reason=unsubscribed的 StreamEnd。其他连接的 subscription_id不属于本连接，不释放其他连接资源。停止 AI运行必须调用 cancel_run，不能用 unsubscribe替代。

## 5. 边界与验证

- 业务 schema、operationId和错误来自 OpenAPI，不复制一套 socket 专属请求模型。
- SocketFrame oneOf校验只保证 envelope合法；还需 operation级参数、body和response校验。
- 框架应测试拆帧、粘帧、超限、无效编码、乱序响应、多订阅、取消订阅以及断线后作业继续。
- 单条事件与列表页输出保持有界；超长工具输出分块，并保留明确的截断标识，不发送超过最大帧的响应。
- 本机 service注册与进程启动发生在连接之前，通过 CLI/OS本地生命周期入口完成，不封装成“启动自己的 socket请求”。

## 6. SDK 等待期限

当前 SDK 的普通本机调用对连接与 hello 握手共用 5 秒期限，握手成功后发送请求与等待响应共用 30 秒期限。事件订阅的连接、hello 与订阅确认共用 5 秒期限；订阅成功后的事件观察不套用整个作业总超时，空闲长作业仍可继续观察。

握手超时表示业务请求尚未发送。业务响应超时只表示结果未知，不代表未接受或已回滚；SDK 关闭该连接，不自动重提请求或发送 `cancel_run`。若是作业提交，客户端保留原 `request_id` 进行显式重试或查询已有作业。订阅握手超时只关闭观察连接，不取消 run。上述期限属于 SDK 资源保护策略，不改变 OpenAPI 的业务 schema 或持久去重语义。
