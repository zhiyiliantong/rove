# 密文配置服务器

`rove-config-server` 是可选公网密文存储服务，不是控制中心、设备注册中心或网络成员服务。常规组网不依赖它持续在线。

## 本机开发

先创建要使用的数据库父目录，或先运行 agent 创建其私有 `.rove` 目录，再按根 README 启动服务。默认监听 `127.0.0.1:43191`，不隐式绑定公网。

1. `create_network` 创建停止状态的网络，记下 `network_id`。
2. `update_settings` 设置 `config_server_url` 为 `http://127.0.0.1:43191`。
3. `create_network_share`，path 参数 `network_id`，body `{}`。
4. 另一个 agent 使用 `import_network`，body 为 `{"source":"url","url":"完整分享链接"}`。
5. 验证接收者拥有相同 `network_id`、不同 `instance_id` 和自己的 `device_id`。当前阶段网络保持 stopped。

可用 `rove call` 完成全部操作。分享 URL 含密钥，应通过 stdin 或 GUI 粘贴处理，不写普通日志。

也可使用 `network create`、`config set --config-server`、`network share <network_id> --qr`、`network join -` 和 `network import --body -`。终端二维码输出到 stderr，正常分享结果仍为 stdout JSON；二维码同样包含密钥，请勿公开。GUI 扫码在本机解析，不把相机图像上传到配置服务；扫码结果需用户确认导入。

## 协议与安全边界

- 上传 `POST /v1/blobs` 只接受 `{"envelope":...}`，额外 `key`、明文字段会被拒绝。
- 下载 `GET /v1/blobs/{blob_id}` 和 `GET /c/{blob_id}` 返回相同密文。
- agent 在本机生成 32 字节密钥、12 字节 nonce，AES-256-GCM 的 AAD 固定为 `rove.network_join_config.v1`。
- URL fragment `#key=...` 不发送给服务。agent 禁止 HTTP 重定向，并从用户配置的 origin 构造分享地址，不把密钥附加到服务返回的任意主机。
- 默认七天过期。有效期内重复下载不消耗链接；到期返回 410，清理后 404。到期不是踢出已经加入的设备。
- HTTP 请求体最大 256 KiB，解码后密文最大 128 KiB（含 tag）。默认存储上限 1 GiB、10,000 条；达到上限返回 429。可用 `--max-storage-bytes`、`--max-blobs` 修改。该上限按已保存 envelope 字节计，不包含 SQLite 元数据，磁盘还需额外配额。
- 每小时清理到期行，SQLite 复用空闲页；文件不会立即缩小。部署者仍应设置磁盘配额、连接数和请求速率限制。
- 上传/下载正常和业务错误响应均为 `Cache-Control: no-store`；不要在反向代理中覆盖为公开缓存。

## 公网部署

使用专用低权限系统账号与持久数据目录；用 TLS 反向代理终止 HTTPS，后端继续监听 loopback，`--public-url` 设置为对外 HTTPS 根地址。HTTP 仅用于显式 loopback 开发。

代理应设置至少与后端一致的 body 限制、每源上传速率、并发连接和超时限制。不要记录请求体、分享完整 URL、网络配置或模型凭据；不要注入分析脚本。服务没有账号和 Rove token，这是产品约定，不表示可以没有资源限额。

备份 SQLite 时使用 SQLite backup API 或在服务停止后复制数据库及必要 WAL 文件；不要在写入时只复制主 db。过期密文无需长期留存。服务端永远不拥有解密密钥。

这些步骤是部署说明，尚未执行任何公网发布或注册宿主机系统服务。
