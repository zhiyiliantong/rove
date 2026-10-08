import {test} from 'node:test';
import assert from 'node:assert/strict';
import {decode_network_config,encode_network_config,parse_config_file,CONFIG_FILE_LIMIT} from '../src/network-import.ts';
test('network files round-trip a bounded base64url JoinConfig without accepting URLs or raw JSON',()=>{
  const config={schema_version:1,display_name:'家庭网络',easytier:{network_secret:'test-only'}};
  const encoded=encode_network_config(config);
  assert.match(encoded,/^[-_A-Za-z0-9]+$/);
  assert.deepEqual(decode_network_config(` \uFEFF${encoded}\n`),config);
  assert.deepEqual(parse_config_file(encoded),{config});
  for(const invalid of ['https://example.invalid/c/test#key=test-only','{"schema_version":1}','[]','%%%','A','x'.repeat(CONFIG_FILE_LIMIT*2+1)])assert.throws(()=>parse_config_file(invalid));
});
