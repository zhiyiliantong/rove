import {test} from 'node:test';
import assert from 'node:assert/strict';
import {parse_config_file,CONFIG_FILE_LIMIT} from '../src/network-import.ts';
test('network files populate URL or JSON without executing an import',()=>{
  assert.deepEqual(parse_config_file('\uFEFF {"schema_version":1}\n'),{config:{schema_version:1}});
  const url='https://example.invalid/c/test#key=test-only';
  assert.deepEqual(parse_config_file(`  ${url}\n`),{url});
  for(const invalid of ['javascript:alert(1)','file:///tmp/config','https://example.invalid/no-key','https://user:secret@example.invalid/#key=x','{invalid','[]','x'.repeat(CONFIG_FILE_LIMIT+1)])assert.throws(()=>parse_config_file(invalid));
});
