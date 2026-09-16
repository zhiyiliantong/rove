import {test} from 'node:test';
import assert from 'node:assert/strict';
import {browser_address,can_open} from '../src/service-address.ts';
test('web service addresses support IPv4, IPv6 and app paths',()=>{
  assert.equal(browser_address('http://10.20.30.4:43210'),'http://10.20.30.4:43210/');
  assert.equal(browser_address('https://[fd00::1]:8443/music?q=album'),'https://[fd00::1]:8443/music?q=album');
});
test('non-web handlers, embedded credentials and malformed addresses stay closed',()=>{
  for(const address of ['tcp://10.20.30.4:8000','file:///tmp/a','javascript:alert(1)','mailto:a@example.com','https://user:secret@example.com/','/relative','not a url']){
    assert.equal(can_open(address),false,address);
    assert.throws(()=>browser_address(address));
  }
});
