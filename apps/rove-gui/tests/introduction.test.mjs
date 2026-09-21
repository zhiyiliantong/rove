import test from 'node:test';
import assert from 'node:assert/strict';
import {INTRO_KEY,fresh_introduction,load_introduction,save_introduction} from '../src/introduction.ts';
test('introduction replay is isolated from credentials, drafts, theme and execution data',()=>{
  const values=new Map([['draft','keep'],['rove-gui-theme','dark']]);
  const storage={getItem:key=>values.get(key)??null,setItem:(key,value)=>values.set(key,value)};
  assert.deepEqual(load_introduction(storage),fresh_introduction());
  const completed={...fresh_introduction(),completed:true,step:3};
  assert.equal(save_introduction(storage,completed),true);
  assert.equal(load_introduction(storage).completed,true);
  save_introduction(storage,{...completed,reset_pending:true});
  assert.deepEqual(load_introduction(storage),fresh_introduction());
  assert.equal(values.get('draft'),'keep');assert.equal(values.get('rove-gui-theme'),'dark');
  save_introduction(storage,fresh_introduction());assert.equal(JSON.parse(values.get(INTRO_KEY)).reset_pending,false);
});
test('invalid or denied storage never pretends a reset is saved',()=>{
  assert.equal(save_introduction(undefined,fresh_introduction()),false);
  assert.equal(save_introduction({getItem:()=>null,setItem:()=>{}},fresh_introduction()),false);
  assert.deepEqual(load_introduction({getItem:()=>'{broken',setItem:()=>{}}),fresh_introduction());
  assert.equal(load_introduction({getItem:()=>JSON.stringify({version:1,step:100}),setItem:()=>{}}).step,3);
});
