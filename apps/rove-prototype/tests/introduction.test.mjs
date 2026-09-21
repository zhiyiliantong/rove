import { test } from 'node:test';
import assert from 'node:assert/strict';
import { loadIntroduction, saveIntroduction, INTRO_KEY } from '../src/introduction.ts';
function memory() { const values = new Map(); return { values, getItem: key => values.get(key) ?? null, setItem: (key, value) => values.set(key, value) }; }
test('first use, progress and completion survive reload', () => {
  const storage = memory(); const state = loadIntroduction(storage);
  assert.deepEqual(state, {version:1,completed:false,step:0,resetPending:false});
  state.step = 2; assert.equal(saveIntroduction(storage,state),true); assert.equal(loadIntroduction(storage).step,2);
  state.completed=true; saveIntroduction(storage,state); assert.equal(loadIntroduction(storage).completed,true);
});
test('reset is consumed at launch and touches no application snapshot', () => {
  const storage = memory(); storage.setItem('rove-prototype-v1','untouched-business-data');
  const state = {version:1,completed:true,step:3,resetPending:false};
  saveIntroduction(storage,{...state,resetPending:true}); assert.equal(state.completed,true);
  assert.deepEqual(loadIntroduction(storage),{version:1,completed:false,step:0,resetPending:false});
  assert.equal(storage.getItem('rove-prototype-v1'),'untouched-business-data');
  assert.equal(JSON.parse(storage.getItem(INTRO_KEY)).resetPending,false);
});
test('corrupt preferences and unavailable storage remain usable without false success', () => {
  const storage=memory(); storage.setItem(INTRO_KEY,'{'); assert.equal(loadIntroduction(storage).step,0);
  storage.setItem(INTRO_KEY,JSON.stringify({version:1,step:99})); assert.equal(loadIntroduction(storage).step,3);
  assert.equal(saveIntroduction(undefined,loadIntroduction()),false);
  assert.equal(saveIntroduction({getItem:()=>null,setItem:()=>{throw Error('denied');}},loadIntroduction()),false);
});
