import test from 'node:test';
import assert from 'node:assert/strict';
import {peer_state,peer_target} from '../src/peer-state.ts';
import {set_locale} from '../src/i18n.ts';
set_locale('zh-CN');
const network_id='11111111-1111-4111-8111-111111111111';
const peer={network_id,device_id:'22222222-2222-4222-8222-222222222222',state:'online',capabilities:[]};
test('mobile capability absence does not change peer identity or prevent selection',()=>{
  assert.match(peer_state(peer),/unsupported/);
  assert.deepEqual(peer_target(peer,network_id),{network_id,device_id:peer.device_id});
  assert.equal(peer_state({...peer,capabilities:['system_exec']}),'在线');
});
test('offline, incompatible and cross-network responses cannot silently select a target',()=>{
  for(const state of ['offline','incompatible','unknown'])assert.throws(()=>peer_target({...peer,state},network_id));
  assert.match(peer_state({...peer,state:'offline'}),/离线/);
  assert.match(peer_state({...peer,state:'incompatible'}),/协议不兼容/);
  assert.throws(()=>peer_target(peer,'33333333-3333-4333-8333-333333333333'));
  assert.throws(()=>peer_target({...peer,device_id:'bad-id'},network_id));
});
