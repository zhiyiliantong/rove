# 核心模型链路（2026-09-17）

范围：正式 agent / CLI / Vue GUI，不是浏览器演示数据。安装包和 4173 原型没有更新。模型与首次引导之后，13.5 的本机会话关联已继续接入，见 [远端会话](remote-conversations.md)。

## 使用

GUI：全局添加 → 模型配置 → 添加模型。提供商决定可修改的默认接口地址；可用参考型号或点击“获取可用型号”，默认全选，也能手动填写。每个型号对应独立可编辑配置名称，共享一次输入的 API 密钥。保存不等于供应商已验证推理/工具权限。

会话加号 → 选择模型。选择存到当前会话所属 agent，重开应用仍保留；跟随默认与显式选择分开。更新连接需要重新填写密钥，留空表示清除，不悄悄保留。移除连接需确认，会删除其所有型号和凭据，但保留历史；已开始的作业继续使用原快照，排队作业不回退其他型号。

新安装第一次批量添加模型时，agent 与模型目录同事务创建唯一“初始网络的会话”，引导创建或加入网络。归档/删除后不重新生成。旧版配置迁移以及兼容 `model set` 不插入额外引导。网络创建后自动显示完整入网配置；实际启动/联网状态仍以网络运行结果为准，二维码/URL 分享需要现有密文配置服务。

CLI 示例（占位值不是可直接调用的凭据）：

```sh
rove model list
# 在 stdin 输入 ModelImport JSON，避免将密钥写入进程参数/历史：
rove model import --body -
# {"provider":"openai","base_url":"https://api.openai.com/v1","api_key":"YOUR_KEY","models":[{"model":"YOUR_MODEL_ID","name":"我的助手"}],"set_default":true}
rove model discover --body -
# ModelConnectionWrite JSON：provider、base_url、api_key
rove model rename MODEL_UUID "家庭助手"
rove model default MODEL_UUID
rove session model SESSION_UUID MODEL_UUID
rove session send SESSION_UUID "先查看设备情况" --model-id MODEL_UUID
rove model update-connection CONNECTION_UUID --body -
rove model delete-connection CONNECTION_UUID --yes
```

`model default` 不带 ID 取消默认；`session model SESSION_UUID` 不带型号 ID 恢复跟随设备默认。远端配置由 `--network` / `--device` SDK 路由完成，密钥仅发送到显式选定的设备，不放进网络分享信息。追加的 `model sync` 和 GUI 同步入口可复制本机一个 API 连接及其全部型号；不是所有连接的后台批量同步。

## 接口和存储

接口源：`api/rove-agent.openapi.json`，新增 7 个操作，agent 共 43、配置服务 3。SDK/socket/HTTP 使用同一运行时契约；`update_session` 支持 model_id/null，`submit_run` 可显式覆盖。

SQLite v4：升级前保存 `rove-before-v4-*.db`，旧默认模型成为 legacy 连接及型号，不改写旧凭据/地址/型号，不更换 device_id。目录只显示 `api_key_configured`。兼容默认影子与目录在同一事务更新。数据目录/数据库/备份仍需按敏感文件保管；升级后回退旧程序须恢复相应备份。

模型型号提交时选择，连接凭据在开始执行时快照。请求去重包含显式型号选择，但保留旧无型号请求的摘要；删除配置后重试已接受的请求仍返回原作业，不重复执行。默认模型删除变成未配置，不自动另选一个。

## 适配边界

执行：OpenAI（兼容 Chat Completions）、DeepSeek、Anthropic、Gemini、Moonshot、Z.ai、MiniMax、Mistral、xAI、OpenRouter、Groq、Ollama 采用锁定 Rig 0.42.0 的适配器；Mistral/xAI/Gemini 版本路径做调用侧规范化，不改写用户保存的地址。模型必须支持对话及工具调用。

型号枚举：Anthropic/Gemini 原生认证与分页，Ollama tags，其余模型列表兼容接口。15 秒总超时、拒绝跳转、每页最多 1 MiB、10 页/2000 个型号，超出返回截断标志。一次保存最多 100 个，设备总量 1000 个。失败不写模型目录、不显示供应商错误正文；可以手动填写。预置目录沿用原型 2026-09-16 版本，不宣称所有型号可用。

2026-09-18 更新：API 连接一键同步已接入，账号每设备单独授权、不复制登录令牌，详见 [API 同步](model-sync.md)；官方 Agent 登录适配仍未实现。多网络与网络名片已在 13.6 完成，见 [多网络实现](multinetwork.md)。原生请求和模拟 SSE 测试不等于真实厂商或全部平台实机验收。

## 验证入口

```sh
CARGO_TARGET_DIR=/mnt/data/rove-build-20260916-xoZ37q/target cargo test -p rove-agent -p rove-cli -p rove-protocol -p rove-sdk --offline --locked -j 2
CARGO_TARGET_DIR=/mnt/data/rove-build-20260916-xoZ37q/target cargo clippy -p rove-agent -p rove-cli -p rove-protocol -p rove-sdk --all-targets --offline --locked -j 2 -- -D warnings
.venv/bin/python scripts/check-contracts.py
openspec-cn validate bootstrap-rove --strict
cd apps/rove-gui
npm test
npm run build
PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH=/path/to/chromium npm run test:e2e -- --workers=1
```

专门证据：`model_catalog.rs`（原子导入/脱敏/迁移/持久选择/唯一引导）、`model_providers.rs`（全部原生请求、三种原生流、枚举分页/限制/无密钥持久化）、`ai_runtime.rs`（选定型号、默认隔离、删除后不回退/快照）、CLI stdin/删除确认和 GUI 浏览器交互。
