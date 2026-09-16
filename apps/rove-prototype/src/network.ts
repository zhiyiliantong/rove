import type { Network, NetworkCard, PeerProbe } from './domain.ts';

// DHCP is an address mode, not a configurable pool in upstream EasyTier 2.6.4.
export const DHCP = 'dhcp';
export function generateNetworkKey(): string {
  return Array.from(globalThis.crypto.getRandomValues(new Uint8Array(32)), b => b.toString(16).padStart(2, '0')).join('');
}
export function cidrRange(value: string) {
  // Also accept the user's three-octet shorthand, e.g. 192.168.100/24.
  const match = value.trim().match(/^(\d{1,3})\.(\d{1,3})\.(\d{1,3})(?:\.(\d{1,3}))?\/(\d{1,2})$/);
  if (!match || match.slice(1, 5).some(n => n !== undefined && +n > 255) || +match[5]! > 32) throw new Error('虚拟网段格式应为 IPv4 CIDR，例如 192.168.100.0/24。');
  const prefix = +match[5]!;
  const address = match.slice(1, 5).reduce((a, n) => a * 256 + +(n ?? 0), 0);
  const size = 2 ** (32 - prefix);
  const start = Math.floor(address / size) * size;
  const network = [24, 16, 8, 0].map(shift => (start >>> shift) & 255).join('.');
  return { start, end: start + size - 1, cidr: `${network}/${prefix}` };
}
export function subnetsOverlap(a: string, b: string): boolean {
  const left = cidrRange(a), right = cidrRange(b);
  return left.start <= right.end && right.start <= left.end;
}
export function validatePeer(value: string) {
  try {
    if (/\s/.test(value)) throw new Error();
    const u = new URL(value);
    if (!['tcp:', 'udp:', 'ws:', 'wss:', 'quic:', 'wg:'].includes(u.protocol) || !u.hostname || u.username || u.password || u.hash || u.search) throw new Error();
    if (!['ws:', 'wss:'].includes(u.protocol) && (!u.port || +u.port < 1 || +u.port > 65535 || (u.pathname && u.pathname !== '/'))) throw new Error();
  } catch { throw new Error('初始节点需要有效协议、主机和端口，例如 tcp://relay.example.invalid:11010。'); }
}
export function validateNetworkCard(card: NetworkCard) {
  if (!card.name.trim() || !card.network_key.trim() || !card.initial_peers.length) throw new Error('请填写网络名称、网络密钥和至少一个初始节点。');
  if (card.subnet !== DHCP) cidrRange(card.subnet);
  card.initial_peers.forEach(validatePeer);
}
export function subnetLabel(network: Network): string {
  return network.subnet === DHCP ? `DHCP · ${network.resolved_subnet ?? '网段待获取'}` : network.subnet;
}
export function networkStatus(network: Network): string {
  return { disconnected: '已保存，未连接', waiting_dhcp: '等待 DHCP 分配（模拟）', connected: '已连接（模拟）', conflict: '网段冲突，未连接' }[network.connection_status];
}
export function mockPeerProbe(peer: string, offline = false): PeerProbe {
  try { validatePeer(peer); } catch (e) { return { peer, status: 'invalid', latency_ms: null, simulated: true, message: (e as Error).message }; }
  if (offline || new URL(peer).hostname === 'timeout.example.invalid') return { peer, status: 'timed_out', latency_ms: null, simulated: true, message: '模拟超时；未发送网络请求。' };
  return { peer, status: 'reachable', latency_ms: 32, simulated: true, message: '模拟可达 · 32 ms；不代表真实连通或入网认证成功。' };
}
