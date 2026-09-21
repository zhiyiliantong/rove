import {test} from 'node:test';
import assert from 'node:assert/strict';
import {persistCleanReview} from '../src/review-reset.ts';
import {seed,STORAGE_KEY} from '../src/mock.ts';
import {INTRO_KEY} from '../src/introduction.ts';
function memory() { const values=new Map([[STORAGE_KEY,JSON.stringify(seed('daily'))],[INTRO_KEY,JSON.stringify({version:1,completed:true,step:3})],['rove-prototype-theme','dark'],['unrelated','keep']]); return {values,getItem:k=>values.get(k)??null,setItem:(k,v)=>values.set(k,v),removeItem:k=>values.delete(k)}; }
test('clean review writes an empty snapshot and first guide page only',()=>{
  const storage=memory();persistCleanReview(storage);
  assert.deepEqual(JSON.parse(storage.getItem(STORAGE_KEY)),seed('empty'));
  assert.deepEqual(JSON.parse(storage.getItem(INTRO_KEY)),{version:1,completed:false,step:0,resetPending:false});
  assert.equal(storage.getItem('rove-prototype-theme'),'dark');assert.equal(storage.getItem('unrelated'),'keep');
});
test('failed snapshot write restores existing introduction and snapshot',()=>{
  const storage=memory(),before=new Map(storage.values);const write=storage.setItem;
  storage.setItem=(k,v)=>{if(k===STORAGE_KEY)throw Error('blocked');return write(k,v);};
  assert.throws(()=>persistCleanReview(storage),/未能完成清空/);assert.deepEqual(storage.values,before);
  assert.throws(()=>persistCleanReview(),/无法确认旧数据已清空/);
});
