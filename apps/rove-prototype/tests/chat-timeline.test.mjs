import test from 'node:test';
import assert from 'node:assert/strict';
import { createMockApi } from '../src/mock.ts';
import { chatTimeline } from '../src/chat-timeline.ts';
test('each prompt has one assistant task bubble, updated in place on completion and reload', () => {
  let time = 1000, saved;
  const storage = { getItem: () => saved ?? null, setItem: (_key, value) => { saved = value; } };
  const api = createMockApi(storage, () => time);
  const a = api.submit('welcome', '查询服务一', 'local'), b = api.submit('welcome', '查询服务二', 'local');
  const timeline = () => { const s = api.snapshot(); return chatTimeline(s.sessions[0].messages, s.runs); };
  const before = timeline();
  assert.deepEqual(before.filter(e => e.kind === 'run').map(e => e.run.status), ['running', 'queued']);
  for (const rid of [a, b]) { const index = before.findIndex(e => e.kind === 'run' && e.run.id === rid); assert.equal(before[index - 1].message.run_id, rid); }
  time += 14000;
  const after = timeline(); assert.equal(after.length, before.length); assert.equal(after.find(e => e.id === `run-${a}`).run.status, 'succeeded'); assert.match(after.find(e => e.id === `run-${a}`).reply, /演示检查完成/);
  const restored = createMockApi(storage, () => time).snapshot(); assert.deepEqual(chatTimeline(restored.sessions[0].messages, restored.runs), after);
});
test('unlinked historical messages and jobs remain visible without inventing associations', () => {
  const api = createMockApi(); const rid = api.submit('welcome', '查询服务', 'local'); const s = api.snapshot();
  const messages = s.sessions[0].messages.map(({ run_id, ...message }) => message);
  messages.push({ id: 'old-answer', role: 'assistant', text: '旧答复' });
  const timeline = chatTimeline(messages, s.runs);
  assert.equal(timeline.filter(e => e.kind === 'message').length, messages.length);
  assert.equal(timeline.at(-1).run.id, rid); assert.equal(timeline.at(-1).reply, undefined);
});
