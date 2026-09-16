import {test} from 'node:test';
import assert from 'node:assert/strict';
import {apply_events} from '../src/run-events.ts';
const snapshot=()=>({run:{run_id:'a',session_id:'s',created_at:'2026-09-08T00:00:00Z',status:'running',input_message:'hello',error:null},output_tail:'',output_truncated:false,snapshot_seq:0});
const event=(seq,text='漫游者')=>({run_id:'a',seq,kind:'assistant_delta',data:{text}});
test('replay, status, and Unicode tail preserve the exact watermark',()=>{
  const initial=snapshot();
  const first=apply_events(initial,{events:[event(1,'🌍'.repeat(65537))],last_seq:1,terminal:false});
  assert.equal(initial.snapshot_seq,0);
  assert.equal(Array.from(first.output_tail).length,65536);
  assert.equal(first.output_truncated,true);
  const next=apply_events(first,{events:[event(1),{run_id:'a',seq:2,kind:'status',data:{status:'succeeded'}}],last_seq:2,terminal:true});
  assert.equal(next.output_tail,first.output_tail);
  assert.equal(next.snapshot_seq,2);
  assert.equal(next.run.status,'succeeded');
  assert.equal(first.run.status,'running');
});
test('wrong run, sequence gaps, and invalid batch watermark reject atomically',()=>{
  for(const batch of [
    {events:[event(1),event(3)],last_seq:3},
    {events:[{...event(1),run_id:'b'}],last_seq:1},
    {events:[],last_seq:1},
    {events:[event(Number.MAX_SAFE_INTEGER+1)],last_seq:0},
  ]){
    const initial=snapshot();
    assert.throws(()=>apply_events(initial,{...batch,terminal:false}));
    assert.equal(initial.snapshot_seq,0);
    assert.equal(initial.output_tail,'');
  }
});
