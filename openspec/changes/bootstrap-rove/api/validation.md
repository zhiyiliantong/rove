# 契约校验记录

本记录验证规划文档与机器可读接口，不表示 agent、GUI、CLI或网络服务已经实现。

## 本轮结果

| 检查 | 结果 |
| --- | --- |
| OpenSpec 严格校验 | `bootstrap-rove` 通过 |
| OpenAPI 3.1.0 合规 | 两份文档通过 openapi-spec-validator 0.9.0 |
| 操作数与唯一性 | 33个 agent操作 + 3个配置服务操作，operationId唯一 |
| Path 参数 | 每个路径占位符均有必需 path参数定义 |
| Schema 合法性 | 61个 schema通过 JSON Schema 2020-12结构检查 |
| 引用解析 | 313处 `$ref` 均可解析 |
| 内嵌示例 | 21个 examples通过 schema与format校验 |
| 正反用例 | contract-examples.json的26个用例全部符合预期，包括必须拒绝的无效输入 |
| 本机映射示例 | SocketRequest的 submit_run示例通过 operationId、路径参数及具体 RunSubmit schema校验 |
| 分享加密向量 | 正常解密与明文一致；错误密钥、密文篡改均被拒绝 |

正反用例覆盖并发上限零值、空 PATCH、缺失 request_id、错误 UUID、加入包混入 device_id、错误导入分支、非回环代理目标、端口越界、缺少远端网络上下文、错误事件 payload、模型状态与内容矛盾、密文上传混入解密密钥，以及有效请求/响应/事件。

## 复核命令

在安装了 openapi-spec-validator 0.9.0的独立工具环境中，从仓库根目录运行：

```sh
python3 -m openapi_spec_validator openspec/changes/bootstrap-rove/api/rove-agent.openapi.json
python3 -m openapi_spec_validator openspec/changes/bootstrap-rove/api/rove-config-server.openapi.json
openspec-cn validate bootstrap-rove --strict
```

本轮校验器安装到临时目录，未修改应用依赖。额外 schema检查使用 jsonschema 4.26.0、Draft202012Validator、FormatChecker与 referencing Registry，并将 OpenAPI文档根注册为本地 URI后逐条验证 components.schemas的 examples与 contract-examples.json。

分享向量以 Node.js crypto AES-256-GCM实现复核。将 ciphertext从base64url解码、末16字节作为tag，使用给定nonce、key与AAD解密；输出 JSON必须等于plaintext。分别修改密钥首字节和密文首字节，解密必须失败。向量固定密钥仅用于互操作验证，不能作为实际分享密钥。

## 实施阶段仍需验证

运行时实际序列化与契约一致性、socket拆帧/粘帧、多请求乱序、SSE重连、并发调度、物理接口访问拒绝、EasyTier多实例集成、移动端能力、安装包及真实两设备闭环均在 tasks.md中跟踪。本轮不把静态契约通过解释为上述运行行为已经通过测试。
