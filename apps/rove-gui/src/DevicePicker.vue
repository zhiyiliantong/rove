<script setup lang="ts">
import {t,notice_text,state_label} from './i18n';
import {nextTick,onBeforeUnmount,onMounted,ref,shallowRef} from 'vue';
import {call,type Target} from './api';
import {peer_state,peer_target,type Peer} from './peer-state';
interface Network {network_id:string;display_name:string;state:string}
interface Page<T> {items:T[];next_cursor:string|null}
const props=defineProps<{disabled:boolean;networkId?:string;target?:Target;readOnly?:boolean;scopedView?:boolean}>();
const emit=defineEmits<{choose:[target:Target]}>();
const networks=shallowRef<Network[]>([]),peers=shallowRef<Peer[]>([]);
const network_id=ref(props.networkId??''),network_cursor=ref<string|null>(null),peer_cursor=ref<string|null>(null);
const busy=ref(false),error=ref('');
interface Alias extends Target {alias:string}
const aliases=shallowRef<Alias[]>([]),editing=ref(''),alias_draft=ref('');
function alias_for(peer:Peer){return aliases.value.find(a=>a.network_id===peer.network_id&&a.device_id===peer.device_id)?.alias??'';}
async function edit_alias(peer:Peer){
  if(busy.value||props.disabled)return;
  editing.value=peer.device_id;alias_draft.value=alias_for(peer)||peer.display_name;error.value='';
  await nextTick();const input=document.getElementById(`alias-${peer.device_id}`) as HTMLInputElement|null;input?.focus();input?.select();
}
function alias_keydown(event:KeyboardEvent,peer:Peer){
  if(event.isComposing||event.keyCode===229)return;
  if(event.key==='Escape'){event.preventDefault();if(!busy.value){editing.value='';error.value='';}}
  if(event.key==='Enter'){event.preventDefault();void save_alias(peer);}
}
async function save_alias(peer:Peer){
  if(editing.value!==peer.device_id||busy.value||props.disabled)return;
  if(alias_draft.value.trim()===(alias_for(peer)||peer.display_name)){editing.value='';return;}
  const current=generation,id=network_id.value;busy.value=true;error.value='';
  try{
    // Always local: selecting a remote management target must not rename its address book.
    const saved=await call('set_device_alias',{network_id:peer.network_id,device_id:peer.device_id,alias:alias_draft.value.trim()||null}) as unknown as Alias;
    if(!alive||current!==generation||id!==network_id.value)return;
    aliases.value=aliases.value.filter(a=>!(a.network_id===peer.network_id&&a.device_id===peer.device_id));
    if(saved.alias)aliases.value=[...aliases.value,saved];
    editing.value='';
  }catch(e){if(alive&&current===generation)error.value=String(e).includes('device_alias_conflict')?t('device_alias.conflict'):notice_text(String(e));}
  finally{if(alive&&current===generation)busy.value=false;}
}
let generation=0,alive=true;
async function refresh_networks(append=false){
  if(busy.value||props.disabled)return;
  const current=++generation;busy.value=true;error.value='';
  try{
    // Discovery always belongs to the client's local gateway, not its current
    // execution target. Never send an old target to these requests.
    const page=await call('list_networks',undefined,undefined,append&&network_cursor.value?{cursor:network_cursor.value}:{}) as unknown as Page<Network>;
    if(!alive||current!==generation)return;
    networks.value=append?[...networks.value,...page.items]:page.items;network_cursor.value=page.next_cursor;
    if(!append){network_id.value='';peers.value=[];peer_cursor.value=null;}
  }catch(e){if(alive&&current===generation)error.value=e instanceof Error?e.message:t('ui.failed_to_read_local_networks');}
  finally{if(alive&&current===generation)busy.value=false;}
}
async function refresh_peers(append=false){
  if(busy.value||props.disabled||!network_id.value)return;
  const id=network_id.value,current=++generation;busy.value=true;error.value='';
  if(!append){peers.value=[];peer_cursor.value=null;}
  try{
    const page=await call('list_network_devices',undefined,{network_id:id},append&&peer_cursor.value?{cursor:peer_cursor.value}:{},props.target) as unknown as Page<Peer>;
    const names=await call('list_device_aliases') as unknown as {items:Alias[]};
    if(!alive||current!==generation||id!==network_id.value)return;
    // A response cannot change the selected network by returning another ID.
    if(page.items.some(peer=>peer.network_id!==id))throw new Error(t('ui.device_list_network_context_does_not_match'));
    aliases.value=names.items;
    peers.value=append?[...peers.value,...page.items]:page.items;peer_cursor.value=page.next_cursor;
  }catch(e){if(alive&&current===generation)error.value=e instanceof Error?e.message:t('ui.device_discovery_failed');}
  finally{if(alive&&current===generation)busy.value=false;}
}
function choose(peer:Peer){
  if(busy.value||props.disabled)return;
  try{emit('choose',peer_target(peer,network_id.value));}catch(e){error.value=e instanceof Error?e.message:t('ui.device_cannot_be_selected');}
}
function change_network(){generation++;peers.value=[];peer_cursor.value=null;error.value='';void refresh_peers();}
onMounted(()=>props.networkId?refresh_peers():refresh_networks());
onBeforeUnmount(()=>{alive=false;generation++;});
</script>
<template>
  <div class="picker">
    <div class="device-heading"><h3>{{t(scopedView?'ui.devices':'ui.choose_a_device_from_locally_joined_networks')}}</h3><button v-if="scopedView" class="refresh-devices" :disabled="busy||disabled||!network_id" :aria-label="t('ui.refresh_devices')" :title="t('ui.refresh_devices')" @click="refresh_peers()"><i class="pi pi-refresh" aria-hidden="true"/></button></div>
    <p v-if="error" class="error" role="alert">{{notice_text(error)}}</p>
    <div v-if="!networkId" class="actions"><button :disabled="busy||disabled" @click="refresh_networks()">{{t('ui.refresh_local_networks')}}</button><button v-if="network_cursor" :disabled="busy||disabled" @click="refresh_networks(true)">{{t('ui.more_networks')}}</button></div>
    <label v-if="!networkId">{{t('ui.networks')}}<select v-model="network_id" :disabled="busy||disabled" @change="change_network"><option value="">{{t('ui.select_a_network')}}</option><option v-for="network in networks" :key="network.network_id" :value="network.network_id">{{network.display_name}} · {{state_label(network.state)}}</option></select></label>
    <button v-if="!scopedView" :disabled="busy||disabled||!network_id" @click="refresh_peers()">{{t('ui.refresh_devices')}}</button>
    <p v-if="network_id&&!busy&&!peers.length" class="muted">{{t('ui.no_rove_devices_were_discovered_check_that_the')}}</p>
    <article v-for="peer in peers" :key="peer.device_id"><div><div class="alias-title">
      <input v-if="editing===peer.device_id" :id="`alias-${peer.device_id}`" v-model="alias_draft" class="alias-input" maxlength="64" :readonly="busy" :aria-busy="busy" :aria-invalid="!!error" :aria-label="t('device_alias.label')" enterkeyhint="done" @keydown="alias_keydown($event,peer)" @blur="save_alias(peer)"/>
      <button v-else-if="scopedView" class="alias-name" :disabled="busy||disabled" :aria-label="t('device_alias.edit')" :title="t('device_alias.hint')" @click="edit_alias(peer)">{{alias_for(peer)||peer.display_name}}</button>
      <strong v-else>{{alias_for(peer)||peer.display_name}}</strong>
    </div>
      <p>{{peer.os}} / {{peer.arch}} · {{peer_state(peer)}}</p><small>{{peer.device_id}}</small><p v-if="peer.overlay_addresses?.length">{{peer.overlay_addresses.join(', ')}}</p><p v-if="peer.last_error" class="error">{{notice_text(peer.last_error.code+': '+peer.last_error.message)}}</p></div><button v-if="!readOnly&&!scopedView" :disabled="busy||disabled||peer.state!=='online'" @click="choose(peer)">{{t('ui.select_device')}}</button></article>
    <button v-if="peer_cursor" :disabled="busy||disabled" @click="refresh_peers(true)">{{t('ui.more_devices')}}</button>
  </div>
</template>
<style scoped>
.alias-title{min-height:44px;display:flex;align-items:center;overflow-wrap:anywhere}.alias-title strong{min-width:0}.alias-title .alias-name{font:inherit;font-weight:700;text-align:left;color:inherit;background:transparent;border:0;box-shadow:none;padding:0;min-height:44px;min-width:44px;max-width:100%;white-space:normal;overflow-wrap:anywhere}.alias-title .alias-input{display:block;width:100%;box-sizing:border-box;min-width:0;min-height:44px;padding:4px 6px;font:inherit;font-weight:700}
.picker{margin-bottom:20px}.device-heading{display:flex;align-items:center;justify-content:space-between;gap:12px}button.refresh-devices{width:44px;height:44px;padding:0;background:transparent;border:0}.picker article{flex-wrap:wrap}.picker article>div{flex:1;min-width:160px}.picker article>button{min-height:44px;align-self:center;flex-shrink:0}.actions{display:flex;gap:8px;margin:12px 0}select{padding:10px;margin:8px;border:1px solid #ccd7ca;border-radius:7px;max-width:100%}small{overflow-wrap:anywhere}article{display:flex;justify-content:space-between;gap:12px}
@media(max-width:600px){.picker article>div{flex-basis:100%}.picker article>button{align-self:flex-start}}
</style>
