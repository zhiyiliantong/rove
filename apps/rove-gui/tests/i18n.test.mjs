import test from 'node:test';
import assert from 'node:assert/strict';
import en from '../src/locales/en.ts';
import zh from '../src/locales/zh-CN.ts';
import {resolve_locale,set_locale,t,format_datetime,format_number,notice_text,state_label} from '../src/i18n.ts';
import {renderRichText} from '../src/rich-text.ts';
import {readFileSync,readdirSync} from 'node:fs';
function flatten(value,prefix=''){return Object.fromEntries(Object.entries(value).flatMap(([key,v])=>typeof v==='string'?[[prefix+key,v]]:Object.entries(flatten(v,prefix+key+'.'))));}
test('Chinese and English resources have identical keys and interpolation parameters',()=>{
  const english=flatten(en),chinese=flatten(zh);
  assert.deepEqual(Object.keys(english).sort(),Object.keys(chinese).sort());
  for(const key of Object.keys(english)){
    const params=s=>[...s.matchAll(/\{(\w+)\}/g)].map(m=>m[1]).sort();
    assert.deepEqual(params(english[key]),params(chinese[key]),key);
    for(const locale of ['zh-CN','en']){set_locale(locale);assert.notEqual(t(key,{p0:'value',p1:'',label:'label',index:1,text:'text'}),key,key);}
  }
});
test('language preference overrides supported system languages and defaults to English',()=>{
  assert.equal(resolve_locale('en',['zh-CN']),'en');assert.equal(resolve_locale('zh-CN',['en-US']),'zh-CN');
  for(const language of ['zh','zh-TW','zh-Hans','ZH-hk'])assert.equal(resolve_locale('system',[language]),'zh-CN');
  assert.equal(resolve_locale(null,['fr-FR','en-GB']),'en');assert.equal(resolve_locale('invalid',['zh-CN']),'zh-CN');
  assert.equal(resolve_locale(null,[]),'en');assert.equal(resolve_locale('system',['fr-FR']),'en');
});
test('dates, numbers and controlled errors change without translating user content',()=>{
  set_locale('en');const english=format_datetime('2026-09-17T08:00:00Z');assert.equal(format_number(1234),'1,234');
  assert.match(notice_text('model_not_configured: diagnostic'),/Configure a model/);assert.equal(state_label('running'),'Running');
  set_locale('zh-CN');assert.notEqual(format_datetime('2026-09-17T08:00:00Z'),english);assert.match(notice_text('model_not_configured: diagnostic'),/配置模型/);assert.equal(state_label('running'),'运行中');
  assert.equal(notice_text('custom diagnostic'),'custom diagnostic');assert.equal(format_datetime('not-a-date'),'not-a-date');
});
test('rich text localizes only controls and keeps exact copy sources and safety',()=>{
  const source='自定义标题 / Keep this text\n\n```sh\nprintf user-data\n```\n\n![用户图片](https://example.invalid/image.png)';
  set_locale('zh-CN');const chinese=renderRichText(source);set_locale('en');const english=renderRichText(source);
  assert.deepEqual(chinese.copies,english.copies);assert.match(english.html,/Copy code/);assert.match(english.html,/自定义标题 \/ Keep this text/);
  assert.match(english.html,/external images are not loaded automatically/);assert.doesNotMatch(english.html,/<img/);
});
test('Vue pages keep Chinese interface copy in locale resources',()=>{
  const root=new URL('../src/',import.meta.url);
  for(const file of readdirSync(root,{recursive:true}).filter(f=>f.endsWith('.vue'))){
    // Language names intentionally remain readable in their own language.
    const source=readFileSync(new URL(file,root),'utf8').replaceAll('简体中文','');
    assert.doesNotMatch(source,/[\u3400-\u9fff]/,file);
  }
});
