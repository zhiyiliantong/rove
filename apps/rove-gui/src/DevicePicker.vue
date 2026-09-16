<script setup lang="ts">
import {onBeforeUnmount,onMounted,ref,shallowRef} from 'vue';
import {call,type Target} from './api';
import {peer_state,peer_target,type Peer} from './peer-state';
interface Network {network_id:string;display_name:string;state:string}
interface Page<T> {items:T[];next_cursor:string|null}
const props=defineProps<{disabled:boolean}>();
const emit=defineEmits<{choose:[target:Target]}>();
const networks=shallowRef<Network[]>([]),peers=shallowRef<Peer[]>([]);
const network_id=ref(''),network_cursor=ref<string|null>(null),peer_cursor=ref<string|null>(null);
const busy=ref(false),error=ref('');
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
  }catch(e){if(alive&&current===generation)error.value=e instanceof Error?e.message:'本机网络读取失败';}
  finally{if(alive&&current===generation)busy.value=false;}
}
async function refresh_peers(append=false){
  if(busy.value||props.disabled||!network_id.value)return;
  const id=network_id.value,current=++generation;busy.value=true;error.value='';
  if(!append){peers.value=[];peer_cursor.value=null;}
  try{
    const page=await call('list_network_devices',undefined,{network_id:id},append&&peer_cursor.value?{cursor:peer_cursor.value}:{}) as unknown as Page<Peer>;
    if(!alive||current!==generation||id!==network_id.value)return;
    // A response cannot change the selected network by returning another ID.
    if(page.items.some(peer=>peer.network_id!==id))throw new Error('设备列表网络上下文不匹配');
    peers.value=append?[...peers.value,...page.items]:page.items;peer_cursor.value=page.next_cursor;
  }catch(e){if(alive&&current===generation)error.value=e instanceof Error?e.message:'设备发现失败';}
  finally{if(alive&&current===generation)busy.value=false;}
}
function choose(peer:Peer){
  if(busy.value||props.disabled)return;
  try{emit('choose',peer_target(peer,network_id.value));}catch(e){error.value=e instanceof Error?e.message:'设备不可选择';}
}
function change_network(){generation++;peers.value=[];peer_cursor.value=null;error.value='';void refresh_peers();}
onMounted(()=>refresh_networks());
onBeforeUnmount(()=>{alive=false;generation++;});
</script>
<template>
  <div class="picker">
    <h3>从本机已加入的网络选择设备</h3>
    <p v-if="error" class="error" role="alert">{{error}}</p>
    <div class="actions"><button :disabled="busy||disabled" @click="refresh_networks()">刷新本机网络</button><button v-if="network_cursor" :disabled="busy||disabled" @click="refresh_networks(true)">更多网络</button></div>
    <label>网络<select v-model="network_id" :disabled="busy||disabled" @change="change_network"><option value="">请选择网络</option><option v-for="network in networks" :key="network.network_id" :value="network.network_id">{{network.display_name}} · {{network.state}}</option></select></label>
    <button :disabled="busy||disabled||!network_id" @click="refresh_peers()">刷新设备</button>
    <p v-if="network_id&&!busy&&!peers.length" class="muted">没有发现可列出的 Rove 设备。请检查网络是否已启动；空列表不代表已连接成功。</p>
    <article v-for="peer in peers" :key="peer.device_id"><div><strong>{{peer.display_name}}</strong><p>{{peer.os}} / {{peer.arch}} · {{peer_state(peer)}}</p><small>{{peer.device_id}}</small><p v-if="peer.last_error" class="error">{{peer.last_error.code}}: {{peer.last_error.message}}</p></div><button :disabled="busy||disabled||peer.state!=='online'" @click="choose(peer)">选择设备</button></article>
    <button v-if="peer_cursor" :disabled="busy||disabled" @click="refresh_peers(true)">更多设备</button>
  </div>
</template>
<style scoped>
.picker{margin-bottom:20px}.actions{display:flex;gap:8px;margin:12px 0}select{padding:10px;margin:8px;border:1px solid #ccd7ca;border-radius:7px;max-width:100%}small{overflow-wrap:anywhere}article{display:flex;justify-content:space-between;gap:12px}
</style>
