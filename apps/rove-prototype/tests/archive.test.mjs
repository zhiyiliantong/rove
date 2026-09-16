import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import Ajv from 'ajv/dist/2020.js';
import { createMockApi, seed } from '../src/mock.ts';
const contract = JSON.parse(readFileSync(new URL('../api/prototype.openapi.json', import.meta.url)));
const ajv = new Ajv({ strict: false });
const valid = ajv.compile({ $ref: '#/components/schemas/Snapshot', components: contract.components });
const errorValid = ajv.compile(contract.components.schemas.Error);
function harness(initial = seed('daily')) {
  let saved = JSON.stringify(initial), time = 1000;
  const storage = { getItem: () => saved, setItem: (_, value) => { saved = value; } };
  return { storage, api: createMockApi(storage, () => time), advance: () => { time += 14000; }, now: () => time, raw: () => JSON.parse(saved) };
}
test('archive and restore preserve identity, drafts, references and timestamps across reload', () => {
  const h = harness(); h.api.saveDraft('welcome', '保留草稿'); h.api.setSessionTarget('welcome', 'windows');
  const before = h.api.snapshot(); h.api.archiveSession('welcome'); h.advance(); h.api.archiveSession('welcome');
  assert.equal(h.api.snapshot().sessions[0].archived_at, 1000);
  const reopened = createMockApi(h.storage, h.now); assert.equal(reopened.snapshot().sessions[0].archived_at, 1000);
  reopened.restoreSession('welcome'); reopened.restoreSession('welcome');
  assert.deepEqual(reopened.snapshot(), before); assert.ok(valid(reopened.snapshot()), JSON.stringify(valid.errors));
});
test('archived tasks continue and deletion only removes terminal history, not services', () => {
  const h = harness(); const other = h.api.createSession();
  const rid = h.api.submit('welcome', '安装音乐播放器', 'windows'); h.api.snapshot(); h.api.archiveSession('welcome');
  assert.throws(() => h.api.deleteSession('welcome'), { code: 'session_has_active_runs', status: 409 });
  h.advance(); const done = createMockApi(h.storage, h.now); const before = done.snapshot();
  assert.equal(before.runs[0].status, 'succeeded'); assert.ok(before.services.some(s => s.id === rid));
  assert.ok(before.sessions.find(s => s.id === 'welcome').messages.some(m => m.role === 'assistant' && m.run_id === rid));
  done.deleteSession('welcome'); const after = createMockApi(h.storage, h.now).snapshot();
  assert.equal(after.runs.length, 0); assert.equal(after.sessions.length, 1); assert.equal(after.sessions[0].id, other);
  for (const key of ['services','models','networks','devices','connections']) assert.deepEqual(after[key], before[key]);
  assert.ok(valid(after), JSON.stringify(valid.errors));
});
test('queued, running and cancelling tasks all block deletion without mutation', () => {
  for (const status of ['queued','running','cancelling']) {
    const h = harness(); h.api.submit('welcome', '查询', 'local'); h.api.archiveSession('welcome');
    const initial = h.raw(); initial.runs[0].status = status;
    const t = harness(initial); const before = t.raw();
    assert.throws(() => t.api.deleteSession('welcome'), { code: 'session_has_active_runs' }); assert.deepEqual(t.raw(), before);
  }
});
test('all terminal states allow deletion', () => {
  for (const status of ['succeeded','failed','cancelled','interrupted']) {
    const h = harness(); h.api.submit('welcome', '查询', 'local'); h.api.archiveSession('welcome');
    const initial = h.raw(); initial.runs[0].status = status; const t = harness(initial);
    t.api.deleteSession('welcome'); assert.equal(t.api.snapshot().sessions.length, 0); assert.equal(t.api.snapshot().runs.length, 0);
  }
});
test('archived sessions reject edits and submission; unknown and nonarchived errors match contract', () => {
  const h = harness(); assert.throws(() => h.api.deleteSession('welcome'), { code: 'session_not_archived', status: 409 });
  h.api.archiveSession('welcome'); const before = h.raw();
  for (const action of [() => h.api.saveDraft('welcome','changed'), () => h.api.setSessionModel('welcome','model-fast'), () => h.api.setSessionTarget('welcome','windows'), () => h.api.submit('welcome','查询','local')]) {
    assert.throws(action, { code: 'session_archived', status: 409 }); assert.deepEqual(h.raw(), before);
  }
  for (const method of ['archiveSession','restoreSession','deleteSession']) {
    assert.throws(() => h.api[method]('missing'), error => error.status === 404 && errorValid({message:error.message,code:error.code}));
  }
  for (const [path, method, id] of [['/sessions/{id}/archive','post','archiveSession'],['/sessions/{id}/archive','delete','restoreSession'],['/sessions/{id}','delete','deleteSession']]) {
    const op = contract.paths[path][method]; assert.equal(op.operationId, id); assert.ok(op.responses['204']); assert.ok(op.responses['404']); assert.equal(op.requestBody, undefined);
  }
  for (const path of ['draft','model','target','runs']) assert.ok(Object.values(contract.paths[`/sessions/{id}/${path}`])[0].responses['409']);
});
test('v1/v2/v3 migration preserves old content and initializes archive state', () => {
  for (const version of [1,2,3]) {
    const initial = seed('daily'); initial.version = version; initial.sessions[0].draft = '旧草稿'; delete initial.sessions[0].archived_at;
    if (version === 1) initial.active_network_id = 'home';
    const h = harness(initial); const s = h.api.snapshot(); assert.equal(s.version, 4); assert.equal(s.sessions[0].archived_at, null); assert.equal(s.sessions[0].draft, '旧草稿'); assert.ok(valid(s), JSON.stringify(valid.errors));
    h.api.archiveSession('welcome'); assert.equal(createMockApi(h.storage, h.now).snapshot().sessions[0].archived_at, 1000);
  }
});
test('deleting onboarding does not trigger onboarding again; no storage remains usable', () => {
  const api = createMockApi(); api.reset('empty');
  const input = { name:'演示',provider:'custom',base_url:'https://api.example.invalid/v1',auth_kind:'api_key',models:['demo'] };
  api.importModels(input); const sid = api.snapshot().sessions[0].id; api.archiveSession(sid); api.deleteSession(sid);
  api.importModels(input); assert.equal(api.snapshot().sessions.length, 0); assert.equal(api.snapshot().initialized, true);
});
