import {t} from './i18n.ts';
import type {Target} from './api';

export interface Peer {
  device_id:string; network_id:string; display_name:string; os:string; arch:string;
  state:'online'|'offline'|'incompatible'; capabilities:string[];
  overlay_addresses:string[]; last_error:null|{code:string;message:string};
}
export function peer_state(peer:Peer):string {
  if(peer.state==='offline')return t('ui.offline_target_unreachable_execution_will_not_fall_back');
  if(peer.state==='incompatible')return t('ui.protocol_incompatible_upgrade_this_client_or_the_target');
  if(peer.state!=='online')return t('ui.unknown_device_status_cannot_select_yet');
  return peer.capabilities.includes('system_exec')?t('ui.online'):t('ui.online_local_commands_unavailable_unsupported');
}
export function peer_target(peer:Peer,network_id:string):Target {
  const uuid=/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
  if(peer.state!=='online'||peer.network_id!==network_id||!uuid.test(network_id)||!uuid.test(peer.device_id))throw new Error(t('ui.device_status_or_network_context_changed_refresh_and'));
  return {network_id,device_id:peer.device_id};
}
