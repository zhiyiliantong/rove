// Default addresses and new presets reviewed against official docs on 2026-09-16.
// Form metadata only: no network request, installed adapter, or account entitlement implied.
export interface Provider {
  id: string; label: string; category: '模型厂商' | '聚合 / 推理平台' | '本地 / 自定义';
  base_url: string; account_adapter: string | null; docs: string;
  rig_provider: string | null;
  keywords: string; note: string; allow_empty_key: boolean;
}
const vendor = { category: '模型厂商', account_adapter: null, allow_empty_key: false } as const;
const platform = { ...vendor, category: '聚合 / 推理平台' } as const;
export const RIG_VERSION = '0.42.0';
// Kept separately from legacy metadata so retired vendors cannot reappear in Add Model.
const registry: Provider[] = [
  { ...vendor, id: 'openai', rig_provider: 'openai', label: 'OpenAI', base_url: 'https://api.openai.com/v1', account_adapter: 'Codex', docs: 'https://developers.openai.com/api/reference/overview', keywords: 'GPT ChatGPT Codex', note: 'API 与 Codex 账号权益不同；账号登录仅演示。' },
  { ...vendor, id: 'deepseek', rig_provider: 'deepseek', label: 'DeepSeek', base_url: 'https://api.deepseek.com', docs: 'https://api-docs.deepseek.com/', keywords: '深度求索', note: '使用开放平台 API 密钥，不是聊天网站账号。' },
  { ...vendor, id: 'anthropic', rig_provider: 'anthropic', label: 'Anthropic（Claude）', base_url: 'https://api.anthropic.com', account_adapter: 'Claude Code', docs: 'https://platform.claude.com/docs/en/api/overview', keywords: 'Claude Code', note: '原生 Anthropic 接口；不能只替换地址就当作通用兼容接口。' },
  { ...vendor, id: 'google', rig_provider: 'gemini', label: 'Google（Gemini）', base_url: 'https://generativelanguage.googleapis.com', docs: 'https://ai.google.dev/gemini-api/docs/openai', keywords: '谷歌 AI Studio', note: '默认 Rig Gemini 原生接口根地址；不是 OpenAI 兼容入口或 Vertex AI。' },
  { ...vendor, id: 'qwen', rig_provider: null, label: '阿里云百炼（Qwen）', base_url: 'https://dashscope.aliyuncs.com/compatible-mode/v1', docs: 'https://help.aliyun.com/zh/model-studio/model-calling-in-sub-workspace', keywords: '通义千问 阿里巴巴 Alibaba DashScope', note: '默认北京地域兼容地址；其他地域、新工作空间或套餐请按控制台修改地址并使用对应密钥，勿混用 Coding Plan / Token Plan。' },
  { ...vendor, id: 'moonshot', rig_provider: 'moonshot', label: 'Moonshot（月之暗面 / Kimi）', base_url: 'https://api.moonshot.cn/v1', docs: 'https://platform.kimi.com/docs/api/quickstart', keywords: '月之暗面 kimi', note: '默认国内开放平台地址；其他站点或订阅套餐需核对专用地址与密钥。' },
  { ...vendor, id: 'zai', rig_provider: 'zai', label: 'Z.ai（GLM / 智谱国际站）', base_url: 'https://api.z.ai/api/paas/v4', docs: 'https://docs.rs/rig-core/0.42.0/rig_core/providers/zai/index.html', keywords: '智谱 GLM Zhipu BigModel', note: '默认 Rig Z.ai 国际站通用 API；国内 BigModel 与 Coding Plan 地址和密钥不能直接混用。' },
  { ...vendor, id: 'zhipu', rig_provider: null, label: '智谱（GLM）', base_url: 'https://open.bigmodel.cn/api/paas/v4/', docs: 'https://docs.bigmodel.cn/cn/guide/capabilities/thinking-mode', keywords: 'BigModel Z AI Zhipu', note: '默认国内通用 API；GLM Coding Plan 使用不同入口，不是通用 API 账号登录。' },
  { ...vendor, id: 'minimax', rig_provider: 'minimax', label: 'MiniMax', base_url: 'https://api.minimax.io/v1', docs: 'https://platform.minimax.io/docs/api-reference/text-prompt-caching', keywords: '海螺 稀宇', note: '默认国际站 OpenAI 兼容地址；国内站、Anthropic 兼容入口及套餐需按对应文档修改。' },
  { ...vendor, id: 'volcengine', rig_provider: null, label: '火山方舟（豆包 / Doubao）', base_url: 'https://ark.cn-beijing.volces.com/api/v3', docs: 'https://www.volcengine.com/docs/82379/1795150', keywords: '字节跳动 ByteDance Volcengine Ark', note: '默认北京地域通用 API；请确认模型已开通，专属部署可手动填 ep- 接入点 ID。Coding Plan 须核对专用地址。' },
  { ...vendor, id: 'mistral', rig_provider: 'mistral', label: 'Mistral AI', base_url: 'https://api.mistral.ai/v1', docs: 'https://docs.mistral.ai/api', keywords: 'Codestral Devstral', note: '默认通用 API；latest 型号别名会随平台更新，网页会员不等于 API 授权。' },
  { ...vendor, id: 'xai', rig_provider: 'xai', label: 'xAI（Grok）', base_url: 'https://api.x.ai/v1', docs: 'https://docs.x.ai/developers/quickstart', keywords: 'Grok SpaceXAI', note: '使用开发者 API 密钥；Grok / X 订阅不代表 API 调用权限。' },
  { ...platform, id: 'siliconflow', rig_provider: null, label: '硅基流动（SiliconFlow）', base_url: 'https://api.siliconflow.cn/v1', docs: 'https://docs.siliconflow.cn/docs/userguide/quickstart', keywords: 'SiliconCloud', note: '默认国内站；平台型号 ID 包含组织前缀，不能直接使用模型原厂的 ID。' },
  { ...platform, id: 'openrouter', rig_provider: 'openrouter', label: 'OpenRouter', base_url: 'https://openrouter.ai/api/v1', docs: 'https://openrouter.ai/docs/quickstart', keywords: '聚合 路由', note: '多厂商聚合入口；保留平台完整型号 ID，实际可用性以平台和账号权限为准。' },
  { ...platform, id: 'groq', rig_provider: 'groq', label: 'Groq', base_url: 'https://api.groq.com/openai/v1', docs: 'https://console.groq.com/docs/models', keywords: 'GroqCloud Llama', note: '推理托管平台 Groq，与 xAI 的 Grok 不同；使用 Groq 平台密钥及型号 ID。' },
  { ...vendor, category: '本地 / 自定义', id: 'ollama', rig_provider: 'ollama', label: 'Ollama（本地）', base_url: 'http://localhost:11434', docs: 'https://docs.ollama.com/api/openai-compatibility', keywords: '本机 离线', allow_empty_key: true, note: '默认 Rig Ollama 原生本机服务，密钥可留空；预置只是参考，不代表已经安装。云端或带认证代理需按其要求配置。' },
  { ...vendor, category: '本地 / 自定义', id: 'custom', rig_provider: null, label: '自定义 / 本地接口', base_url: 'http://localhost:11434/v1', docs: '', keywords: 'Custom LM Studio vLLM', allow_empty_key: true, note: '填写自己的兼容接口与型号；是否免密和支持型号枚举由目标服务决定。' },
];
export const providers = registry.filter(p => p.rig_provider !== null);
export function providerConfig(id: string): Provider {
  // Historical metadata only. Never rewrite a saved provider ID, endpoint or model name.
  return registry.find(p => p.id === id) ?? registry.find(p => p.id === 'custom')!;
}
