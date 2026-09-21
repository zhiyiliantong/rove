import {test} from 'node:test';
import assert from 'node:assert/strict';
import {network_secret,normalize_subnet,form_config} from '../src/network-config.ts';
test('network form separates DHCP and manual subnet and preserves legacy metadata',()=>{
  const secret=network_secret();assert.match(secret,/^[0-9a-f]{64}$/);assert.notEqual(secret,network_secret());
  assert.equal(normalize_subnet('192.168.100.20/16'),'192.168.0.0/16');
  for(const input of ['192.168.100/24','300.1.2.3/24','192.168.1.1/33','1.2.3.4/0'])assert.throws(()=>normalize_subnet(input));
  const automatic=form_config('home',secret,[' tcp://host:11010 ','tcp://host:11010'],true,'');
  assert.equal(automatic.dhcp,true);assert.equal(automatic.ipv4_cidr,undefined);assert.deepEqual(automatic.bootstrap_peers,['tcp://host:11010']);
  const manual=form_config('home',secret,[],false,'192.168.100.8/24');
  assert.equal(manual.dhcp,false);assert.equal(manual.ipv4_cidr,'192.168.100.0/24');assert.equal(manual.local_ipv4,undefined);
  const legacy=form_config('renamed',secret,[],true,'',{network_name:'old-identity',dhcp:true,ipv4_cidr:'10.1.2.0/24'});
  assert.equal(legacy.network_name,'old-identity');assert.equal(legacy.ipv4_cidr,'10.1.2.0/24');
});
