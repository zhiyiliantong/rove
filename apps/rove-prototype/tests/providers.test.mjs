import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import Ajv from 'ajv/dist/2020.js';
import { providers, providerConfig, RIG_VERSION } from '../src/providers.ts';
import { createMockApi } from '../src/mock.ts';
const spec = JSON.parse(readFileSync(new URL('../api/prototype.openapi.json', import.meta.url)));
const ajv = new Ajv({ strict: false });
const validate = ajv.compile({ $ref: '#/components/schemas/Provider', components: spec.components });
const validateSnapshot = ajv.compile({ $ref: '#/components/schemas/Snapshot', components: spec.components });

test('new provider catalog is pinned to dedicated Rig modules; removed providers have no preset models', () => {
  const mapping = { openai: 'openai', deepseek: 'deepseek', anthropic: 'anthropic', google: 'gemini', moonshot: 'moonshot', zai: 'zai', minimax: 'minimax', mistral: 'mistral', xai: 'xai', openrouter: 'openrouter', groq: 'groq', ollama: 'ollama' };
  assert.deepEqual(Object.fromEntries(providers.map(p => [p.id, p.rig_provider])), mapping);
  const lock = readFileSync(new URL('../../../Cargo.lock', import.meta.url), 'utf8');
  assert.ok(lock.includes(`name = "rig-core"\nversion = "${RIG_VERSION}"`));
  const api = createMockApi();
  for (const provider of ['qwen', 'volcengine', 'siliconflow', 'zhipu', 'custom']) {
    assert.equal(providers.some(p => p.id === provider), false);
    assert.equal(providerConfig(provider).rig_provider, null);
    assert.equal(api.listModelCatalog({ provider, auth_kind: 'api_key', refresh: false }).models.length, 0);
  }
  assert.equal(providerConfig('google').base_url, 'https://generativelanguage.googleapis.com');
  assert.equal(providerConfig('ollama').base_url, 'http://localhost:11434');
  assert.equal(providerConfig('zai').base_url, 'https://api.z.ai/api/paas/v4');
});

test('12 unique provider presets satisfy the OpenAPI contract and preserve custom fallback', () => {
  const api = createMockApi(), list = api.listProviders();
  assert.equal(list.length, 12); assert.equal(new Set(list.map(p => p.id)).size, list.length);
  assert.equal(spec.paths['/providers'].get.operationId, 'demo_list_providers');
  for (const p of list) {
    assert.ok(validate(p), JSON.stringify(validate.errors));
    assert.equal(new URL(p.base_url).username, ''); assert.ok(p.note);
    if (!p.allow_empty_key) assert.equal(new URL(p.base_url).protocol, 'https:');
    if (p.id !== 'custom') assert.equal(new URL(p.docs).protocol, 'https:');
  }
  list[0].base_url = 'https://changed.example.invalid'; list.pop();
  assert.equal(api.listProviders().length, 12); assert.equal(api.listProviders()[0].base_url, providers[0].base_url);
  for (const id of ['unknown', '兼容接口', '官方 Agent（演示）']) assert.equal(providerConfig(id).id, 'custom');
});

test('new vendors have reference models, preserve exact IDs and never gain account login implicitly', () => {
  const api = createMockApi();
  for (const p of api.listProviders()) {
    const c = api.listModelCatalog({ provider: p.id, auth_kind: 'api_key', refresh: false });
    assert.equal(c.source, 'preset'); assert.equal(c.models.length > 0, p.id !== 'custom');
    assert.equal(new Set(c.models.map(m => m.id)).size, c.models.length);
    assert.equal(api.listModelCatalog({ provider: p.id, auth_kind: 'api_key', refresh: true }).source, 'mock_api');
    if (!p.account_adapter) assert.throws(() => api.listModelCatalog({ provider: p.id, auth_kind: 'official_agent', refresh: true }), /尚未接入/);
    const models = c.models.map(m => m.id); if (!models.length) models.push('private/model:1');
    const ids = api.importModels({ name: p.id, provider: p.id, base_url: p.base_url, auth_kind: 'api_key', models });
    const snapshot = api.snapshot(); assert.ok(validateSnapshot(snapshot), JSON.stringify(validateSnapshot.errors));
    const imported = snapshot.models.filter(m => ids.includes(m.id));
    assert.deepEqual(imported.map(m => m.model), models); assert.ok(imported.every(m => m.name.endsWith('-01')));
  }
});

test('legacy and newly added providers survive reload with endpoint, names and credentials unchanged', () => {
  let saved; const storage = { getItem: () => saved ?? null, setItem: (_key, value) => { saved = value; } };
  const api = createMockApi(storage);
  for (const provider of ['qwen', 'volcengine', 'siliconflow', 'zhipu', 'custom', 'google', 'ollama']) {
    // Seed pre-filter records through the backward-compatible fixture API.
    api.importModels({ name: 'legacy-' + provider, provider, base_url: 'https://old.example.invalid/v1', auth_kind: 'api_key', models: ['Old/Model:1'] });
  }
  api.importModels({ name: 'custom-url', provider: 'minimax', base_url: 'https://proxy.example.invalid/v1', auth_kind: 'api_key', models: ['MiniMax-M2.7'] });
  const before = api.snapshot(), after = createMockApi(storage).snapshot();
  assert.deepEqual(after.connections, before.connections); assert.deepEqual(after.models, before.models);
  assert.equal(after.version, 4); assert.equal(after.default_model_id, before.default_model_id);
});
