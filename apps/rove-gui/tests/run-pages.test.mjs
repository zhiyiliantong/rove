import {test} from 'node:test';
import assert from 'node:assert/strict';
import {merge_run_page} from '../src/run-pages.ts';
const run=(run_id,status='queued')=>({run_id,session_id:'s',created_at:'2026-09-08T00:00:00Z',status,input_message:'hello',error:null});

test('refresh keeps loaded pages and only updates matching run identities',()=>{
  const first=[run('a'),run('b')];
  const expanded=merge_run_page(first,[run('c'),run('d')]);
  const refreshed=merge_run_page(expanded,[run('c','running'),run('d')]);
  assert.deepEqual(refreshed.map(r=>r.run_id),['a','b','c','d']);
  assert.equal(refreshed[2].status,'running');
  assert.equal(expanded[2].status,'queued');
  assert.equal(first.length,2);
});
test('overlapping pages deduplicate and new tail runs remain visible',()=>{
  const visible=[run('a'),run('b')];
  const next=merge_run_page(visible,[run('b','succeeded'),run('c')]);
  assert.deepEqual(next.map(r=>r.run_id),['a','b','c']);
  assert.equal(next[1].status,'succeeded');
  assert.deepEqual(merge_run_page(next,[]),next);
  const immediate=merge_run_page(visible,[run('z')]);
  assert.deepEqual(merge_run_page(immediate,[run('c')]).map(r=>r.run_id),['a','b','c','z']);
});
