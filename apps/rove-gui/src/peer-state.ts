import type {Target} from './api';

export interface Peer {
  device_id:string; network_id:string; display_name:string; os:string; arch:string;
  state:'online'|'offline'|'incompatible'; capabilities:string[];
  overlay_addresses:string[]; last_error:null|{code:string;message:string};
}
export function peer_state(peer:Peer):string {
  if(peer.state==='offline')return '离线：目标不可达，不会转到本机执行';
  if(peer.state==='incompatible')return '协议不兼容：需要升级客户端或目标 agent';
  if(peer.state!=='online')return '未知设备状态：暂不可选择';
  return peer.capabilities.includes('system_exec')?'在线':'在线 · 不提供本机命令（unsupported）';
}
export function peer_target(peer:Peer,network_id:string):Target {
  const uuid=/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
  if(peer.state!=='online'||peer.network_id!==network_id||!uuid.test(network_id)||!uuid.test(peer.device_id))throw new Error('设备状态或网络上下文已变化，请刷新后重试');
  return {network_id,device_id:peer.device_id};
}
