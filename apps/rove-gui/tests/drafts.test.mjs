import test from 'node:test';
import assert from 'node:assert/strict';
import {DRAFT_KEY,load_drafts,save_drafts} from '../src/drafts.ts';
test('unsent drafts survive a document restart, with sent and deleted entries removed',()=>{
  const values=new Map();const storage={getItem:key=>values.get(key)??null,setItem:(key,value)=>values.set(key,value)};
  const drafts={session1:'保留原文',session2:'unchanged'};assert.equal(save_drafts(storage,drafts),true);
  assert.deepEqual(load_drafts(storage),drafts);delete drafts.session1;drafts.session2='';save_drafts(storage,drafts);
  assert.deepEqual(load_drafts(storage),{});assert.deepEqual([...values.keys()],[DRAFT_KEY]);
});
test('malformed, oversized and unavailable draft storage are safe',()=>{
  assert.deepEqual(load_drafts({getItem:()=>'{bad'}),{});
  assert.deepEqual(load_drafts({getItem:()=>JSON.stringify({version:1,drafts:{s:'x'.repeat(65537),ok:'yes',notText:3}})}),{ok:'yes'});
  assert.equal(save_drafts(undefined,{}),false);assert.equal(save_drafts({setItem:()=>{throw Error();}},{}),false);
});
