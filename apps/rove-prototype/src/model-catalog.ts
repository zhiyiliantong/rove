import type { Connection } from './domain.ts';
export interface CatalogModel { id: string; label: string; note: string }
export interface ModelCatalog { provider: string; auth_kind: Connection['auth_kind']; source: 'preset' | 'mock_api' | 'mock_account'; checked_on: string; models: CatalogModel[]; message: string }
export interface CatalogRequest { provider: string; auth_kind: Connection['auth_kind']; refresh: boolean }
// Official catalogs reviewed 2026-09-16; NOT a user's account entitlement list.
// See docs/prototype-handoff.md for source URLs and adapter limitations.
const presets: Record<string, CatalogModel[]> = {
  openai: [
    { id: 'gpt-6-astra', label: 'GPT-6 Astra', note: '通用 / 推理 / Agent' },
    { id: 'gpt-5.6-sol', label: 'GPT-5.6 Sol', note: '复杂任务' },
    { id: 'gpt-5.6-terra', label: 'GPT-5.6 Terra', note: '均衡型' },
    { id: 'gpt-5.6-luna', label: 'GPT-5.6 Luna', note: '轻量型' },
  ],
  deepseek: [
    { id: 'deepseek-flash', label: 'DeepSeek V4.1 Flash', note: '当前通用型号' },
    { id: 'deepseek-v4-pro', label: 'DeepSeek V4 Pro（兼容 ID）', note: '官方已转路由至 V4.1 Flash，并非独立 Pro 能力' },
  ],
  anthropic: [
    { id: 'claude-fable-5-1', label: 'Claude Fable 5.1', note: '长程任务 / 推理' },
    { id: 'claude-opus-5', label: 'Claude Opus 5', note: '复杂任务' },
    { id: 'claude-sonnet-5', label: 'Claude Sonnet 5', note: '均衡型' },
    { id: 'claude-haiku-4-5-20251001', label: 'Claude Haiku 4.5', note: '轻量型' },
  ],
  google: [{ id: 'gemini-3.8-flash', label: 'Gemini 3.8 Flash', note: '官方兼容接口示例型号' }],
  moonshot: [{ id: 'kimi-k2.6', label: 'Kimi K2.6', note: '官方 API 示例型号' }],
  zai: [{ id: 'glm-4.6', label: 'GLM 4.6', note: 'Rig Z.ai 模块参考型号，不代表账号权限或型号白名单' }],
  minimax: [{ id: 'MiniMax-M2.7', label: 'MiniMax M2.7', note: '保留厂商型号 ID 大小写' }],
  mistral: [{ id: 'mistral-small-latest', label: 'Mistral Small（latest）', note: '动态别名，具体版本由厂商决定' }],
  xai: [{ id: 'grok-4.6', label: 'Grok 4.6', note: '官方快速开始示例，Early Access 需账号权限' }],
  openrouter: [{ id: '~openai/gpt-latest', label: 'GPT 最新旗舰（OpenRouter 别名）', note: '动态别名，不固定具体版本，保留 ~ 和平台前缀' }],
  groq: [
    { id: 'llama-3.3-70b-versatile', label: 'Llama 3.3 70B（Groq）', note: 'Groq 托管型号' },
    { id: 'llama-3.1-8b-instant', label: 'Llama 3.1 8B（Groq）', note: 'Groq 托管轻量型号' },
  ],
  ollama: [{ id: 'qwen3:8b', label: 'Qwen3 8B（Ollama）', note: '官方示例，须先在目标设备安装；本原型不会检测安装情况' }],
  custom: [],
};
export function presetCatalog(provider: string, auth_kind: Connection['auth_kind']): ModelCatalog {
  return { provider, auth_kind, source: 'preset', checked_on: '2026-09-16', models: (presets[provider] ?? []).map(m => ({ ...m })), message: '本地预置 · 未验证账号权限；API 与会员可用型号可能不同。' };
}
function slug(value: string) { return value.toLowerCase().replace(/[^a-z0-9.-]+/g, '-').replace(/^-+|-+$/g, '') || 'model'; }
export function generatedModelNames(provider: string, models: string[], existing: string[]): string[] {
  const used = new Set(existing);
  return models.map(model => {
    const vendor = slug(provider), type = slug(model);
    const prefix = type.startsWith(`${vendor}-`) ? type : `${vendor}-${type}`;
    const previous = [...used].filter(name => name.startsWith(`${prefix}-`)).map(name => name.slice(prefix.length + 1)).filter(suffix => /^\d+$/.test(suffix)).map(Number).filter(Number.isSafeInteger);
    let sequence = previous.reduce((max, number) => Math.max(max, number), 0) + 1;
    while (used.has(`${prefix}-${String(sequence).padStart(2, '0')}`)) sequence++;
    const name = `${prefix}-${String(sequence).padStart(2, '0')}`;
    used.add(name); return name;
  });
}
