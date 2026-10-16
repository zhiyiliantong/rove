<script setup lang="ts">
import {ref,watch} from 'vue';
import {t} from './i18n';
import {call, type Json, type Target} from './api';
import {network_secret,generated_network_name,form_config} from './network-config';
import NetworkError from './components/NetworkError.vue';
const props=defineProps<{initial?:Record<string,Json>;displayName?:string;localIpv4?:string|null;target?:Target;busy:boolean;editing?:boolean;restartOnSave?:boolean;saveStage?:string}>();
const emit=defineEmits<{save:[value:Record<string,Json>]}>();
const name=ref(''),secret=ref(''),show=ref(false),automatic=ref(true),subnet=ref('192.168.100.0/24'),local=ref(''),error=ref('');
type DiscoveryResolution={source_url:string;resolved_url:string;protocol:string;chain:string[]};
type Peer={endpoint:string;testing:boolean;status:string;latency:number|null;resolvedEndpoint:string;resolutionChain:string[]};
const peers=ref<Peer[]>([]);
interface PublicNode {id:string;name:string;address:string;protocol:string;health_status:string;latency_ms?:number|null;relay_claim:string;metadata_verified:boolean}
const publicNodes=ref<PublicNode[]>([]),publicLoading=ref(false),publicFetched=ref(false),publicStale=ref(false),publicError=ref('');
watch(()=>props.initial,()=>{
  name.value=props.displayName??(props.initial?'':generated_network_name());secret.value=String(props.initial?.network_secret??network_secret());show.value=false;
  automatic.value=props.initial?.dhcp!==false;subnet.value=String(props.initial?.ipv4_cidr??'192.168.100.0/24');local.value=props.localIpv4??'';
  peers.value=((props.initial?.bootstrap_peers??['']) as string[]).map(endpoint=>newPeer(endpoint));error.value='';
},{immediate:true});
function newPeer(endpoint=''):Peer{return {endpoint,testing:false,status:'',latency:null,resolvedEndpoint:'',resolutionChain:[]};}
function clearPeerResult(peer:Peer){peer.status='';peer.latency=null;peer.resolvedEndpoint='';peer.resolutionChain=[];}
function addPeer(endpoint:string){
  if(peers.value.some(peer=>peer.endpoint.trim()===endpoint))return;
  const empty=peers.value.find(peer=>!peer.endpoint.trim());
  if(empty){empty.endpoint=endpoint;clearPeerResult(empty);}else if(peers.value.length<64)peers.value.push(newPeer(endpoint));
}
async function fetchPublic(){
  publicLoading.value=true;publicFetched.value=false;publicStale.value=false;publicError.value='';
  try{const result=await call('list_public_bootstrap_nodes',undefined,undefined,undefined,props.target) as unknown as {items:PublicNode[];stale:boolean};publicNodes.value=result.items;publicStale.value=result.stale;}
  catch(e){publicNodes.value=[];publicError.value=e instanceof Error?e.message:t('network_form.public_failed');}
  finally{publicLoading.value=false;publicFetched.value=true;}
}
async function test(peer:Peer){
  const endpoint=peer.endpoint.trim();peer.testing=true;clearPeerResult(peer);
  try{
    let probeEndpoint=endpoint;
    if(/^(https?|txt|srv):\/\//i.test(endpoint)){
      const resolution=await call('resolve_bootstrap_discovery',{url:endpoint,network_name:name.value.trim()},undefined,undefined,props.target) as unknown as DiscoveryResolution;
      if(peer.endpoint.trim()!==endpoint)return;
      probeEndpoint=resolution.resolved_url;
      peer.resolvedEndpoint=probeEndpoint;
      peer.resolutionChain=resolution.chain;
    }
    const result=await call('test_network_peer',{endpoint:probeEndpoint},undefined,undefined,props.target) as {status:string;latency_ms:number|null};
    if(peer.endpoint.trim()===endpoint){peer.status=result.status;peer.latency=result.latency_ms;}
  }
  catch(e){if(peer.endpoint.trim()===endpoint)peer.status=e instanceof Error?e.message:'unreachable';}
  finally{peer.testing=false;}
}
function submit(){
  if(props.busy)return;
  error.value='';
  try{const config=form_config(name.value,secret.value,peers.value.map(p=>p.endpoint),automatic.value,subnet.value,props.initial);
    emit('save',{display_name:name.value.trim(),[props.editing?'easytier':'config']:config,local_ipv4:automatic.value?null:(local.value.trim()||null)});
  }catch{error.value=t('network_form.invalid_subnet');}
}
</script>
<template>
  <form class="vertical network-form" @submit.prevent="submit">
    <label>{{t('network_form.name')}}<input v-model="name" required maxlength="128" :placeholder="t('ui.network_name')"/></label>
    <label>{{t('network_form.key')}}<span class="key-row"><input v-model="secret" :readonly="editing" required maxlength="4096" :type="show?'text':'password'" autocomplete="off" spellcheck="false"/><button type="button" @click="show=!show">{{t(show?'network_form.hide':'network_form.show')}}</button><button v-if="!editing" type="button" @click="secret=network_secret()">{{t('network_form.regenerate')}}</button></span></label>
    <label>{{t('network_form.mode')}}<select v-model="automatic"><option :value="true">{{t('network_form.automatic')}}</option><option :value="false">{{t('network_form.manual')}}</option></select></label>
    <p v-if="automatic" class="muted">{{t('network_form.dhcp_hint')}}</p>
    <template v-else><label>{{t('network_form.subnet')}}<input v-model="subnet" required placeholder="192.168.100.0/24"/></label><label>{{t('network_form.local_ip')}}<input v-model="local" placeholder="192.168.100.2"/></label><p class="muted">{{t('network_form.local_hint')}}</p></template>
    <fieldset><legend>{{t('network_form.peers')}}</legend>
      <div class="discovery-actions"><button type="button" :disabled="publicLoading" @click="fetchPublic">{{t(publicLoading?'network_form.public_loading':'network_form.public_fetch')}}</button></div>
      <p class="muted">{{t('network_form.public_hint')}}</p>
      <NetworkError v-if="publicError" :message="publicError"/>
      <p v-else-if="publicStale" class="notice" role="status">{{t('network_form.public_stale')}}</p>
      <p v-else-if="publicFetched&&!publicLoading&&publicNodes.length===0" class="muted">{{t('network_form.public_empty')}}</p>
      <div v-for="node in publicNodes" :key="node.id" class="public-node">
        <span><strong>{{node.name}}</strong><small>{{node.address}} · {{node.health_status}}{{node.latency_ms!=null?` · ${node.latency_ms} ms`:''}} · {{node.metadata_verified?'verified':'community metadata'}}</small></span>
        <button type="button" :disabled="peers.some(peer=>peer.endpoint.trim()===node.address)" :aria-label="t('network_form.use_public_named',{name:node.name})" @click="addPeer(node.address)">{{t('network_form.use_public')}}</button>
      </div>
      <div v-for="(peer,index) in peers" :key="index" class="peer-row">
        <input v-model="peer.endpoint" :aria-label="t('network_form.peer',{n:index+1})" placeholder="https://directory.example/node · txt://bootstrap.example.com · tcp://host:11010" @input="clearPeerResult(peer)"/>
        <button type="button" :disabled="peer.testing||!peer.endpoint.trim()" @click="test(peer)">{{t('network_form.test')}}</button>
        <button type="button" :aria-label="t('network_form.remove_peer',{n:index+1})" @click="peers.splice(index,1)">{{t('network_form.remove')}}</button>
        <p v-if="peer.resolvedEndpoint" class="muted">{{t('network_form.resolved_for_probe',{endpoint:peer.resolvedEndpoint})}}<small>{{peer.resolutionChain.join(' → ')}}</small></p>
        <NetworkError v-if="peer.status&&!['reachable','unreachable','unsupported'].includes(peer.status)" style="flex-basis:100%" :message="peer.status"/>
        <p v-else-if="peer.status" role="status">{{t('network_form.'+peer.status)}}{{peer.latency!==null&&peer.status==='reachable'?` · ${peer.latency} ms`:''}}</p>
      </div>
      <button type="button" :disabled="peers.length>=64" @click="peers.push(newPeer())">{{t('network_form.add_peer')}}</button>
      <p class="muted">{{t('network_form.probe_hint')}}</p>
    </fieldset>
    <NetworkError v-if="error" :message="error"/>
    <p v-if="restartOnSave" class="notice" role="status">{{t('network_form.restart_hint')}}</p>
    <p v-if="saveStage" role="status">{{t('network_form.edit_'+saveStage)}}</p>
    <button :disabled="busy">{{t(editing?(restartOnSave?'network_form.save_restart':'ui.save_changes'):'ui.create_network')}}</button>
  </form>
</template>
<style scoped>
.network-form{padding:12px 0}.key-row,.peer-row,.public-node{display:flex;gap:8px;flex-wrap:wrap;align-items:center}.key-row input,.peer-row input{flex:1;min-width:140px}.peer-row{margin-bottom:8px}.peer-row p{flex-basis:100%;margin:0;overflow-wrap:anywhere}.peer-row p small{display:block}.discovery-actions{margin-bottom:6px}.public-node{justify-content:space-between;border:1px solid var(--border,#d8e2df);border-radius:8px;padding:8px;margin:8px 0}.public-node span{min-width:0;overflow-wrap:anywhere}.public-node small{display:block;overflow-wrap:anywhere}fieldset{min-width:0;border:1px solid var(--border,#d8e2df);border-radius:10px;padding:12px}input,select{box-sizing:border-box;width:100%}button{white-space:nowrap}
</style>
