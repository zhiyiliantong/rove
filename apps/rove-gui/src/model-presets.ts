// Static reference metadata reused from the approved prototype, 2026-09-16.
// Not an account entitlement list; no prototype mock API or credentials are imported.
export const model_presets = [
  {
    "id": "openai",
    "label": "OpenAI",
    "base_url": "https://api.openai.com/v1",
    "models": [
      "gpt-6-astra",
      "gpt-5.6-sol",
      "gpt-5.6-terra",
      "gpt-5.6-luna"
    ]
  },
  {
    "id": "deepseek",
    "label": "DeepSeek",
    "base_url": "https://api.deepseek.com",
    "models": [
      "deepseek-flash",
      "deepseek-v4-pro"
    ]
  },
  {
    "id": "anthropic",
    "label": "Anthropic（Claude）",
    "base_url": "https://api.anthropic.com",
    "models": [
      "claude-fable-5-1",
      "claude-opus-5",
      "claude-sonnet-5",
      "claude-haiku-4-5-20251001"
    ]
  },
  {
    "id": "gemini",
    "label": "Google（Gemini）",
    "base_url": "https://generativelanguage.googleapis.com",
    "models": [
      "gemini-3.8-flash"
    ]
  },
  {
    "id": "moonshot",
    "label": "Moonshot（月之暗面 / Kimi）",
    "base_url": "https://api.moonshot.cn/v1",
    "models": [
      "kimi-k2.6"
    ]
  },
  {
    "id": "zai",
    "label": "Z.ai（GLM / 智谱国际站）",
    "base_url": "https://api.z.ai/api/paas/v4",
    "models": [
      "glm-4.6"
    ]
  },
  {
    "id": "minimax",
    "label": "MiniMax",
    "base_url": "https://api.minimax.io/v1",
    "models": [
      "MiniMax-M2.7"
    ]
  },
  {
    "id": "mistral",
    "label": "Mistral AI",
    "base_url": "https://api.mistral.ai/v1",
    "models": [
      "mistral-small-latest"
    ]
  },
  {
    "id": "xai",
    "label": "xAI（Grok）",
    "base_url": "https://api.x.ai/v1",
    "models": [
      "grok-4.6"
    ]
  },
  {
    "id": "openrouter",
    "label": "OpenRouter",
    "base_url": "https://openrouter.ai/api/v1",
    "models": [
      "~openai/gpt-latest"
    ]
  },
  {
    "id": "groq",
    "label": "Groq",
    "base_url": "https://api.groq.com/openai/v1",
    "models": [
      "llama-3.3-70b-versatile",
      "llama-3.1-8b-instant"
    ]
  },
  {
    "id": "ollama",
    "label": "Ollama（本地）",
    "base_url": "http://localhost:11434",
    "models": [
      "qwen3:8b"
    ]
  }
];
