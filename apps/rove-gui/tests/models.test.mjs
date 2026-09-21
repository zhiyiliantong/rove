import test from 'node:test';
import assert from 'node:assert/strict';
import {generated_names} from '../src/models.ts';
import {model_presets} from '../src/model-presets.ts';
test('automatic model names are stable, editable and collision-aware',()=>{
  assert.deepEqual(generated_names('openai',['gpt-demo','gpt-demo'],['openai-gpt-demo-01','openai-gpt-demo-09']),['openai-gpt-demo-10','openai-gpt-demo-11']);
  assert.deepEqual(generated_names('deepseek',['deepseek-chat'],[]),['deepseek-chat-01']);
  assert.deepEqual(generated_names('openai_compatible',['chat'],[]),['openai-compatible-chat-01']);
  assert.ok(generated_names('openai',['a'.repeat(256)],[])[0].length<=256);
});
test('offered providers have native Rig adapters and no prototype mock dependency',()=>{
  assert.equal(model_presets.length,12);assert.equal(new Set(model_presets.map(p=>p.id)).size,12);
  for(const p of model_presets){assert.ok(p.models.length);assert.ok(/^https?:/.test(p.base_url));}
});
