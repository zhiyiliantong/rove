# Rove 原型 0.6 交接与真实接入边界

## 状态

2026-09-16 用户确认归档 `conversation-archive-management`，10/10 项任务完成，主规格已同步，归档位于 `openspec/changes/archive/2026-09-16-conversation-archive-management/`；同时授权返回 `bootstrap-rove` 按原型实现正式应用。会话列表“更多”及详情可归档；会话区和设置进入归档管理，查看、恢复、确认永久删除。归档不会停止任务，详情只读；queued/running/cancelling 阻止删除，成功删除仅移除对应会话和任务历史，不删除服务。原型仍是浏览器 mock，不是正式 agent 能力；0.5 的历史验收和归档记录保持不变。以下早期“尚未授权”均是当时的历史边界，本次正式迁移授权以本段为准。

2026-09-16 验收定稿：用户明确回复“验收通过”，确认当前浏览器原型 0.5，包含第 17 组“管理 Rove”建议及设置页入口；`prototype-first-ui` 46/46 项完成。此确认不表示真实模型、跨设备部署、存储维护能力已实现或实机验收通过，也不授权正式 GUI/SDK 迁移。归档记录见 `openspec/changes/archive/2026-09-16-prototype-first-ui/`，两份原型规格同步至 `openspec/specs/`。4173 原型预览保留。

对话管理发现入口：欢迎页/未发送消息的普通会话新增“管理 Rove”建议，设置页顶部增加说明和“通过对话管理”按钮；统一另开本机会话并填入只检查空间/运行情况的草稿，旧草稿、远端目标和任务不变。无模型时打开添加模型，不创建管理会话，原有手动设置与首次初始化网络流程保留。仅新增原型入口和能力说明，未实现磁盘扫描、清理、迁移、诊断或破坏性操作确认协议；真实接入需另行定义，不能将文案计为完成。5 项针对性浏览器测试及类型/生产构建通过。现有 OpenAPI 0.5 / Snapshot v3 不变。

产品边界澄清：音乐、私人影院、多台代码代理采用同一部署模式。Rove 负责多设备对等联通、设备管理、部署和服务访问；每台设备的 rove-agent 基于 Rig 安装、配置与维护第三方开源软件。播放、媒体管理、代码代理的具体业务和协作逻辑由第三方软件提供，不增加 Rove 专用多代码代理编排引擎或场景管理页面。通用跨设备执行及恢复语义保留，现有原型仍只是模拟。欢迎页标语改为“数据自己掌握，跳出平台控制”，表达数据自主的产品方向，不代表对任何第三方平台的事实指控或所有数据永不外传的保证；真实接入外部模型/软件时仍需明确数据发送范围。

2026-09-16 应答视觉追加：参考用户截图，助手采用无整体外框的正文，代码/独立公式为浅色圆角块，右上角图标复制；回复底部只有已有复制、朗读/停止操作，说明复制桌面悬停/聚焦显示、触屏常显。保留模拟流式、源文复制、本地语音和降级；没有增加点赞、分享、真实 AI 或后端调用，因此 OpenAPI 仍为 0.5、22 个操作。38 项数据测试、34 项浏览器测试和构建通过；LAN 390/1440px HTTP 200，无页面错误/外部请求/横向溢出，手动复制可用。浅深色截图见 `apps/rove-prototype/test-results/lan-reply-refined-*.png`。朗读测试使用模拟系统语音，不代表实机音频验收。

2026-09-14：用户确认 Excalidraw 信息架构、批量模型关系、本机会话/目标执行分离、四字段网络名片，以及服务只通过会话管理；授权独立原型实施。没有授权修改现有 GUI 或真实后台。本次源码位于 `apps/rove-prototype`。

验收基线为 PrimeVue 4.5.5、Aura 1.2.5、绿色/中性色主题，响应式断点 640 / 850 / 1100px。已按本次明确授权开始真实应用迁移，不能把原型验收当作原生平台验收。依赖精确版本和间接依赖锁在各工程的 package-lock.json。

采用按需引入组件及主题 token 的方式，参考 [PrimeVue 安装说明](https://primevue.dev/vite/) 与 [主题说明](https://primevue.dev/theming/styled/)。当前网站已显示新版示例；本工程固定 4.x，不随网站最新大版本自动升级。依赖版本通过官方 npm 源核实，未修改用户全局 npm 镜像。

## 页面 / 协议对应

真实来源：`api/rove-agent.openapi.json`。
新增语义：`apps/rove-prototype/api/prototype.openapi.json`（OpenAPI 3.1，版本 0.6，25 个操作；无对应 HTTP 服务器）。新增 `POST /sessions/{id}/archive`（归档）、`DELETE /sessions/{id}/archive`（恢复）、`DELETE /sessions/{id}`（永久删除已归档且无活跃任务的会话）。成功无正文 204；缺失记录 404；归档后写操作、未归档删除及活跃任务删除为 409，SessionError 含 code/status/message。快照 v4 增加 archived_at，迁移 v1/v2/v3，不重置 initialized。正式 agent 的归档持久化和跨设备任务约束仍须后续单独接入。

| 页面或行为 | 既有 operationId | 尚缺的真实语义 |
| --- | --- | --- |
| 模型连接与型号列表 | get_model_config / set_model_config / clear_model_config | 当前只支持设备单默认配置；需连接/凭据引用、型号多记录、模型列表获取、单选默认、会话选择、迁移及重认证 |
| 初始化网络 | create_session / create_network / start_network / import_network | 首模型引导一次性标记、本机会话在未入网时的生命周期 |
| 网络列表与名片 | list_networks / update_network / delete_network / start_network / stop_network / create_network_share | 虚拟网段与 EasyTier 地址配置映射、四字段共享与设备字段分离；原型名片不替代既有加密分享协议 |
| 节点详情与模型同步 | list_network_devices / get_device | UI 显示模型与真实 Device/Peer DTO 分开；需显式一次性复制选定配置，不能通过读取脱敏接口偷偷获取密钥 |
| 本机会话 | list_sessions / create_session / list_messages | 本机拥有会话并关联远端作业的新关系；不能仅把远程 session 当成本机会话 |
| 执行与进度恢复 | submit_run / get_run / list_runs / subscribe_run_events / cancel_run | 接收确认、幂等提交、目标持久化事件、本地关联及断线补取；不同于浏览器模拟时钟 |
| 服务列表 / 会话操作 | list_services / get_service / publish_service / update_service / unpublish_service | 列表只读，UI 无管理入口；agent 工具执行变更并自动登记；访问认证和平台打开方式需单独接入 |
| 本机并发 | get_settings / update_settings | 原型本机设置仅用于本机任务；远端示例保持其独立并发配置 |

PrototypeApi 的 `snapshot()` 聚合是原型便利接口，不建议直接把它变成要求所有设备在线的生产中心接口。正式适配器应保持设备对等、各设备保存自身执行事实。

## 必须保留的 UX

### 0.6 会话归档管理（2026-09-16，已确认归档）

验证：45 项单元测试（含 7 项归档/迁移/契约测试）、44 项浏览器测试（含 5 项归档流程）、Vue 类型/生产构建、该变更及两份现有主规格 strict 校验全部通过。默认 Playwright 1243 浏览器未安装，指定本机已有 `chromium_headless_shell-1223/chrome-headless-shell-linux64/chrome-headless-shell` 后完成全量测试，未升级依赖或停止 LAN 服务。`http://10.1.2.237:4173/` 在 320/390/1440px 自动化浏览器中 HTTP 200、无页面错误、无外部请求、无整页横向溢出，归档、取消删除、恢复可用；截图为 `apps/rove-prototype/test-results/lan-archives-*.png`。这些是开发机浏览器验证，不冒充手机或 Windows 实机验收；原型 0.6 现已获归档确认，正式平台仍须实测。

保留归档会话的原 ID、消息、草稿和执行引用，恢复不重放任务；重复归档保留初次时间。归档管理位于 `/archives`，仍属于会话主导航，不新增移动底部栏标签；包括空状态、长标题折行、已删除旧链接提示。删除确认列明不可恢复、消息/草稿/任务历史范围，取消和 Escape 均不变更数据；数据层独立验证可删除条件，不能绕过 UI 删除活跃任务上下文。

存储键仍为 `rove-prototype-v1`。v4 快照不能直接交给旧 UI，旧版不认识该版本会退回演示种子；需要回退时先保留快照并显式转换，不能通过重置清除用户记录。归档状态仍依赖浏览器本地存储许可，任务“继续”仍是模拟时间线重算，不代表浏览器关闭后有后台执行进程。

### 0.5 Rig 供应商筛选（2026-09-16，已验收基线）

本轮追加：模型显示名称自动生成后可逐型号编辑/恢复自动名，ModelImport 增加可选 model_names（exact model ID → display name），省略时兼容自动生成；trim 后拒绝空名、批内及既有重名，校验在任何状态写入之前完成。不改型号 ID、凭据或旧记录，创建后刷新可恢复。

型号目录默认全选，打开/切换/失效重置后选择当前目录全部预置，用户可取消，立即生成可编辑名称。旧 0.4 “清空型号选择”的重置规则由此替换，但仍不携带旧供应商 ID 或认证。

富文本与流式：新增可选 Run.output_text，按固定文案与时间累积（35ms/字符，UI 150ms 刷新），断连暂停、取消保留片段、重开按时间恢复，不是 API/SSE 实现。固定演示文案不提前宣称安装完成。输入“演示富文本和公式”查看 Markdown/LaTeX/代码示例；完成时同一气泡接入最终关联回复。

采用 [markdown-it](https://markdown-it.github.io/markdown-it/interfaces/MarkdownItOptions.html) 15.0.2（关闭 HTML）与 [KaTeX 安全选项](https://katex.org/docs/options) 0.18.7（trust=false、maxExpand=100、maxSize=20）。代码/公式/说明分段按钮只复制源码，链接不自动打开，外部图片仅显示占位，不读取或自动请求用户链接。资源随构建打包，无 CDN；单独懒加载富文本组件。

朗读使用 [SpeechSynthesis](https://developer.mozilla.org/en-US/docs/Web/API/SpeechSynthesis) 且仅选择 localService 语音，必须点击，支持停止，组件卸载取消，不发送云 TTS 请求。流式时不可朗读；无本地语音、错误或文本超过 4000 字符有明确提示。自动化测试使用模拟语音引擎验证流程，不算真实声音/系统语音验收。复制使用 [Clipboard.writeText](https://developer.mozilla.org/en-US/docs/Web/API/Clipboard/writeText)，不安全上下文/权限失败显示只读手动复制框。以上属于浏览器操作，不新增 HTTP 后端接口。

聊天以左右气泡代替进度卡：Message 增加可选 run_id，用户请求和最终回复关联同一任务，`chat-timeline.ts` 将其展示为请求后的单一助手任务气泡。状态文字更新为结果，保留取消、折叠详情；不显示进度条。旧无关联消息不删除，旧无关联任务末尾保留；不通过文本猜测关联。OpenAPI 0.5 仍为 22 个操作、Snapshot v3，不改变任务状态机或真实后台边界。

用户要求以 Rig 已支持的提供商为依据筛选保留。核对 `crates/rove-agent/Cargo.toml`、Cargo.lock 及本机 rig-core 0.42.0 源码，参见 [固定版本官方目录](https://docs.rs/rig-core/0.42.0/rig_core/providers/index.html)。本轮只筛选已有项，不自动扩充全部 Rig 模块。

| 新建供应商 ID | Rig 模块 | 处理 |
| --- | --- | --- |
| openai / deepseek / anthropic | 同名模块 | 保留 |
| google | gemini | 保留，新建根地址为 `https://generativelanguage.googleapis.com` |
| moonshot / minimax / mistral / xai | 同名模块 | 保留 |
| openrouter / groq | 同名模块 | 保留聚合/推理平台 |
| ollama | ollama | 保留，新建原生根地址为 `http://localhost:11434` |
| zai | zai | GLM 新建改为 Z.ai 国际站，默认 `https://api.z.ai/api/paas/v4`，预置 `glm-4.6` 为该模块示例 |

共 12 家。Qwen/百炼、火山方舟、硅基流动没有对应专用模块，自定义也不是供应商，均退出新增目录。旧 zhipu 国内站不能等同于默认国际站，所以不改写为 zai；旧配置、凭据引用、型号名称、默认选择、会话和地址保留，并可在“编辑连接”查看与修改。编辑弹窗临时显示旧供应商，不回流新增列表。

`Provider.rig_provider` 指明筛选依据，`GET /providers` 返回非空模块映射；旧条目内部元数据的 null 不对外列出。Snapshot 仍 v3，历史内部输入兼容，不清空本地存储。保留手动型号和自定义地址，不将预置当作型号白名单，也不因上游模型来自未列厂商而过滤 Ollama/OpenRouter 等平台的型号。

Gemini/Ollama 原生地址和 Z.ai 国际站配置来自本机固定版本的 `providers/gemini/client.rs`、`providers/ollama.rs`、`providers/zai.rs`。实际接入还须验证协议、地区、工具调用、流式、模型权限与账号适配。当前 `rove-agent/src/ai.rs` 仍仅允许 openai/openai_compatible，此次没有迁移 GUI、调用 Rig 或连接真实模型；页面明确区分这两层支持。

### 0.4 供应商扩展（2026-09-16）

本节是历史记录；当前新增目录以 0.5 筛选结果为准，表中被移除的供应商不再显示于新增入口。

参考 [OpenCode 供应商配置](https://opencode.ai/docs/providers) 的目录预设、自定义地址和适配器分离方式。TRAE IDE 官方模型页为动态内容，本次未完整读取其厂商列表，不把本表称为 TRAE 的完整支持清单。没有引入这些工具的 SDK 或代码依赖。

共 16 项：原 OpenAI、DeepSeek、Anthropic、自定义保持不变，新增以下 12 项。下列 URL 与型号来自官方文档/示例，核对日 2026-09-16；仅为表单参考，不承诺最新全量目录、账户权限、部署存在或接口连通。

| 新增供应商 | 默认接口地址 | 参考型号 / 官方来源 |
| --- | --- | --- |
| Google Gemini | `https://generativelanguage.googleapis.com/v1beta/openai/` | `gemini-3.8-flash`，[兼容接口](https://ai.google.dev/gemini-api/docs/openai) |
| 阿里云百炼 / Qwen | `https://dashscope.aliyuncs.com/compatible-mode/v1` | `qwen-plus`，[北京地域示例](https://help.aliyun.com/zh/model-studio/model-calling-in-sub-workspace) |
| Moonshot / Kimi | `https://api.moonshot.cn/v1` | `kimi-k2.6`，[快速开始](https://platform.kimi.com/docs/api/quickstart) |
| 智谱 / GLM | `https://open.bigmodel.cn/api/paas/v4/` | `glm-5.1`，[思考模式](https://docs.bigmodel.cn/cn/guide/capabilities/thinking-mode) |
| MiniMax | `https://api.minimax.io/v1` | `MiniMax-M2.7`，[官方示例](https://platform.minimax.io/docs/api-reference/text-prompt-caching) |
| 火山方舟 / 豆包 | `https://ark.cn-beijing.volces.com/api/v3` | `doubao-seed-2-0-lite-260215`，[快速开始](https://www.volcengine.com/docs/82379/1795150) |
| Mistral | `https://api.mistral.ai/v1` | `mistral-small-latest`，[API 参考](https://docs.mistral.ai/api) |
| xAI / Grok | `https://api.x.ai/v1` | `grok-4.6`，Early Access，[快速开始](https://docs.x.ai/developers/quickstart) |
| 硅基流动 | `https://api.siliconflow.cn/v1` | `Pro/deepseek-ai/DeepSeek-R1`，[快速上手](https://docs.siliconflow.cn/docs/userguide/quickstart) |
| OpenRouter | `https://openrouter.ai/api/v1` | `~openai/gpt-latest` 动态别名，[快速开始](https://openrouter.ai/docs/quickstart) |
| Groq | `https://api.groq.com/openai/v1` | `llama-3.3-70b-versatile`、`llama-3.1-8b-instant`，[型号目录](https://console.groq.com/docs/models) |
| Ollama 本地 | `http://localhost:11434/v1` | `qwen3:8b`，[本地兼容接口示例](https://docs.ollama.com/api/openai-compatibility) |

- `providers.ts` 为统一元数据注册表，`model-catalog.ts` 为参考型号；新增 `PrototypeApi.listProviders()` 对应 `GET /providers` / `demo_list_providers`，返回元数据副本。Provider schema 定义 ID、标签、类别、地址、账号适配器名、来源、搜索别名、提示和空演示密钥许可。它不是远端服务发现或已安装适配器清单。
- 搜索覆盖中文/英文标签、厂商 ID、关键词与类别。型号完整 ID（含大小写、`~`、`/`、`:`）不变，生成显示名称时才规范化。Ollama 示例不表示已安装；自定义仍手填，获取失败仍可回退。
- 百炼的不同工作空间可能采用专用域名；[首次调用文档](https://help.aliyun.com/zh/model-studio/first-api-call-to-qwen) 的工作空间地址与表中北京兼容地址应按控制台配置选择。MiniMax 默认国际站，其他站点必须匹配密钥。GLM / 方舟等 Coding Plan 与通用 API 不应混用；[智谱套餐示例](https://docs.bigmodel.cn/cn/coding-plan/using5-1) 单独使用 coding 入口。界面已显示这些注意事项。
- 新增项不启用账号登录。原有账号演示只适用于已有 Codex / Claude Code 路径；会员不是通用 API 授权。空密钥仅为本地/自定义演示便利，不验证用户编辑后的远端 URL 是否免密。不发送或保存输入的真实凭据。
- 正式接入仍由 SDK/agent 按适配器枚举及缓存，不从浏览器直接请求厂商；不是每个服务都能统一拼接 `/models`。例如 [OpenRouter 型号目录接口](https://openrouter.ai/docs/api/api-reference/models/get-models) 可列平台目录，但不等于当前账号可调用清单。Azure、Bedrock、Vertex 等项目/区域/云身份参数本轮不做虚假的单地址预设。
- 快照维持 v3；未知旧厂商按 `custom` ID 回退，不依赖原数组下标。旧模型名称、凭据引用和自定义地址不被扩展目录覆盖。

### 0.3 型号清单与命名（2026-09-16）

全局不再有独立扫一扫，移动端在加入网络内扫码。Model 新增稳定 name，按厂商、原始型号 ID、序号生成，重复导入递增、每型号独立命名；Connection 仍共享凭据，名称默认该批首条模型名。会话、设置与同步选择使用 Model.name，UUID 与 API 型号 ID 不变。旧 Snapshot v1/v2 升为 v3，不清理草稿、连接或默认模型。

原型 `POST /model-catalog` 对应进程内 `listModelCatalog`，返回厂商、认证方式、source、checked_on、型号 id/label/note。预置为 preset，模拟获取为 mock_api/mock_account，绝不声明 live。checked_on 是参考目录核对日，不是接口调用成功时间。预置对 API / 账号都可见，但不代表会员权益。失败回到预置并保留手动输入；账号保存仍要求完成模拟登录。

官方依据及真实接入方式：

| 来源 | 可获取方式 | 限制 |
| --- | --- | --- |
| [OpenAI Models API](https://developers.openai.com/api/reference/resources/models/methods/list) | API 密钥下 GET /v1/models | 完整目录不等于所有型号都适用于 Rig 或当前任务，仍需按能力和权限处理 |
| [DeepSeek Models API](https://api-docs.deepseek.com/api/list-models/) | API 密钥下 GET /models | 使用该厂商实际 base URL；不视作网页会员接口 |
| [Anthropic Models API](https://platform.claude.com/docs/en/api/models/list) | API 密钥下 GET /v1/models，包含分页与能力信息 | 需 Anthropic 协议/请求头，不等价于 Claude Code 会员模型列表 |
| [Codex App Server](https://learn.chatgpt.com/docs/app-server#list-models-modellist) | 官方账号适配器内 model/list，分页、hidden 与能力字段 | 对应工具上下文，不向 API 地址发送账号 session；不声称目录本身保证额度或请求成功 |

建议真实架构为：本地预置先展示 → 通过 SDK/本机 agent 在所选认证上下文获取完整分页列表 → 标明实时/缓存/预置来源与时间 → 失败显示原因并回退。缓存需以厂商、接口地址、账号/凭据引用和认证方式分开，切换不能沿用旧权限；实际 401/403 不允许以预置清单冒充已认证。任意兼容接口可能不提供列表，可手动填写。正式 Claude Code 等会员适配器能否枚举及兼容哪个版本仍需验证，不虚构统一 OAuth 型号接口。

预置参考：[OpenAI 常用型号](https://developers.openai.com/api/docs/models/all) 使用 gpt-6-astra、gpt-5.6-sol/terra/luna；[Claude 官方目录](https://platform.claude.com/docs/en/models/overview) 使用 claude-fable-5-1、claude-opus-5、claude-sonnet-5、claude-haiku-4-5-20251001；[DeepSeek 更新日志](https://api-docs.deepseek.com/updates/) 使用 deepseek-flash，并将 deepseek-v4-pro 明确标为已转路由的兼容 ID，不当作另一个仍独立运行的 Pro 型号。列表为 2026-09-16 核对的参考，不是从用户账号实际获取的结果。

### 0.2 反馈更新与源依据

- 用户将单网络限制改为多网络同时连接、虚拟地址范围不能重叠。原型已改为每网络独立连接状态，服务汇总已连接网络，单独断开不会改变其他网络；任务保留提交时所属网络。`/active-network` 被 `/networks/{id}/connection` 的连接/断开操作替代，新增 `/peers/probe`。浏览器快照 version 2，旧 localStorage 键不变并自动迁移 version 1。
- EasyTier 固定 revision `8428a89d2dabc94c97d370ec607c6ca142473626` 的 [NetworkIdentity 源码](https://github.com/EasyTier/EasyTier/blob/8428a89d2dabc94c97d370ec607c6ca142473626/easytier/src/common/config.rs) 将 network_secret 作为字符串生成摘要，没有要求输入必须是十六进制或 64 字符。Rove 的默认是 Web Crypto 生成 32 随机字节再编码为 64 字符十六进制；保留非十六进制手动密钥。它是共享口令，不是非对称私钥。
- [EasyTier DHCP 实现](https://github.com/EasyTier/EasyTier/blob/8428a89d2dabc94c97d370ec607c6ca142473626/easytier/src/instance/instance.rs) 在没有路由时等待；有地址的 peer 影响所选网段，否则使用内置范围。结合 [launcher 配置转换](https://github.com/EasyTier/EasyTier/blob/8428a89d2dabc94c97d370ec607c6ca142473626/easytier/src/launcher.rs)，不能把输入的 CIDR 当成 DHCP 自定义池。本原型返回 `10.126.126.0/24` 只是固定演示，不保证实际分配一致；没有真实发现节点。名片 subnet 为 `dhcp` 时说明由接收端获取实际范围，不能视作保证或预留网段。
- **真实后台仍为单网络模式**，`docs/easytier-addressing.md` 的现状未被本次改动替换。未来多网络接入需在 agent/SDK/API 中修改串行守卫、各实例的 TUN/监听/路由资源、设备多成员关系、任务传输选路、服务地址与认证映射，并对 DHCP 地址变化、OS 实际网段、IPv6 和平台 VPN 能力实测。本轮只实现 IPv4 虚拟网段之间的原型冲突检查。手动 CIDR 的真实设备地址分配仍需正式设计，不能仅把网段字段传入 DHCP。
- 原型中本机保持单一 `local` device_id，每网络分别保存 local_ip。远端 fixture 暂为按网络的设备视图，不能把其 `network_id` 单字段直接当作未来多网络设备模型；正式接入须分离 Device 与 NetworkMembership，使用稳定 device_id 加 network_id 选路。
- 初始节点独立行支持模拟测试、失败与修改后失效；不发出真实 TCP/UDP/DNS 请求。未来真正测试必须经 SDK 调用本机 agent，执行对应协议连接/握手、超时/取消；单纯 TCP 可达不代表 EasyTier 认证或入网成功。
- 厂商默认接口依据 [OpenAI API](https://developers.openai.com/api/reference/overview)、[DeepSeek API](https://api-docs.deepseek.com/)、[Anthropic API](https://platform.claude.com/docs/en/api/overview)。分别为 `https://api.openai.com/v1`、`https://api.deepseek.com`、`https://api.anthropic.com`；各厂商协议可能不同，真实适配器不能全部视为 OpenAI 格式。
- 认证方式是互斥 API 密钥 / 账号登录，不是厂商。[Codex 登录文档](https://learn.chatgpt.com/docs/auth) 区分 ChatGPT 订阅登录和 API 密钥；[Claude Code 登录文档](https://code.claude.com/docs/en/authentication) 描述其官方工具认证。原型只演示对应官方工具适配器，不导出或复用会员 session 为通用 API 凭据；DeepSeek/自定义没有已接入账号适配器，入口禁用。表单接口地址可改，但账号登录凭据不得发往该地址。
- 会话加号模型入口打开实际模型选择，取消执行设备下拉；对话内写出设备名，仍显示任务执行端。桌面附件只有文件，移动端提供拍照/照片与网络扫一扫；移动底栏不随横屏变成侧栏。普通浏览器支持外观切换，原生接入后平台能力由宿主注入，不能依赖浏览器 UA 作为正式能力证明。

- 主导航会话 / 服务 / 网络；全局加号提供创建入口和设置。
- 移动端会话列表/详情切换；桌面宽屏分栏。窄桌面仍是桌面平台，窗口行为由平台适配器提供。
- 草稿、模型选择、执行目标随会话保存；浏览网络不改变连接，改变草稿目标不改变已接收任务。
- 加入网络即信任，分享必须明确包含网络密钥；不新增 Rove RBAC。
- 服务只能经会话修改；列表只供查找/使用，不能退回长表单管理后台。
- 输入、错误、不可达、失败、unsupported 和取消必须能区分；不能只依赖红绿颜色。

## 模拟与真实的差异

原型没有 Rig、EasyTier、Tauri、后台守护进程、OAuth、真实模型列表、真实部署或凭据存储。固定规则识别少量示例指令，任务按持久化时间戳推进。浏览器重新打开恢复只验证 UI，不能证明 Android 退出后 Agent 存活，也不能证明 Android → Windows 网络控制成功。

网络初始节点不从公网目录获取，扫码不启用摄像头；附件仅有文件名，语音只填入可审阅文字；服务不打开真实地址。新设备加入和跨设备配置复制仍需真实环境验证。

浏览器演示快照不是现有 agent DTO。原型契约清楚标为未接入；单元测试校验快照，以及与旧 API 共用的状态词汇，不声称所有页面已经与真实 OpenAPI 兼容。

## 正式迁移计划（已授权，bootstrap-rove 第 13 组）

1. 原型已确认并归档，保持独立预览；正式 GUI 首批迁移主题、导航和真实作业输出展示，不引入原型 mock。
2. 为上述新增语义建立正式 OpenSpec / OpenAPI 变更，明确凭据权限边界、任务所有权和幂等恢复。
3. 将确认后的页面、组件和主题分阶段迁入正式 GUI，沿用 Tauri → SDK 桥接；测试专用 IPC fixture 只存在于浏览器测试，不接入生产代码。后续再评估共享 UI 包。
4. GUI 与 CLI 同时映射到 rove-sdk，不让 GUI 绕过 SDK 直连 agent 或处理专有远程传输。
5. 分别测试 Linux、Windows、Android 的安装升级、socket、overlay、任务后台存活及重连。新增验证不自动勾选 bootstrap-rove 未完成的任务。

## 预览部署

0.5 验证记录（2026-09-16）：38 项数据测试、31 项浏览器测试、Vue 类型/生产构建及 OpenSpec strict validate 通过。覆盖 Rig 12 项目录与旧配置、默认全选/取消、独立改名与原子重名校验、气泡排序/单一结果/恢复/取消、流式前缀/断连、Markdown/公式/未闭合片段/XSS 与外部图片防护、全文/代码/说明/LaTeX 复制、模拟本地语音启停及能力不足反馈。安装新增依赖时 npm 审计 0 vulnerabilities。主入口约 349 kB、按需富文本组件约 362 kB、共享组件约 173 kB，当前构建无超过 500 kB chunk 的提示，字体本地打包。

局域网构建 URL `http://10.1.2.237:4173/` HTTP 200；390px 与 1440px 浏览器检查默认全选、中文手改名称保存/刷新、富文本流式完成、同气泡恢复、LaTeX 手动复制降级，无页面错误、外部请求或横向溢出。查看截图 `test-results/lan-rich-chat-390.png`、`lan-rich-chat-1440.png`；这是开发机浏览器检查，不是用户视觉验收、Android 真机或实际系统声音验证。只更新原静态预览构建，未改监听范围、防火墙、GUI 或真实 agent。

0.4 验证记录（2026-09-16）：30 项数据层测试、25 项浏览器测试、Vue 类型/生产构建及 OpenSpec strict validate 通过。覆盖 16 项注册表/OpenAPI 校验、旧厂商回退、新厂商完整型号保存、账号适配器限制、中文/英文搜索、空搜索结果、地区提示、切换后清除认证/选择、320px 布局及 Ollama 空演示密钥。修正 PrimeVue 4 的搜索框无障碍属性绑定，并区分可见空结果和屏幕阅读器 live region。通过局域网构建 URL 的 390px Android UA/触控模拟验证硅基流动搜索、型号保存、刷新恢复，无页面错误、外部请求或横向溢出（不是 Android 真机验收）。HTTP 200，仍仅绑定 10.1.2.237:4173。主 JS 约 520 kB，保留非阻塞构建体积提示；未连接真实 API/OAuth 或修改现有 GUI/agent。

0.3 验证记录（2026-09-16）：构建、27 项数据层测试、23 项浏览器测试与 OpenSpec strict validate 通过。局域网 URL 的 390px Android UA / 触控模拟验证了加入网络内扫码、没有重复扫一扫菜单、认证前预置列表、自动名称保存/刷新恢复，无页面错误或外部请求。未连接真实模型 API、OAuth 或修改现有 GUI/agent。主 JS chunk 约 514 kB，构建仍有体积提示，非错误。

0.2 验证记录（2026-09-14）：`npm test` 23 项通过，`npm run test:e2e` 20 项通过，类型/生产构建通过，OpenSpec strict validate 通过。构建有约 511 kB 主 JS chunk 的体积提示，非错误。新增测试包含包含关系冲突、DHCP 等待/冲突/取消、旧快照迁移、随机密钥显隐、多节点独立测试、厂商/认证切换、会话模型入口、桌面文件菜单、移动横屏底栏。经 `http://10.1.2.237:4173/` 的 Android UA / 触控 / 1024px 平板浏览器模拟检查底栏、多网络、随机密钥、扫一扫、厂商地址通过，无页面错误或外部请求；这仍是开发机浏览器模拟，不是 Android 真机测试。

2026-09-14 用户指定监听 `10.1.2.237`。已将当前开发机的预览从回环地址切换到 `http://10.1.2.237:4173/`，仅绑定该网卡；以 `npm run preview:lan` 启动静态构建预览，未修改防火墙或其他服务。通过该 URL 验证 HTTP 200 和 390px 浏览器的模型初始化、创建网络/名片、任务提交，无页面错误。此检查从开发机进行，不冒充其他设备实测；用户体验验收仍待完成。

局域网 HTTP 不是安全上下文，实测不提供 crypto.randomUUID。新增基于 crypto.getRandomValues 的演示 UUID 工具并替换相关调用，避免新建会话/网络失败；补充回归测试后数据层 15 项通过，生产构建通过。剪贴板不可用时仍提供手动复制 URL。

## 技术验收记录

- Vue TypeScript 类型检查与 Vite 生产构建通过；页面按需分包。最终 14 个数据层测试、15 个浏览器测试通过；最后一轮浏览器测试直接针对构建产物的回环预览运行，HTTP 200。
- 数据层覆盖 OpenAPI 快照校验、既有 RunStatus 对照、初始化幂等、共享连接、型号删除、远端任务恢复、并发/排队、取消、离线拒绝、名片字段、地址验证、失败/unsupported、草稿目标隔离、服务会话改名/移除、一次性同步与禁止复制账号 session。
- 浏览器覆盖 320 / 360 / 390 / 640 / 768 / 1024 / 1440px，各尺寸会话、服务、网络、模型和设置无整页横向溢出；连续缩放、768×390 横屏、20px 字体、长名称/URL、键盘跳转、弹窗 Escape、主题和窗口外观独立性。
- 首轮连续缩放检测捕获下拉面板退场动画的暂时溢出，稳定布局检查后通过；补充了平台选择器的显式可访问名称，修复跳转主内容链接误改 hash 路由问题。
- 手动查看桌面/手机服务页截图，确认桌面三列卡片、移动单列与底部导航。截图只作技术检查，不代替用户验收。
- npm 审计发现测试依赖 Ajv 8.17.1 的已知问题，升级至 8.20.0；当前审计为 0 vulnerabilities。
- 交互模拟无外部 HTTP 请求。补充 hasTouch / isMobile 浏览器上下文的 tap 导航、服务访问和模型表单测试，修复装饰图标进入可访问名称的问题。相机、麦克风、原生手势及系统窗口能力未做实机验证，浏览器触控模拟不算移动平台验收。
