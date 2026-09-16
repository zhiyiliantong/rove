import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import Ajv from 'ajv/dist/2020.js';
import { createMockApi, seed, exportCard, parseCard } from '../src/mock.ts';
import { cidrRange, subnetsOverlap, generateNetworkKey, validatePeer } from '../src/network.ts';
const contract = JSON.parse(readFileSync(new URL('../api/prototype.openapi.json', import.meta.url)));
const validate = new Ajv({ strict: false }).compile({ $ref: '#/components/schemas/Snapshot', components: contract.components });
function harness() { let time = 1000; let value = ''; const storage = { getItem: () => value, setItem: (_, v) => { value = v; } }; return { api: createMockApi(storage, () => time), storage, clock: () => time, advance: ms => { time += ms; } }; }
const card = (name, subnet) => ({ name, subnet, network_key: 'arbitrary-easytier-secret', initial_peers: ['tcp://relay.example.invalid:11010'] });
test('range comparison handles containment, adjacent networks, high bit, /0 and /32', () => {
  assert.equal(cidrRange('192.168.100/16').cidr, '192.168.0.0/16');
  assert.equal(cidrRange('192.168.100.52/24').cidr, '192.168.100.0/24');
  for (const [a,b] of [['192.168.100/24','192.168.100/16'],['0.0.0.0/0','255.255.255.255/32'],['192.168.0.1/32','192.168.0.0/24']]) { assert.ok(subnetsOverlap(a,b)); assert.ok(subnetsOverlap(b,a)); }
  assert.equal(subnetsOverlap('192.168.100/24','192.168.101/24'), false);
  assert.equal(subnetsOverlap('10.0.0.1/32','10.0.0.2/32'), false);
  for (const bad of ['999.0.0.0/24','10.0.0.0/33','10.0.0.0/-1','10.0/24','abc']) assert.throws(() => cidrRange(bad));
});
test('network secret uses Web Crypto on insecure LAN HTTP without randomUUID', () => {
  const saved = Object.getOwnPropertyDescriptor(globalThis, 'crypto');
  Object.defineProperty(globalThis, 'crypto', { configurable: true, value: { getRandomValues: array => { array.fill(255); return array; } } });
  try { assert.equal(generateNetworkKey(), 'ff'.repeat(32)); } finally { Object.defineProperty(globalThis, 'crypto', saved); }
  const keys = new Set(Array.from({length:100}, generateNetworkKey)); assert.equal(keys.size,100); keys.forEach(k => assert.match(k,/^[a-f0-9]{64}$/));
});
test('non-overlapping joins coexist; conflicts and edits never alter existing connection', () => {
  const h = harness(); h.api.connectNetwork('studio');
  const nid = h.api.saveNetwork(card('nested', '10.42.0/16'));
  assert.throws(() => h.api.connectNetwork(nid), /重叠/);
  let s = h.api.snapshot(); assert.equal(s.networks.filter(n=>n.connection_status==='connected').length,2);
  assert.equal(s.networks.find(n=>n.id===nid).connection_status,'conflict');
  assert.throws(() => h.api.saveNetwork(card('changed','10.99.0.0/24'),'home'), /先断开/);
  h.api.disconnectNetwork('studio'); s = h.api.snapshot(); assert.equal(s.networks.find(n=>n.id==='home').connection_status,'connected');
  assert.ok(validate(s),JSON.stringify(validate.errors));
});
test('DHCP is pending before resolution; simultaneous DHCP collision rejects only later network', () => {
  const h = harness(); const a = h.api.saveNetwork(card('auto-a','dhcp')); const b = h.api.saveNetwork(card('auto-b','dhcp'));
  h.api.connectNetwork(a); h.api.connectNetwork(b);
  let s = h.api.snapshot(); assert.equal(s.networks.find(n=>n.id===a).connection_status,'waiting_dhcp'); assert.equal(s.networks.find(n=>n.id===a).resolved_subnet,null);
  h.advance(2000); s = createMockApi(h.storage,h.clock).snapshot();
  assert.equal(s.networks.find(n=>n.id===a).connection_status,'connected'); assert.equal(s.networks.find(n=>n.id===b).connection_status,'conflict'); assert.equal(s.networks.find(n=>n.id==='home').connection_status,'connected');
  assert.equal(s.devices.filter(d=>d.id==='local').length,1); assert.ok(validate(s),JSON.stringify(validate.errors));
});
test('offline DHCP waits and can be cancelled without connecting on late resolution', () => {
  const h = harness(); h.api.reset('offline'); const a=h.api.saveNetwork(card('auto','dhcp')); h.api.connectNetwork(a); h.advance(50000);
  assert.equal(h.api.snapshot().networks.find(n=>n.id===a).connection_status,'waiting_dhcp'); h.api.disconnectNetwork(a); h.advance(50000);
  assert.equal(h.api.snapshot().networks.find(n=>n.id===a).connection_status,'disconnected');
});
test('v1 migration retains conversations and single connection without duplicate local identity', () => {
  const old = seed('daily'); old.version=1; old.active_network_id='home'; old.networks=old.networks.map(({id,name,subnet,network_key,initial_peers})=>({id,name,subnet,network_key,initial_peers}));
  old.sessions[0].draft='保留用户草稿'; old.devices[0].network_id='home';
  const api=createMockApi({getItem:()=>JSON.stringify(old),setItem:()=>{}}); const s=api.snapshot();
  assert.equal(s.version,4); assert.equal(s.sessions[0].draft,'保留用户草稿'); assert.equal('active_network_id' in s,false); assert.equal(s.devices[0].network_id,''); assert.ok(validate(s),JSON.stringify(validate.errors));
});
test('DHCP card and multiple peers round trip without leaking runtime addresses', () => {
  const c=card('auto','dhcp'); c.initial_peers.push('udp://relay.example.invalid:11010');
  assert.deepEqual(parseCard(exportCard(c)),c); assert.equal(Object.keys(JSON.parse(exportCard(c)).network).length,4);
});
test('peer probes return independently simulated success, timeout and invalid results', async () => {
  const api=createMockApi(); const outcomes=await Promise.all(['tcp://relay.example.invalid:11010','udp://timeout.example.invalid:11010','broken'].map(p=>api.probePeer(p)));
  assert.deepEqual(outcomes.map(p=>p.status),['reachable','timed_out','invalid']);
  const check=new Ajv({strict:false}).compile(contract.components.schemas.PeerProbe); outcomes.forEach(result=>assert.ok(check(result)));
  for(const bad of ['tcp://host:99999','tcp://host:0','tcp://host','tcp://u:password@host:80','https://host','udp://host:123?q=key']) assert.throws(()=>validatePeer(bad));
  validatePeer('wss://example.invalid/path'); validatePeer('tcp://[::1]:11010');
});
