# Rove 现行接口契约

此目录是运行时、生成器、SDK 与契约测试的固定接口来源。`bootstrap-rove` 于 2026-09-21 归档，其 `api/` 保留历史快照；以后小提案直接更新此目录，并在提案中记录接口变化。

- `rove-agent.openapi.json`：49 个 agent 操作，HTTP 与本机 socket 共用 schema。
- `rove-config-server.openapi.json`：3 个密文配置服务操作。
- `local-socket.md`：本机传输帧、调用与订阅映射。
- `contract-examples.json`：正反 schema 用例。
- `sharing-test-vector.json`：固定测试密钥的加密互操作向量，仅用于测试。

从仓库根目录运行 `.venv/bin/python scripts/check-contracts.py`，并运行 `cargo test -p rove-protocol` 验证生成和嵌入契约。接口行为以 OpenAPI 和 `openspec/specs/` 为准；平台实际交付范围以 `docs/implementation-status.md` 为准。

每个涉及接口的小提案同时更新 OpenAPI、SDK/CLI/GUI 对等映射和必要契约用例。不要让构建依赖活动提案目录，也不要在归档历史中修改现行接口。
