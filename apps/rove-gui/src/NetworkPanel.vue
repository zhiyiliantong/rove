<script setup lang="ts">
import {t,notice_text,state_label} from './i18n';
import {ref,shallowRef,onMounted,onBeforeUnmount,nextTick,watch,computed} from 'vue';
import {invoke} from '@tauri-apps/api/core';
import jsQR from 'jsqr';
import {call as agent_call,type Json,type Target} from './api';
import {CONFIG_FILE_LIMIT,decode_network_config,encode_network_config,parse_config_file} from './network-import';
import NetworkForm from './NetworkForm.vue';
import {save_network_edit,NetworkEditError,type EditStage} from './network-edit';
const edit_stage=ref<EditStage>();
import NetworkError from './components/NetworkError.vue';
import ModelSync from './ModelSync.vue';
import Dialog from 'primevue/dialog';
import DevicePicker from './DevicePicker.vue';
const props=defineProps<{target?:Target;active?:boolean;mobile?:boolean;mode?:'create'|'join';networkId?:string}>();
const emit=defineEmits<{done:[];open:[id:string]}>();
type View='list'|'create'|'join'|'card'|'edit';
const view=ref<View>(props.mode??'list');
const view_title=computed(()=>t(view.value==='card'?'network_form.card':view.value==='join'?'ui.join_network':view.value==='edit'?'ui.edit_network_configuration_explicitly':'ui.create_network'));
watch(()=>props.active,value=>{if(value===false){stop_camera();if(!props.mode)view.value='list';}else if(value)void action(refresh);});
function call(...args:Parameters<typeof agent_call>){return agent_call(args[0],args[1],args[2],args[3],props.target);}
interface Network {network_id:string;display_name:string;state:string;local_ipv4?:string|null;overlay_cidr?:string|null;overlay_addresses?:string[];last_error:null|{code?:string;message:string}}
const detail=shallowRef<Network>();
let detail_generation=0;
const networks=shallowRef<Network[]>([]),next_cursor=ref<string|null>(null),sync_network=ref<Network>();
const error=ref(''),busy=ref(false),create_key=ref(0),import_value=ref(''),file_notice=ref('');
const edit_config=ref<Record<string,Json>>(),edit_name=ref(''),edit_local=ref<string|null>(null),editing=ref('');
const exported=ref(''),card_payload=ref(''),qr_svg=ref(''),card_id=ref('');
const qr_error=ref(''),copied=ref(false);
let card_generation=0;
function invalidate_card(){card_generation++;exported.value='';card_payload.value='';qr_svg.value='';qr_error.value='';copied.value=false;card_id.value='';}
const camera=ref(false),video=ref<HTMLVideoElement>(),camera_error=ref('');
const card_element=ref<HTMLElement>();
const card=computed(()=>exported.value?JSON.parse(exported.value) as {display_name:string;easytier:{network_name:string;network_secret:string;dhcp:boolean;ipv4_cidr?:string;bootstrap_peers:string[]}}:null);
let media:MediaStream|undefined,frame:number|undefined,alive=true,camera_generation=0;
async function action(task:()=>Promise<void>){busy.value=true;error.value='';try{await task();}catch(e){error.value=e instanceof Error?e.message:t('ui.operation_failed');}finally{busy.value=false;}}
async function load(append=false){const page=await call('list_networks',undefined,undefined,append&&next_cursor.value?{cursor:next_cursor.value}:{}) as unknown as {items:Network[];next_cursor:string|null};if(!alive)return;networks.value=append?[...networks.value,...page.items]:page.items;next_cursor.value=page.next_cursor;}
function show(next:View){stop_camera();if(next!=='card')invalidate_card();error.value='';file_notice.value='';if(next==='create')create_key.value++;if(next==='join')import_value.value='';view.value=next;}
async function load_detail(){
  const id=props.networkId,current=++detail_generation;if(!id){detail.value=undefined;return;}
  try{const value=await call('get_network',undefined,{network_id:id}) as unknown as Network;
    if(!alive||current!==detail_generation||id!==props.networkId)return;
    if(value.network_id!==id)throw new Error(t('ui.device_list_network_context_does_not_match'));
    detail.value=value;
  }catch(e){if(alive&&current===detail_generation&&id===props.networkId)throw e;}
}
async function refresh(){await load();await load_detail();}
watch(()=>props.networkId,()=>{detail.value=undefined;show('list');void action(load_detail);});
defineExpose({open:(next:'create'|'join')=>show(next),refresh:()=>action(refresh),busy,detail});
async function render_qr(){
  const generation=card_generation,payload=card_payload.value;if(!payload)return;
  qr_error.value='';qr_svg.value='';
  try{const svg=await invoke<string>('network_config_qr',{config:payload});if(alive&&generation===card_generation&&payload===card_payload.value)qr_svg.value=svg;}
  catch{if(alive&&generation===card_generation)qr_error.value=t('network_import.qr_failed');}
}
async function load_card(id:string){
  invalidate_card();const generation=card_generation;error.value='';card_id.value=id;
  try{
    const config=await call('get_network_join_config',undefined,{network_id:id}) as Json;
    if(!alive||generation!==card_generation||view.value!=='card')return;
    const payload=encode_network_config(config);
    exported.value=JSON.stringify(config,null,2);card_payload.value=payload;
    await render_qr();
    await nextTick();if(alive&&generation===card_generation)card_element.value?.focus({preventScroll:true});
  }catch{if(alive&&generation===card_generation)error.value=t('network_form.card_load_failed');}
}
async function open_card(id:string){view.value='card';busy.value=true;await load_card(id);busy.value=false;}
async function create(body:Record<string,Json>){
  busy.value=true;error.value='';
  try{const network=await call('create_network',body) as unknown as Network;try{await load();}catch{/* The saved network can still provide its card. */}view.value='card';await load_card(network.network_id);}
  catch(e){error.value=e instanceof Error?e.message:t('ui.operation_failed');}
  finally{busy.value=false;}
}
async function join(){await action(async()=>{const config=decode_network_config(import_value.value);await call('import_network',{source:'manual',config});import_value.value='';await load();if(props.mode)emit('done');else view.value='list';});}
async function read_file(event:Event){
  const input=event.target as HTMLInputElement,file=input.files?.[0];input.value='';file_notice.value='';if(!file)return;
  await action(async()=>{if(file.size>CONFIG_FILE_LIMIT*2)throw new Error(t('network_import.too_large'));let result;try{result=parse_config_file(await file.text());}catch{throw new Error(t('network_import.invalid'));}if(!alive)return;import_value.value=encode_network_config(result.config);file_notice.value=t('network_import.loaded');});
}
async function copy_card(){try{await navigator.clipboard.writeText(card_payload.value);copied.value=true;}catch{error.value=t('ui.clipboard_unavailable_select_the_text_below_and_copy');}}
async function network_action(operation:string,id:string){await action(async()=>{await call(operation,undefined,{network_id:id});await refresh();});}
async function remove(network:Network){if(!window.confirm(t('ui.delete_the_local_configuration_for_value_applications_will',{p0:network.display_name})))return;await action(async()=>{await call('delete_network',undefined,{network_id:network.network_id});detail_generation++;detail.value=undefined;await load();if(props.networkId===network.network_id)emit('open','');});}
async function edit(network:Network){await action(async()=>{const config=await call('get_network_join_config',undefined,{network_id:network.network_id}) as Record<string,Json>;edit_config.value=config.easytier as Record<string,Json>;edit_name.value=String(config.display_name);edit_local.value=network.local_ipv4??null;editing.value=network.network_id;view.value='edit';});}
async function save_edit(config:Record<string,Json>){if(busy.value)return;await action(async()=>{
  try{await save_network_edit(call,editing.value,config,stage=>{edit_stage.value=stage;});}
  catch(e){if(e instanceof NetworkEditError)throw new Error(t('network_form.edit_failed_'+e.stage)+': '+e.message);throw e;}
  finally{invalidate_card();edit_stage.value=undefined;try{await refresh();}catch{/* Preserve the lifecycle error and edited draft. */}}
  editing.value='';view.value='list';
});}
function back(){if(busy.value)return;stop_camera();invalidate_card();error.value='';if(props.mode)emit('done');else show('list');}
function stop_camera(){camera_generation++;camera.value=false;if(frame!==undefined)cancelAnimationFrame(frame);frame=undefined;media?.getTracks().forEach(track=>track.stop());media=undefined;}
async function start_camera(){
  stop_camera();const generation=camera_generation;camera_error.value='';
  if(!navigator.mediaDevices?.getUserMedia){camera_error.value=t('ui.no_camera_interface_is_available_on_this_platform');return;}
  camera.value=true;
  try{
    const stream=await navigator.mediaDevices.getUserMedia({video:{facingMode:{ideal:'environment'}},audio:false});
    if(!alive||generation!==camera_generation){stream.getTracks().forEach(track=>track.stop());return;}
    media=stream;await nextTick();if(!video.value){stop_camera();return;}video.value.srcObject=media;await video.value.play();
    if(!alive||generation!==camera_generation)return;
    const canvas=document.createElement('canvas'),context=canvas.getContext('2d',{willReadFrequently:true});if(!context)throw new Error(t('network_import.invalid'));
    let last_scan=0;
    const scan=(time:number)=>{if(!alive||!camera.value||generation!==camera_generation)return;try{const source=video.value;if(source&&source.readyState>=2&&time-last_scan>200){last_scan=time;const scale=Math.min(1,800/source.videoWidth);canvas.width=Math.max(1,Math.round(source.videoWidth*scale));canvas.height=Math.max(1,Math.round(source.videoHeight*scale));context.drawImage(source,0,0,canvas.width,canvas.height);const pixels=context.getImageData(0,0,canvas.width,canvas.height),code=jsQR(pixels.data,pixels.width,pixels.height);if(code){decode_network_config(code.data);import_value.value=code.data;stop_camera();return;}}frame=requestAnimationFrame(scan);}catch{stop_camera();camera_error.value=t('network_import.invalid');}};
    frame=requestAnimationFrame(scan);
  }catch{if(generation===camera_generation){stop_camera();camera_error.value=t('ui.camera_unavailable_or_permission_denied_paste_the_sharing');}}
}
function hidden(){if(document.hidden)stop_camera();}
onMounted(()=>{document.addEventListener('visibilitychange',hidden);void action(refresh);});
onBeforeUnmount(()=>{alive=false;invalidate_card();stop_camera();document.removeEventListener('visibilitychange',hidden);});
</script>

<template>
  <section class="network-panel" :class="{'network-panel-embedded':mode}">
    <ModelSync v-if="sync_network&&!target" :key="sync_network.network_id" :network-id="sync_network.network_id" :network-name="sync_network.display_name" @close="sync_network=undefined"/>
    <template v-if="!mode&&!networkId">
      <NetworkError v-if="error&&view==='list'" :message="error"/>
      <article v-for="network in networks" :key="network.network_id" class="network-row"><button class="network-entry" :aria-label="network.display_name" @click="emit('open',network.network_id)"><i class="pi pi-share-alt" aria-hidden="true"/><span><strong>{{network.display_name}}</strong><small>{{state_label(network.state)}}<template v-if="network.overlay_cidr"> · {{network.overlay_cidr}}</template></small></span><i class="pi pi-chevron-right" aria-hidden="true"/></button><p v-if="network.last_error" class="error">{{notice_text((network.last_error.code?network.last_error.code+': ':'')+network.last_error.message)}}</p></article>
      <div v-if="!networks.length" class="empty-network"><p class="muted">{{t('network_import.empty_list')}}</p></div><button v-if="next_cursor" :disabled="busy" @click="action(()=>load(true))">{{t('ui.load_more_networks')}}</button>
    </template>
    <template v-else-if="!mode&&networkId">
      <p v-if="busy&&!detail" role="status">{{t('network_detail.loading')}}</p>
      <NetworkError v-if="error&&view==='list'" :message="error"/>
      <template v-if="detail">
        <div class="network-detail-summary"><span>{{state_label(detail.state)}}</span><button :disabled="busy" @click="open_card(detail.network_id)"><i class="pi pi-qrcode" aria-hidden="true"/> {{t('network_form.card')}}</button></div>
        <dl class="network-facts"><div><dt>{{t('ui.network_id')}}</dt><dd>{{detail.network_id}}</dd></div><div><dt>{{t('network_form.actual_subnet')}}</dt><dd>{{detail.overlay_cidr||'—'}}</dd></div><div><dt>{{t('network_detail.addresses')}}</dt><dd>{{detail.overlay_addresses?.join(', ')||'—'}}</dd></div></dl>
        <NetworkError v-if="detail.last_error" :message="(detail.last_error.code?detail.last_error.code+': ':'')+detail.last_error.message"/>
        <div class="actions network-controls"><button :disabled="busy||!['stopped','failed'].includes(detail.state)" @click="network_action('start_network',detail.network_id)">{{t('ui.start')}}</button><button :disabled="busy||detail.state==='stopped'" @click="network_action('stop_network',detail.network_id)">{{t('ui.stop')}}</button><button :disabled="busy" :aria-label="t('ui.edit_network_configuration_explicitly')" @click="edit(detail)"><i class="pi pi-pencil" aria-hidden="true"/> {{t('ui.edit_network_configuration_explicitly')}}</button><button :disabled="busy" @click="remove(detail)">{{t('ui.delete')}}</button><button v-if="!target" :disabled="busy||detail.state!=='running'" @click="sync_network=detail">{{t('model_sync.title')}}</button></div>
        <DevicePicker v-if="!busy" :key="networkId" :network-id="networkId" :target="target" :disabled="false" scoped-view/>
      </template>
    </template>
    <component :is="mode?'div':Dialog" v-if="view!=='list'" :class="mode?'network-flow':'network-dialog'" :visible="true" modal :closable="false" :close-on-escape="!busy" :style="{width:mode?'100%':view==='card'?'32rem':'38rem'}" :aria-label="view_title" @update:visible="back">
    <header class="network-flow-header"><button class="network-back" :disabled="busy" :aria-label="t(mode?'intro.close':'ui.back_to_networks')" :title="t(mode?'intro.close':'ui.back_to_networks')" @click="back"><i class="pi pi-chevron-left" aria-hidden="true"/></button><h2>{{view_title}}</h2></header>
    <div class="network-flow-body">
    <template v-if="view==='create'">
      <p v-if="error" class="error" role="alert">{{notice_text(error)}}</p><NetworkForm :key="create_key" :busy="busy" :target="target" @save="create"/>
    </template>
    <template v-else-if="view==='join'">
      <p>{{t('network_import.base64_hint')}}</p><form class="vertical" @submit.prevent="join"><label>{{t('network_import.base64_label')}}<textarea v-model="import_value" rows="7" required spellcheck="false" autocomplete="off" :placeholder="t('network_import.base64_placeholder')"/></label><button :disabled="busy">{{t('network_import.decode_import')}}</button></form><div class="join-actions"><button v-if="mobile&&!camera" class="secondary" :disabled="busy" @click="start_camera">{{t('ui.scan_qr_code')}}</button><button v-if="camera" class="secondary" @click="stop_camera">{{t('ui.close_camera')}}</button><label>{{t('network_import.file')}}<input type="file" accept=".txt,.rove,text/plain" :disabled="busy" @change="read_file"/></label></div><p v-if="file_notice" role="status">{{file_notice}}</p><video v-if="camera" ref="video" muted playsinline :aria-label="t('ui.qr_code_camera_preview')"/><p v-if="camera_error" class="error" role="alert">{{notice_text(camera_error)}}</p><p v-if="error" class="error" role="alert">{{notice_text(error)}}</p>
    </template>
    <template v-else-if="view==='card'">
      <div v-if="card" ref="card_element" class="network-card" tabindex="-1"><p>{{t('network_form.name')}}：{{card.display_name}} · {{card.easytier.network_name}}</p><p>{{t('network_form.subnet')}}：{{card.easytier.dhcp?t('network_form.automatic'):card.easytier.ipv4_cidr}}</p><div v-if="qr_svg" class="qr" role="img" :aria-label="t('ui.network_sharing_qr_code')" v-html="qr_svg"/><div v-if="qr_error" role="alert"><p class="error">{{qr_error}}</p><button class="secondary" @click="render_qr">{{t('network_import.retry_qr')}}</button></div><p>{{t('network_import.card_hint')}}</p><label>{{t('network_import.base64_label')}}<textarea readonly :value="card_payload" rows="3"/></label><p v-if="error" class="error" role="alert">{{notice_text(error)}}</p></div>
      <p v-else-if="busy" role="status">{{t('network_detail.loading')}}</p>
      <div v-else class="secret network-card"><p class="error" role="alert">{{t('network_form.card_load_failed')}}</p><button :disabled="busy" @click="open_card(card_id)">{{t('network_form.retry_card')}}</button></div>
    </template>
    <template v-else-if="view==='edit'">
      <p v-if="error" class="error" role="alert">{{notice_text(error)}}</p>
      <NetworkForm :key="editing" :initial="edit_config" :display-name="edit_name" :local-ipv4="edit_local" :busy="busy" :restart-on-save="['running','starting'].includes(detail?.state??'')" :save-stage="edit_stage" :target="target" editing @save="save_edit"/>
    </template>
    </div>
    <footer v-if="view==='card'&&card" class="network-flow-footer"><button class="copy-config" @click="copy_card"><i class="pi pi-copy" aria-hidden="true"/> {{t('network_import.copy')}}</button><span v-if="copied" role="status">{{t('network_import.copied')}}</span></footer>
    </component>
  </section>
</template>

<style scoped>
.heading,.primary-actions,.join-actions{display:flex;align-items:center;justify-content:space-between;gap:8px;flex-wrap:wrap}.actions{display:flex;flex-wrap:wrap;gap:6px}.actions button{font-size:12px;padding:7px 10px}article{flex-wrap:wrap;gap:10px}.back{margin-bottom:12px}.empty-network{padding:18px;border:1px dashed var(--border,#d8e2df);border-radius:10px}.empty-network button{margin-right:8px}textarea{width:100%;box-sizing:border-box}.qr{width:min(260px,100%);margin:16px auto}.qr :deep(svg){width:100%;display:block}video{width:min(480px,100%);display:block;margin:12px 0;border-radius:8px}.network-card,.card-peers{overflow-wrap:anywhere;min-width:0}.network-card h2{margin:0 0 12px}.network-card p{margin:10px 0}.network-card textarea{font-family:ui-monospace,monospace;word-break:break-all}.join-actions{justify-content:flex-start;margin:12px 0}.join-actions label{display:flex;align-items:center;gap:8px}.secondary{background:transparent;color:inherit;border:1px solid var(--border,#b9c9c3)}
</style>
