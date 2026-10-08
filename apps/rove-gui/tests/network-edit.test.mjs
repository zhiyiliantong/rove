import {test} from 'node:test';
import assert from 'node:assert/strict';
import {save_network_edit,NetworkEditError} from '../src/network-edit.ts';

function fixture(states,fail){
  const calls=[];
  return {calls,call:async(op,body,path)=>{calls.push({op,body,path});if(op===fail)throw new Error('fixture failure');if(op==='get_network')return {state:states.shift()??'starting'};return {};}};
}
const sleep=async()=>{};
test('running and starting networks stop, wait, save, start and wait',async()=>{
  for(const initial of ['running','starting']){
    const f=fixture([initial,'stopping','stopped','starting','running']),phases=[];
    await save_network_edit(f.call,'n1',{display_name:'new'},p=>phases.push(p),sleep);
    assert.deepEqual(f.calls.map(c=>c.op),['get_network','stop_network','get_network','get_network','update_network','start_network','get_network','get_network']);
    assert.deepEqual(phases,['checking','stopping','saving','starting']);
    assert.ok(f.calls.every(c=>c.path.network_id==='n1'));
  }
});
test('stopped network saves without starting',async()=>{
  const f=fixture(['stopped']);await save_network_edit(f.call,'n1',{},undefined,sleep);
  assert.deepEqual(f.calls.map(c=>c.op),['get_network','update_network']);
});
test('stopping network finishes stopping but does not undo user stop intent',async()=>{
  const f=fixture(['stopping','stopped']);await save_network_edit(f.call,'n1',{},undefined,sleep);
  assert.ok(!f.calls.some(c=>c.op==='start_network'));
});
test('errors preserve stage and saved outcome; no later mutations',async()=>{
  for(const [fail,stage,saved] of [['get_network','checking',false],['stop_network','stopping',false],['update_network','saving',false],['start_network','starting',true]]){
    const f=fixture(['running','stopped'],fail);
    await assert.rejects(save_network_edit(f.call,'n1',{},undefined,sleep),e=>e instanceof NetworkEditError&&e.stage===stage&&e.saved===saved);
    assert.equal(f.calls.at(-1).op,fail);
  }
});
test('stop timeout never writes; start timeout reports saved configuration',async()=>{
  for(const [states,stage,saved] of [[['running','stopping','stopping'],'stopping',false],[['running','stopped','starting','starting'],'starting',true]]){
    const f=fixture(states);
    await assert.rejects(save_network_edit(f.call,'n1',{},undefined,sleep,2),e=>e.stage===stage&&e.saved===saved&&e.message.includes('Timed out'));
    assert.equal(f.calls.some(c=>c.op==='update_network'),saved);
  }
});
test('runtime failure is not success',async()=>{
  const f=fixture(['running','stopped','failed']);
  await assert.rejects(save_network_edit(f.call,'n1',{},undefined,sleep),e=>e.stage==='starting'&&e.saved);
});
