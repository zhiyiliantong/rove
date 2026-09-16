import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import Ajv from 'ajv/dist/2020.js';
import { createMockApi, seed } from '../src/mock.ts';
import { generatedModelNames } from '../src/model-catalog.ts';
test('editable names preserve exact model IDs and shared credentials across reload',()=>{
 let saved; const storage={getItem:()=>saved??null,setItem:(_key,value)=>{saved=value;}};
 const api=createMockApi(storage); const input={name:'写代码',provider:'minimax',base_url:'https://api.minimax.io/v1',auth_kind:'api_key',models:['MiniMax-M2.7','private/model:1'],model_names:{'MiniMax-M2.7':'  我的编码助手  '}};
 const spec=JSON.parse(readFileSync(new URL('../api/prototype.openapi.json',import.meta.url)));
 const valid=new Ajv({strict:false}).compile({$ref:'#/components/schemas/ModelImport',components:spec.components}); assert.ok(valid(input),JSON.stringify(valid.errors));
 const ids=api.importModels(input), before=api.snapshot();const imported=before.models.filter(m=>ids.includes(m.id));
 assert.deepEqual(imported.map(m=>m.name),['我的编码助手','minimax-private-model-1-01']);assert.deepEqual(imported.map(m=>m.model),input.models);assert.equal(imported[0].connection_id,imported[1].connection_id);
 const after=createMockApi(storage).snapshot();assert.deepEqual(after.models,before.models);assert.deepEqual(after.connections,before.connections);
});
test('blank or duplicate manual names reject atomically including duplicates with generated defaults',()=>{
 const api=createMockApi(); const input={name:'connection',provider:'openai',base_url:'https://api.openai.com/v1',auth_kind:'api_key',models:['alpha','beta']};
 for(const names of [{alpha:'  '},{alpha:'同名',beta:'同名'},{alpha:'custom-demo-chat-01'},{alpha:'openai-beta-01'}]){
  const before=api.snapshot();assert.throws(()=>api.importModels({...input,model_names:names}),/名称不能为空|名称重复/);assert.deepEqual(api.snapshot(),before);
 }
});
const spec=JSON.parse(readFileSync(new URL('../api/prototype.openapi.json',import.meta.url)));
const validate=new Ajv({strict:false}).compile({$ref:'#/components/schemas/ModelCatalog',components:spec.components});
test('API and account forms can show vendor presets before authentication',()=>{
 const api=createMockApi();
 for(const provider of ['openai','anthropic','deepseek'])for(const auth_kind of ['api_key','official_agent']){
  const c=api.listModelCatalog({provider,auth_kind,refresh:false}); assert.ok(c.models.length); assert.equal(c.source,'preset'); assert.equal(c.checked_on,'2026-09-16'); assert.ok(validate(c),JSON.stringify(validate.errors));
  assert.equal(c.models.some(m=>m.id.startsWith('demo-')),false);
 }
 assert.equal(api.listModelCatalog({provider:'custom',auth_kind:'api_key',refresh:false}).models.length,0);
});
test('discovery carries mock source and rejects unsupported account adapter; failure keeps presets',()=>{
 const api=createMockApi();
 for(const auth_kind of ['api_key','official_agent']){const c=api.listModelCatalog({provider:'openai',auth_kind,refresh:true});assert.equal(c.source,auth_kind==='api_key'?'mock_api':'mock_account');assert.ok(validate(c));}
 assert.throws(()=>api.listModelCatalog({provider:'deepseek',auth_kind:'official_agent',refresh:true}),/尚未接入/);
 api.reset('failure'); assert.throws(()=>api.listModelCatalog({provider:'openai',auth_kind:'api_key',refresh:true}),/保留本地预置/);
 assert.ok(api.listModelCatalog({provider:'openai',auth_kind:'api_key',refresh:false}).models.length);
});
test('generated names retain exact model IDs, disambiguate repeated batches and share credentials',()=>{
 const api=createMockApi(); const input={name:'shared',provider:'openai',base_url:'https://api.openai.com/v1',auth_kind:'api_key',models:['gpt-6-astra','gpt-5.6-sol','gpt-6-astra']};
 const ids=api.importModels(input); const s=api.snapshot(); const models=s.models.filter(m=>ids.includes(m.id));
 assert.deepEqual(models.map(m=>m.name),['openai-gpt-6-astra-01','openai-gpt-5.6-sol-01']);assert.equal(models[0].model,'gpt-6-astra');assert.equal(models[0].connection_id,models[1].connection_id);
 const more=api.importModels(input);assert.equal(api.snapshot().models.find(m=>m.id===more[0]).name,'openai-gpt-6-astra-02');
 assert.deepEqual(generatedModelNames('deepseek',['deepseek-flash'],[]),['deepseek-flash-01']);
 assert.equal(generatedModelNames('openai',['gpt6'],['openai-gpt6-01'])[0],'openai-gpt6-02');
 assert.equal(generatedModelNames('openai',['gpt6'],['openai-gpt6-03'])[0],'openai-gpt6-04');
});
test('v2 migration adds names without changing credentials, model IDs, defaults or drafts',()=>{
 const old=seed('daily');old.version=2;old.models.forEach(m=>delete m.name);old.sessions[0].draft='草稿保留';
 const api=createMockApi({getItem:()=>JSON.stringify(old),setItem:()=>{}});const s=api.snapshot();assert.equal(s.version,4);assert.equal(s.sessions[0].draft,'草稿保留');assert.equal(s.default_model_id,old.default_model_id);assert.deepEqual(s.connections,old.connections);assert.equal(s.models[0].id,old.models[0].id);assert.ok(s.models.every(m=>m.name));
 const validateSnapshot=new Ajv({strict:false}).compile({$ref:'#/components/schemas/Snapshot',components:spec.components}); assert.ok(validateSnapshot(s),JSON.stringify(validateSnapshot.errors));
});
