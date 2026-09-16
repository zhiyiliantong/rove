import type { Snapshot, Scenario, PrototypeApi, ModelImport, NetworkCard, Connection, Network } from './domain.ts';
import { demoId } from './ids.ts';
import { generatedModelNames, presetCatalog } from './model-catalog.ts';
import { providers, providerConfig } from './providers.ts';
import { replyPreview } from './reply-preview.ts';
import { DHCP, cidrRange, subnetsOverlap, validateNetworkCard, mockPeerProbe } from './network.ts';

export class SessionError extends Error {
  readonly code: string;
  readonly status: number;
  constructor(code: string, status: number, message: string) { super(message); this.name = 'SessionError'; this.code = code; this.status = status; }
}
export const STORAGE_KEY = 'rove-prototype-v1';
const id = demoId;
const clone = <T>(value: T): T => JSON.parse(JSON.stringify(value));
export function seed(scenario: Scenario): Snapshot {
  const state: Snapshot = { version: 4, scenario, initialized: scenario !== 'empty', connections: [], models: [], default_model_id: null, networks: [], devices: [], services: [], sessions: [], runs: [], max_active_runs: 4 };
  if (scenario === 'empty') return state;
  state.connections = [{ id: 'connection-demo', name: '日常助手', provider: '兼容接口', base_url: 'https://api.example.invalid/v1', auth_kind: 'api_key', credential_ref: 'demo-only' }];
  state.models = [{ id: 'model-demo', connection_id: 'connection-demo', model: 'demo-chat', name: 'custom-demo-chat-01' }, { id: 'model-fast', connection_id: 'connection-demo', model: 'demo-fast', name: 'custom-demo-fast-01' }];
  state.default_model_id = 'model-demo';
  state.networks = [{ id: 'home', name: '我的漫游空间', subnet: '10.42.0.0/24', network_key: 'a1'.repeat(32), initial_peers: ['tcp://relay.example.invalid:11010'] }, { id: 'studio', name: '工作室', subnet: '10.43.0.0/24', network_key: 'b2'.repeat(32), initial_peers: ['tcp://relay.example.invalid:11010'] }].map(n => ({ ...n, connection_status: n.id === 'home' ? 'connected' : 'disconnected', resolved_subnet: n.id === 'home' ? n.subnet : null, local_ip: n.id === 'home' ? '10.42.0.2' : null, connection_error: null, connect_started_at: null }));
  state.devices = [
    { id: 'local', network_id: '', name: '此设备', os: '本机', ip: '本机', online: true, model_ids: ['model-demo', 'model-fast'] },
    { id: 'windows', network_id: 'home', name: '书房电脑', os: 'Windows', ip: '10.42.0.3', online: scenario !== 'offline', model_ids: ['model-demo'] },
    { id: 'linux', network_id: 'home', name: '家庭服务器', os: 'Linux', ip: '10.42.0.4', online: true, model_ids: ['model-demo'] },
    { id: 'tablet', network_id: 'home', name: '随身平板', os: 'Android', ip: '10.42.0.5', online: false, model_ids: [] },
  ].map(d => ({ ...d, model_copies: d.model_ids.map(mid => ({ source_model_id: mid, model: state.models.find(m => m.id === mid)!.model, connection_name: '日常助手', base_url: 'https://api.example.invalid/v1', credential_ref: 'demo-only' })) }));
  state.services = [
    { id: 'music', name: '家庭音乐库', device_id: 'linux', network_id: 'home', address: 'http://10.42.0.4:4533', reachable: true, kind: 'music' },
    { id: 'files', name: '我的文件', device_id: 'windows', network_id: 'home', address: 'http://10.42.0.3:8080', reachable: scenario !== 'offline', kind: 'files' },
    { id: 'code', name: '代码工作台', device_id: 'tablet', network_id: 'home', address: 'http://10.42.0.5:3000', reachable: false, kind: 'code' },
  ];
  state.sessions = [{ id: 'welcome', archived_at: null, title: '让设备为你协作', owner_device_id: 'local', model_id: 'model-demo', draft: '', target_device_id: 'local', onboarding: false, messages: [{ id: 'welcome-message', role: 'assistant', text: '你好，我是这台设备上的 rove-agent。想在书房电脑安装音乐播放器，还是看看家里的服务？告诉我就好。' }] }];
  return state;
}

export const validateCard = validateNetworkCard;
export function exportCard(card: NetworkCard) {
  return JSON.stringify({ format: 'rove-prototype-card-v1', network: { name: card.name, subnet: card.subnet, network_key: card.network_key, initial_peers: [...card.initial_peers] } });
}
export function cardUrl(card: NetworkCard) {
  return `https://rove.example.invalid/#demo-card=${encodeURIComponent(exportCard(card))}`;
}
export function parseCard(value: string): NetworkCard {
  try {
    const raw = value.startsWith('https://rove.example.invalid/#demo-card=') ? decodeURIComponent(value.split('#demo-card=')[1]!) : value;
    const parsed = JSON.parse(raw);
    if (parsed.format !== 'rove-prototype-card-v1') throw new Error();
    const n = parsed.network;
    if (!n || ['name', 'subnet', 'network_key'].some(k => typeof n[k] !== 'string') || !Array.isArray(n.initial_peers) || n.initial_peers.some((p: unknown) => typeof p !== 'string')) throw new Error();
    const card = { name: n.name, subnet: n.subnet, network_key: n.network_key, initial_peers: n.initial_peers };
    validateCard(card);
    return card;
  } catch { throw new Error('仅支持本原型导出的演示 URL 或 JSON 配置，不会导入真实网络凭据。'); }
}
export function validateEndpoint(url: string) {
  try { const u = new URL(url); if (!['http:', 'https:'].includes(u.protocol) || u.username || u.password || u.search || u.hash) throw new Error(); }
  catch { throw new Error('接口地址需要 http(s) 基础地址，不应包含账号、密钥、查询参数或片段。'); }
}

export function createMockApi(storage?: Pick<Storage, 'getItem' | 'setItem'>, now = () => Date.now()): PrototypeApi {
  let state = seed('daily');
  try { const saved = storage?.getItem(STORAGE_KEY); if (saved) { const s = JSON.parse(saved); if ([1, 2, 3, 4].includes(s.version) && Array.isArray(s.sessions) && Array.isArray(s.runs) && Array.isArray(s.models) && Array.isArray(s.connections) && Array.isArray(s.networks) && Array.isArray(s.devices) && Array.isArray(s.services)) {
    if (s.version === 1) {
      for (const n of s.networks) Object.assign(n, { connection_status: n.id === s.active_network_id ? 'connected' : 'disconnected', resolved_subnet: n.id === s.active_network_id ? cidrRange(n.subnet).cidr : null, local_ip: n.id === s.active_network_id ? '自动分配（旧演示）' : null, connection_error: null, connect_started_at: null });
      delete s.active_network_id; s.version = 2;
    }
    const names = s.models.map((m: { name?: string }) => m.name).filter(Boolean);
    for (const m of s.models) if (!m.name) { m.name = generatedModelNames(providerConfig(s.connections.find((c: Connection) => c.id === m.connection_id)?.provider ?? 'custom').id, [m.model], names)[0]; names.push(m.name); }
    for (const conversation of s.sessions) conversation.archived_at ??= null;
    s.version = 4; state = s;
  } } } catch { /* Invalid/unavailable storage falls back to demo fixtures. */ }
  for (const s of state.sessions) s.target_device_id ??= 'local';
  for (const r of state.runs) { r.service_id ??= null; r.service_name ??= null; }
  for (const d of state.devices) { d.model_copies ??= []; if (d.id === 'local') { d.network_id = ''; d.ip = '本机'; } }
  const connected = (nid: string | null) => state.networks.some(n => n.id === nid && n.connection_status === 'connected');
  function resolveNetwork(n: Network, subnet: string) {
    const cidr = cidrRange(subnet).cidr;
    const conflict = state.networks.find(other => other.id !== n.id && other.connection_status === 'connected' && other.resolved_subnet && subnetsOverlap(cidr, other.resolved_subnet));
    n.connect_started_at = null;
    if (conflict) {
      n.connection_status = 'conflict'; n.resolved_subnet = cidr; n.local_ip = null;
      n.connection_error = `${cidr} 与“${conflict.name}”的 ${conflict.resolved_subnet} 重叠，不能同时连接。其他已连接网络保持不变。`;
    } else {
      n.connection_status = 'connected'; n.resolved_subnet = cidr; n.connection_error = null;
      const range = cidrRange(cidr); const address = Math.min(range.start + 2, range.end);
      n.local_ip = [24, 16, 8, 0].map(shift => (address >>> shift) & 255).join('.');
      if (!state.devices.some(d => d.id === 'local')) state.devices.push({ id: 'local', network_id: '', name: '此设备', os: '本机', ip: '本机', online: true, model_ids: state.models.map(m => m.id), model_copies: [] });
    }
  }
  function save() { try { storage?.setItem(STORAGE_KEY, JSON.stringify(state)); } catch { /* App remains usable without browser persistence. */ } }
  const session = (sessionId: string) => { const s = state.sessions.find(s => s.id === sessionId); if (!s) throw new SessionError('session_not_found', 404, '会话不存在，请返回列表。'); return s; };
  const editableSession = (sid: string) => {
    const s = session(sid);
    if (s.archived_at !== null) throw new SessionError('session_archived', 409, '会话已归档，请先恢复再继续对话。');
    return s;
  };
  function addSession(onboarding = false) {
    const sid = id();
    state.sessions.unshift({ id: sid, archived_at: null, title: onboarding ? '初始化网络' : '新的会话', owner_device_id: 'local', model_id: state.default_model_id ?? '', draft: '', target_device_id: 'local', onboarding, messages: [{ id: id(), role: 'assistant', text: onboarding ? '模型已准备好。想加入已有网络，还是创建自己的漫游空间？' : '想让哪台设备做什么？我会在执行前显示目标设备。' }] });
    save(); return sid;
  }
  function advance() {
    const t = now();
    for (const n of state.networks) if (n.connection_status === 'waiting_dhcp' && n.connect_started_at !== null && t - n.connect_started_at >= 1800 && state.scenario !== 'offline') resolveNetwork(n, '10.126.126.0/24');
    for (const run of state.runs) {
      if (!['queued', 'running'].includes(run.status)) continue;
      const device = state.devices.find(d => d.id === run.device_id);
      run.waiting_connection = run.device_id !== 'local' && (!device?.online || !connected(run.network_id));
      if (run.waiting_connection) continue;
      // The timeline advances independently of the initiating view, including after reload.
      if (run.status === 'running') {
        const prompt = session(run.session_id).messages.find(m => m.role === 'user' && m.run_id === run.id)?.text ?? '';
        const preview = replyPreview(prompt);
        run.output_text = Array.from(preview).slice(0, Math.max(0, Math.floor((t - run.started_at!) / 35))).join('');
        run.progress = Math.min(100, Math.max(5, Math.floor((t - run.started_at!) / 120)));
        if (run.progress === 100) {
          run.status = run.outcome === 'success' ? 'succeeded' : 'failed';
          run.error = run.outcome === 'failure' ? '演示：软件源暂时不可用，请稍后重试。' : run.outcome === 'unsupported' ? 'unsupported：演示目标平台不支持此操作。' : null;
          let reply = '演示检查完成。已连接网络的服务：' + (state.services.filter(s => run.network_id ? s.network_id === run.network_id : connected(s.network_id)).map(s => `${s.name}（${s.reachable ? '可访问' : '不可达'}）`).join('、') || '暂无') + '。';
          if (run.status === 'succeeded') {
            if (run.intent === 'install_music' && !state.services.some(s => s.id === run.id)) { const address = state.devices.find(d => d.id === run.device_id)?.ip; state.services.push({ id: run.id, name: '新音乐服务', device_id: run.device_id, network_id: run.network_id ?? '', address: `http://${address && /^\d+\./.test(address) ? address : 'music.example.invalid'}:4533`, reachable: true, kind: 'music' }); reply = '演示任务已完成，音乐服务已出现在服务列表。没有实际安装软件。'; }
            if (run.intent === 'rename_service') { const service = state.services.find(s => s.id === run.service_id); if (service && run.service_name) { service.name = run.service_name; reply = `演示服务已改名为“${run.service_name}”。`; } else { run.status = 'failed'; run.error = '目标服务已不存在，请重新查询。'; } }
            if (run.intent === 'remove_service') { state.services = state.services.filter(s => s.id !== run.service_id); reply = '指定演示服务已移除，没有卸载任何真实软件。'; }
          }
          run.output_text = preview;
          session(run.session_id).messages.push({ id: id(), role: 'assistant', text: preview + '\n\n' + (run.error ?? reply), run_id: run.id });
        }
      }
    }
    for (const run of state.runs.filter(r => r.status === 'queued')) {
      if (run.waiting_connection) continue;
      const running = state.runs.filter(r => r.status === 'running' && r.device_id === run.device_id);
      if (running.length >= (run.device_id === 'local' ? state.max_active_runs : 4) || state.runs.some(r => r.status === 'running' && r.session_id === run.session_id)) continue;
      run.status = 'running'; run.started_at = t; run.progress = 5; run.output_text = '';
    }
    save();
  }
  return {
    snapshot() { advance(); return clone(state); },
    reset(scenario) { state = seed(scenario); save(); },
    listProviders() { return clone(providers); },
    listModelCatalog(input) {
      const catalog = presetCatalog(input.provider, input.auth_kind);
      if (!input.refresh) return catalog;
      if (input.auth_kind === 'official_agent' && !providerConfig(input.provider).account_adapter) throw new Error('此服务商尚未接入账号登录，预置列表不代表会员授权。');
      if (state.scenario === 'failure' || state.scenario === 'offline') throw new Error('模拟型号列表获取失败；已保留本地预置，可重试或手动填写。');
      return { ...catalog, source: input.auth_kind === 'api_key' ? 'mock_api' : 'mock_account', message: '模拟获取结果 · 未调用真实接口；不是账号实际可用型号清单。' };
    },
    importModels(input: ModelImport) {
      validateEndpoint(input.base_url);
      if (!input.name.trim() || !input.models.length) throw new Error('请填写配置名称并选择至少一个型号。');
      const types = [...new Set(input.models.map(m => m.trim()).filter(Boolean))];
      if (!types.length) throw new Error('型号不能为空。');
      const defaults = generatedModelNames(providerConfig(input.provider).id, types, state.models.map(m => m.name));
      const names = types.map((model, index) => Object.hasOwn(input.model_names ?? {}, model) ? input.model_names![model]!.trim() : defaults[index]!);
      if (names.some(name => !name)) throw new Error('模型名称不能为空。');
      if (new Set(names).size !== names.length || names.some(name => state.models.some(m => m.name === name))) throw new Error('模型名称重复，请修改名称或恢复自动名称。');
      const cid = id();
      state.connections.push({ id: cid, name: input.name.trim(), provider: input.provider, base_url: input.base_url, auth_kind: input.auth_kind, credential_ref: 'demo-only' });
      const models = types.map((model, i) => ({ id: id(), connection_id: cid, model, name: names[i]! }));
      state.models.push(...models);
      state.default_model_id ??= models[0]!.id;
      if (!state.initialized) { state.initialized = true; addSession(true); }
      save(); return models.map(m => m.id);
    },
    editConnection(cid, patch: Pick<Connection, 'name' | 'base_url'>) { validateEndpoint(patch.base_url); if (!patch.name.trim()) throw new Error('名称不能为空。'); const c = state.connections.find(c => c.id === cid); if (!c) throw new Error('连接不存在。'); Object.assign(c, patch); save(); },
    deleteModel(mid) { state.models = state.models.filter(m => m.id !== mid); state.connections = state.connections.filter(c => state.models.some(m => m.connection_id === c.id)); if (state.default_model_id === mid) state.default_model_id = state.models[0]?.id ?? null; for (const s of state.sessions) if (s.model_id === mid) s.model_id = state.default_model_id ?? ''; for (const d of state.devices.filter(d => d.id === 'local')) { d.model_ids = d.model_ids.filter(m => m !== mid); d.model_copies = d.model_copies.filter(m => m.source_model_id !== mid); } save(); },
    setDefaultModel(mid) { if (!state.models.some(m => m.id === mid)) throw new Error('模型不存在。'); state.default_model_id = mid; save(); },
    createSession() { return addSession(); },
    archiveSession(sid) { const s = session(sid); if (s.archived_at === null) { s.archived_at = now(); save(); } },
    restoreSession(sid) { const s = session(sid); if (s.archived_at !== null) { s.archived_at = null; save(); } },
    deleteSession(sid) {
      const s = session(sid);
      if (s.archived_at === null) throw new SessionError('session_not_archived', 409, '请先归档会话，再到归档管理删除。');
      if (state.runs.some(r => r.session_id === sid && ['queued', 'running', 'cancelling'].includes(r.status))) throw new SessionError('session_has_active_runs', 409, '会话仍有进行中的任务，请等待完成，或恢复会话后取消任务再删除。');
      state.sessions = state.sessions.filter(s => s.id !== sid);
      state.runs = state.runs.filter(r => r.session_id !== sid);
      save();
    },
    setSessionModel(sid, mid) { editableSession(sid); if (!state.models.some(m => m.id === mid)) throw new Error('模型不存在。'); editableSession(sid).model_id = mid; save(); },
    saveDraft(sid, draft) { editableSession(sid).draft = draft; save(); },
    setSessionTarget(sid, did) { editableSession(sid); if (did !== 'local' && !state.devices.some(d => d.id === did)) throw new Error('设备不存在。'); editableSession(sid).target_device_id = did; save(); },
    submit(sid, text, deviceId) {
      const s = editableSession(sid); advance();
      if (!text.trim()) throw new Error('请先输入消息。');
      if (!state.models.some(m => m.id === s.model_id)) throw new Error('请先添加并选择模型。');
      const device = state.devices.find(d => d.id === deviceId);
      if (deviceId !== 'local' && (!device?.online || !connected(device.network_id))) throw new Error('目标设备不可达，任务尚未提交；草稿已保留。');
      const service = state.services.find(v => text.includes(v.name) && v.device_id === deviceId && (connected(v.network_id) || (deviceId === 'local' && !v.network_id)));
      const remove = /删除|移除|卸载/.test(text);
      const rename = text.match(/改名为[“"「]?([^”"」\n]+)[”"」]?/);
      if ((remove || rename) && !service) throw new Error('请在消息中写出目标设备上的完整服务名称。演示支持“删除家庭音乐库”或“将家庭音乐库改名为我的音乐”。');
      const intent = remove ? 'remove_service' : rename ? 'rename_service' : /安装|部署/.test(text) && /音乐/.test(text) ? 'install_music' : 'query';
      const rid = id();
      s.messages.push({ id: id(), role: 'user', text, run_id: rid }); s.draft = '';
      if (s.title === '新的会话') s.title = text.slice(0, 24);
      state.runs.push({ id: rid, session_id: sid, device_id: deviceId, network_id: deviceId === 'local' ? null : device!.network_id, model_id: s.model_id, status: 'queued', submitted_at: now(), started_at: null, progress: 0, outcome: state.scenario === 'failure' ? 'failure' : state.scenario === 'unsupported' ? 'unsupported' : 'success', waiting_connection: false, intent, service_id: service?.id ?? null, service_name: rename?.[1]?.trim() ?? null, error: null });
      save(); return rid;
    },
    cancelRun(rid) { const r = state.runs.find(r => r.id === rid); if (r && ['running', 'queued'].includes(r.status)) { r.status = 'cancelled'; save(); } },
    saveNetwork(card, nid) { validateCard(card); const normalized = { ...clone(card), name: card.name.trim(), subnet: card.subnet === DHCP ? DHCP : cidrRange(card.subnet).cidr }; if (nid) { const n = state.networks.find(n => n.id === nid); if (!n) throw new Error('网络不存在。'); if (['connected', 'waiting_dhcp'].includes(n.connection_status)) throw new Error('请先断开此网络，再修改配置；其他网络不受影响。'); Object.assign(n, normalized, { connection_status: 'disconnected', resolved_subnet: null, connection_error: null }); } else { nid = id(); state.networks.push({ id: nid, ...normalized, connection_status: 'disconnected', resolved_subnet: null, local_ip: null, connection_error: null, connect_started_at: null }); } save(); return nid; },
    connectNetwork(nid) { advance(); const n = state.networks.find(n => n.id === nid); if (!n) throw new Error('网络不存在。'); if (['connected', 'waiting_dhcp'].includes(n.connection_status)) return; n.connection_error = null; n.resolved_subnet = null; if (n.subnet === DHCP) { n.connection_status = 'waiting_dhcp'; n.connect_started_at = now(); } else resolveNetwork(n, n.subnet); save(); if (n.connection_error) throw new Error(n.connection_error); },
    disconnectNetwork(nid) { const n = state.networks.find(n => n.id === nid); if (!n) throw new Error('网络不存在。'); Object.assign(n, { connection_status: 'disconnected', resolved_subnet: null, local_ip: null, connect_started_at: null, connection_error: null }); save(); },
    async probePeer(peer) { await new Promise(resolve => setTimeout(resolve, 350)); return mockPeerProbe(peer, state.scenario === 'offline'); },
    deleteNetwork(nid) { if (state.networks.some(n => n.id === nid && ['connected', 'waiting_dhcp'].includes(n.connection_status))) throw new Error('请先断开网络，再删除保存的配置。'); state.networks = state.networks.filter(n => n.id !== nid); state.devices = state.devices.filter(d => d.network_id !== nid || d.id === 'local'); state.services = state.services.filter(s => s.network_id !== nid); save(); },
    syncModels(did, mids) { const d = state.devices.find(d => d.id === did); if (!d?.online || (d.id !== 'local' && !connected(d.network_id))) throw new Error('设备不可达，未同步。'); if (!mids.length || mids.some(mid => !state.models.some(m => m.id === mid))) throw new Error('请选择有效模型。'); if (mids.some(mid => state.connections.find(c => c.id === state.models.find(m => m.id === mid)?.connection_id)?.auth_kind === 'official_agent')) throw new Error('官方账号登录型号需要在目标设备通过其适配器认证，本原型不复制账号 session。'); d.model_ids = [...new Set([...d.model_ids, ...mids])]; for (const mid of mids) { const m = state.models.find(m => m.id === mid)!; const c = state.connections.find(c => c.id === m.connection_id)!; d.model_copies = d.model_copies.filter(copy => copy.source_model_id !== mid); d.model_copies.push({ source_model_id: mid, model: m.model, connection_name: c.name, base_url: c.base_url, credential_ref: 'demo-only' }); } save(); },
    setConcurrency(value) { if (!Number.isInteger(value) || value < 1 || value > 16) throw new Error('并发数应为 1–16 的整数。'); state.max_active_runs = value; save(); },
  };
}
