<script setup lang="ts">
import {ref,watch} from 'vue';
import {t,notice_text} from './i18n';
import {call, type Json, type Target} from './api';
import {network_secret,form_config} from './network-config';
const props=defineProps<{initial?:Record<string,Json>;displayName?:string;localIpv4?:string|null;target?:Target;busy:boolean;editing?:boolean}>();
const emit=defineEmits<{save:[value:Record<string,Json>]}>();
const name=ref(''),secret=ref(''),show=ref(false),automatic=ref(true),subnet=ref('192.168.100.0/24'),local=ref(''),error=ref('');
type Peer={endpoint:string;testing:boolean;status:string;latency:number|null};
const peers=ref<Peer[]>([]);
watch(()=>props.initial,()=>{
  name.value=props.displayName??'';secret.value=String(props.initial?.network_secret??network_secret());show.value=false;
  automatic.value=props.initial?.dhcp!==false;subnet.value=String(props.initial?.ipv4_cidr??'192.168.100.0/24');local.value=props.localIpv4??'';
  peers.value=((props.initial?.bootstrap_peers??['']) as string[]).map(endpoint=>({endpoint,testing:false,status:'',latency:null}));error.value='';
},{immediate:true});
async function test(peer:Peer){
  const endpoint=peer.endpoint.trim();peer.testing=true;peer.status='';
  try{const result=await call('test_network_peer',{endpoint},undefined,undefined,props.target) as {status:string;latency_ms:number|null};if(peer.endpoint.trim()===endpoint){peer.status=result.status;peer.latency=result.latency_ms;}}
  catch(e){if(peer.endpoint.trim()===endpoint)peer.status=e instanceof Error?e.message:'unreachable';}
  finally{peer.testing=false;}
}
function submit(){
  error.value='';
  try{const config=form_config(name.value,secret.value,peers.value.map(p=>p.endpoint),automatic.value,subnet.value,props.initial);
    emit('save',{display_name:name.value.trim(),[props.editing?'easytier':'config']:config,local_ipv4:automatic.value?null:(local.value.trim()||null)});
  }catch{error.value=t('network_form.invalid_subnet');}
}
</script>
<template>
  <form class="vertical network-form" @submit.prevent="submit">
    <label>{{t('network_form.name')}}<input v-model="name" required maxlength="128" :placeholder="t('ui.network_name')"/></label>
    <label>{{t('network_form.key')}}<span class="key-row"><input v-model="secret" required maxlength="4096" :type="show?'text':'password'" autocomplete="off" spellcheck="false"/><button type="button" @click="show=!show">{{t(show?'network_form.hide':'network_form.show')}}</button><button type="button" @click="secret=network_secret()">{{t('network_form.regenerate')}}</button></span></label>
    <label>{{t('network_form.mode')}}<select v-model="automatic"><option :value="true">{{t('network_form.automatic')}}</option><option :value="false">{{t('network_form.manual')}}</option></select></label>
    <p v-if="automatic" class="muted">{{t('network_form.dhcp_hint')}}</p>
    <template v-else><label>{{t('network_form.subnet')}}<input v-model="subnet" required placeholder="192.168.100.0/24"/></label><label>{{t('network_form.local_ip')}}<input v-model="local" placeholder="192.168.100.2"/></label><p class="muted">{{t('network_form.local_hint')}}</p></template>
    <fieldset><legend>{{t('network_form.peers')}}</legend>
      <div v-for="(peer,index) in peers" :key="index" class="peer-row">
        <input v-model="peer.endpoint" :aria-label="t('network_form.peer',{n:index+1})" placeholder="tcp://host:11010" @input="peer.status=''"/>
        <button type="button" :disabled="peer.testing||!peer.endpoint.trim()" @click="test(peer)">{{t('network_form.test')}}</button>
        <button type="button" :aria-label="t('network_form.remove_peer',{n:index+1})" @click="peers.splice(index,1)">{{t('network_form.remove')}}</button>
        <p v-if="peer.status" role="status">{{['reachable','unreachable','unsupported'].includes(peer.status)?t('network_form.'+peer.status):notice_text(peer.status)}}{{peer.latency!==null&&peer.status==='reachable'?` · ${peer.latency} ms`:''}}</p>
      </div>
      <button type="button" :disabled="peers.length>=64" @click="peers.push({endpoint:'',testing:false,status:'',latency:null})">{{t('network_form.add_peer')}}</button>
      <p class="muted">{{t('network_form.probe_hint')}}</p>
    </fieldset>
    <p v-if="error" role="alert" class="error">{{error}}</p>
    <button :disabled="busy">{{t(editing?'ui.save_changes':'ui.create_network')}}</button>
  </form>
</template>
<style scoped>
.network-form{padding:12px 0}.key-row,.peer-row{display:flex;gap:8px;flex-wrap:wrap}.key-row input,.peer-row input{flex:1;min-width:140px}.peer-row{margin-bottom:8px}.peer-row p{flex-basis:100%;margin:0}fieldset{min-width:0;border:1px solid var(--border,#d8e2df);border-radius:10px;padding:12px}input,select{box-sizing:border-box;width:100%}button{white-space:nowrap}
</style>
