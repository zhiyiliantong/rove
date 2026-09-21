<script setup lang="ts">
import {t,notice_text,state_label} from './i18n';
import {ref,shallowRef,onMounted,onBeforeUnmount} from 'vue';
import {call as agent_call,type Target,type Json} from './api';
import {openUrl} from '@tauri-apps/plugin-opener';
import {browser_address,can_open} from './service-address';
interface Service {service_id:string;network_id:string;name:string;protocol:string;target:{host:string;port:number};listen_port:number;access_info:string;state:string;endpoints:string[];target_status:string;last_error:null|{code?:string;message:string}}
const props=defineProps<{target?:Target}>();
const network=ref(props.target?.network_id??''),items=shallowRef<Service[]>([]),cursor=ref<string|null>(null);
const busy=ref(false),error=ref(''),revealed=ref('');
const networks=shallowRef<{network_id:string;display_name:string}[]>([]),network_cursor=ref<string|null>(null);
let alive=true;
function call(...args:Parameters<typeof agent_call>){return agent_call(args[0],args[1],args[2],args[3],props.target);}
async function action(task:()=>Promise<void>){if(busy.value)return;busy.value=true;error.value='';try{await task();}catch(e){if(alive)error.value=e instanceof Error?e.message:t('ui.service_operation_failed');}finally{busy.value=false;}}
async function load(append=false){const id=network.value;const page=await call('list_services',undefined,undefined,{network_id:id,...(append&&cursor.value?{cursor:cursor.value}:{})}) as unknown as {items:Service[];next_cursor:string|null};if(!alive||id!==network.value)return;items.value=append?[...items.value,...page.items]:page.items;cursor.value=page.next_cursor;}
async function load_networks(append=false){
  const page=await call('list_networks',undefined,undefined,append&&network_cursor.value?{cursor:network_cursor.value}:{}) as unknown as {items:{network_id:string;display_name:string}[];next_cursor:string|null};
  if(!alive)return;
  networks.value=append?[...networks.value,...page.items]:page.items;network_cursor.value=page.next_cursor;
  if(!network.value)network.value=networks.value[0]?.network_id??'';
  if(network.value)await load();
}
function change_network(){items.value=[];cursor.value=null;revealed.value='';if(network.value)void action(()=>load());}
async function copy(address:string){await action(async()=>{if(!navigator.clipboard)throw new Error(t('ui.clipboard_unavailable_copy_the_address_manually'));await navigator.clipboard.writeText(address);});}
async function open(address:string){await action(()=>openUrl(browser_address(address)));}
onMounted(()=>action(()=>load_networks()));
onBeforeUnmount(()=>{alive=false;});
</script>
<template>
  <section>
    <h2>{{t('ui.services_provided_by_this_device')}}</h2>
    <p class="muted">{{t('ui.the_service_directory_belongs_to_the_selected_device')}}</p>
    <p v-if="error" class="error" role="alert">{{notice_text(error)}}</p>
    <form @submit.prevent="action(()=>load_networks())"><label>{{t('service_browser.network')}}<select v-model="network" :disabled="busy||(!networks.length&&!network)" @change="change_network">
      <option v-if="!network" value="">{{t('service_browser.no_networks')}}</option>
      <option v-if="network&&!networks.some(n=>n.network_id===network)" :value="network">{{network}}</option>
      <option v-for="item in networks" :key="item.network_id" :value="item.network_id">{{item.display_name}} · {{item.network_id}}</option>
    </select></label><button :disabled="busy">{{t('ui.refresh')}}</button></form>
    <button v-if="network_cursor" :disabled="busy" @click="action(()=>load_networks(true))">{{t('ui.load_more_networks')}}</button>
    <p v-if="!busy&&network&&!items.length&&!error" class="muted">{{t('service_browser.empty')}}</p>
    <article v-for="service in items" :key="service.service_id" class="service-card">
      <div><h3>{{service.name}}</h3><p>{{state_label(service.state)}} {{t('ui.target')}} {{state_label(service.target_status)}}</p><small>{{service.service_id}}</small><p v-if="service.last_error" class="error">{{notice_text((service.last_error.code?service.last_error.code+': ':'')+service.last_error.message)}}</p></div>
      <div v-for="address in service.endpoints" :key="address" class="address"><input readonly :value="address" :aria-label="t('ui.service_access_address')"/><button v-if="can_open(address)" :disabled="busy" @click="open(address)">{{t('ui.open_in_browser')}}</button><button :disabled="busy" @click="copy(address)">{{t('ui.copy_address')}}</button></div>
      <p v-if="!service.endpoints.length" class="muted">{{t('ui.no_endpoint_is_currently_available_a_saved_definition')}}</p>
      <div class="actions"><button @click="revealed=revealed===service.service_id?'':service.service_id">{{revealed===service.service_id?t('ui.hide_authentication_notes'):t('ui.view_authentication_notes')}}</button></div>
      <pre v-if="revealed===service.service_id" class="secret">{{service.access_info||t('ui.no_application_authentication_notes_provided')}}</pre>
    </article>
    <button v-if="cursor" :disabled="busy" @click="action(()=>load(true))">{{t('ui.load_more')}}</button>
    <p class="muted">{{t('ui.start_installation_and_service_management_through_a_conversation')}}</p>
  </section>
</template>
<style scoped>
.service-card{display:block;padding:18px 0}.service-card h3{margin:0}.actions,.address{display:flex;flex-wrap:wrap;gap:8px;margin:12px 0}.actions button{font-size:12px}.address input{flex:1;min-width:180px}.secret{white-space:pre-wrap;overflow-wrap:anywhere}select{padding:10px;border:1px solid #ccd7ca;border-radius:7px;background:#fafcf9}small{overflow-wrap:anywhere}
</style>
