<script setup lang="ts">
import {t,notice_text,format_datetime,state_label} from './i18n';
import {ref, shallowRef, onMounted, onBeforeUnmount, nextTick, watch,computed} from 'vue';
import {invoke} from '@tauri-apps/api/core';
import jsQR from 'jsqr';
import {call as agent_call, type Json, type Target} from './api';
import {CONFIG_FILE_LIMIT,parse_config_file} from './network-import';
import NetworkForm from './NetworkForm.vue';
import ModelSync from './ModelSync.vue';
const props=defineProps<{target?:Target;active?:boolean;mobile?:boolean;mode?:'create'|'join'}>();
watch(()=>props.active,value=>{if(value===false)stop_camera();else if(value)void action(()=>load());});
function call(...args:Parameters<typeof agent_call>){return agent_call(args[0],args[1],args[2],args[3],props.target);}
interface Network {network_id:string;display_name:string;state:string;local_ipv4?:string|null;overlay_cidr?:string|null;last_error:null|{code?:string;message:string}}
const networks=shallowRef<Network[]>([]), next_cursor=ref<string|null>(null);
const sync_network=ref<Network>();
const url=ref(''),manual=ref(''),error=ref(''),busy=ref(false),create_key=ref(0);
const edit_config=ref<Record<string,Json>>(),edit_name=ref(''),edit_local=ref<string|null>(null),card_secret=ref(false);
const shared=ref(''),expires=ref(''),qr_svg=ref(''),exported=ref(''),editing=ref(''),replacement=ref('');
const camera=ref(false),video=ref<HTMLVideoElement>(),camera_error=ref('');
const manual_open=ref(false),file_notice=ref('');
const card=computed(()=>exported.value?JSON.parse(exported.value) as {display_name:string;easytier:{network_name:string;network_secret:string;dhcp:boolean;ipv4_cidr?:string;bootstrap_peers:string[]}}:null);
let media:MediaStream|undefined,frame:number|undefined,alive=true,camera_generation=0;
async function action(task:()=>Promise<void>){busy.value=true;error.value='';try{await task();}catch(e){error.value=e instanceof Error?e.message:t('ui.operation_failed');}finally{busy.value=false;}}
async function load(append=false){const page=await call('list_networks',undefined,undefined,append&&next_cursor.value?{cursor:next_cursor.value}:{}) as unknown as {items:Network[];next_cursor:string|null};if(!alive)return;networks.value=append?[...networks.value,...page.items]:page.items;next_cursor.value=page.next_cursor;}
async function create(body:Record<string,Json>){await action(async()=>{const network=await call('create_network',body) as unknown as Network;create_key.value++;await load();card_secret.value=false;exported.value=JSON.stringify(await call('get_network_join_config',undefined,{network_id:network.network_id}),null,2);});}
async function join(){await action(async()=>{await call('import_network',{source:'url',url:url.value.trim()});url.value='';await load();});}
async function join_manual(){await action(async()=>{let config:Json;try{config=JSON.parse(manual.value);}catch{throw new Error(t('ui.network_configuration_is_not_valid_json'));}await call('import_network',{source:'manual',config});manual.value='';await load();});}
async function read_file(event:Event){
  const input=event.target as HTMLInputElement,file=input.files?.[0];input.value='';file_notice.value='';if(!file)return;
  await action(async()=>{
    if(file.size>CONFIG_FILE_LIMIT)throw new Error(t('network_import.too_large'));
    let result;try{result=parse_config_file(await file.text());}catch{throw new Error(t('network_import.invalid'));}
    if(!alive)return;
    if('url' in result){url.value=result.url;manual.value='';}else{manual.value=JSON.stringify(result.config,null,2);manual_open.value=true;url.value='';}
    file_notice.value=t('network_import.loaded');
  });
}
async function share(id:string){await action(async()=>{const result=await call('create_network_share',{}, {network_id:id}) as {url:string;expires_at:string};shared.value=result.url;expires.value=result.expires_at;qr_svg.value='';qr_svg.value=await invoke<string>('share_qr',{url:result.url});});}
async function network_action(operation:string,id:string){await action(async()=>{await call(operation,undefined,{network_id:id});await load();});}
async function remove(network:Network){if(!window.confirm(t('ui.delete_the_local_configuration_for_value_applications_will',{p0:(network.display_name)})))return;await network_action('delete_network',network.network_id);}
async function export_config(id:string){await action(async()=>{card_secret.value=false;exported.value=JSON.stringify(await call('get_network_join_config',undefined,{network_id:id}),null,2);});}
async function edit(network:Network){await action(async()=>{const config=await call('get_network_join_config',undefined,{network_id:network.network_id}) as Record<string,Json>;edit_config.value=config.easytier as Record<string,Json>;edit_name.value=String(config.display_name);edit_local.value=network.local_ipv4??null;editing.value=network.network_id;});}
async function save_edit(config:Record<string,Json>){await action(async()=>{await call('update_network',config,{network_id:editing.value});editing.value='';replacement.value='';await load();});}
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
    const canvas=document.createElement('canvas');const context=canvas.getContext('2d',{willReadFrequently:true});
    if(!context)throw new Error(t('ui.cannot_read_camera_images_use_url_import_instead'));
    let last_scan=0;
    const scan=(time:number)=>{
      if(!alive||!camera.value||generation!==camera_generation)return;
      try {
      const source=video.value;
      if(source&&source.readyState>=2&&time-last_scan>200){
        last_scan=time;const scale=Math.min(1,800/source.videoWidth);canvas.width=Math.max(1,Math.round(source.videoWidth*scale));canvas.height=Math.max(1,Math.round(source.videoHeight*scale));context.drawImage(source,0,0,canvas.width,canvas.height);
        const pixels=context.getImageData(0,0,canvas.width,canvas.height);const code=jsQR(pixels.data,pixels.width,pixels.height);
        if(code){url.value=code.data;stop_camera();return;}
      }
      frame=requestAnimationFrame(scan);
      } catch { stop_camera();camera_error.value=t('ui.cannot_decode_the_camera_image_paste_the_sharing'); }
    };
    frame=requestAnimationFrame(scan);
  }catch{if(generation===camera_generation){stop_camera();camera_error.value=t('ui.camera_unavailable_or_permission_denied_paste_the_sharing');}}
}
function hidden(){if(document.hidden)stop_camera();}
onMounted(()=>{document.addEventListener('visibilitychange',hidden);void action(()=>load());});
onBeforeUnmount(()=>{alive=false;stop_camera();document.removeEventListener('visibilitychange',hidden);});
</script>

<template>
  <section>
    <ModelSync v-if="sync_network&&!target" :key="sync_network.network_id" :network-id="sync_network.network_id" :network-name="sync_network.display_name" @close="sync_network=undefined"/>
    <div v-if="!mode" class="heading"><h2>{{t('ui.my_networks')}}</h2><button :disabled="busy" @click="action(()=>load())">{{t('ui.refresh')}}</button></div>
    <p class="muted">{{t('ui.you_can_save_and_share_multiple_networks_but')}}</p>
    <p v-if="error" class="error" role="alert">{{notice_text(error)}}</p>
    <NetworkForm v-if="mode!=='join'" :key="create_key" :busy="busy" :target="target" @save="create"/>
    <template v-if="!mode">
    <article v-for="network in networks" :key="network.network_id">
      <div><strong>{{network.display_name}}</strong><p>{{state_label(network.state)}} · {{network.network_id}}</p><p v-if="network.overlay_cidr">{{t('network_form.actual_subnet')}}：{{network.overlay_cidr}}</p><p v-if="network.last_error" class="error">{{notice_text((network.last_error.code?network.last_error.code+': ':'')+network.last_error.message)}}</p></div>
      <div class="actions"><button v-if="!target" :disabled="busy||network.state!=='running'" @click="sync_network=network">{{t('model_sync.title')}}</button><button :disabled="busy" @click="share(network.network_id)">{{t('ui.share')}}</button><button :disabled="busy" @click="network_action('start_network',network.network_id)">{{t('ui.start')}}</button><button :disabled="busy" @click="network_action('stop_network',network.network_id)">{{t('ui.stop')}}</button><button :disabled="busy" @click="edit(network)">{{t('ui.edit')}}</button><button :disabled="busy" @click="export_config(network.network_id)">{{t('ui.export_configuration')}}</button><button :disabled="busy" @click="remove(network)">{{t('ui.delete')}}</button></div>
    </article>
    <p v-if="!networks.length" class="muted">{{t('ui.no_networks_yet_create_one_or_import_a')}}</p>
    <button v-if="next_cursor" :disabled="busy" @click="action(()=>load(true))">{{t('ui.load_more_networks')}}</button>
    </template>
    <template v-if="mode!=='create'">
    <form @submit.prevent="join"><input v-model="url" type="url" required :placeholder="t('ui.sharing_url_including_its_decryption_key')"><button :disabled="busy">{{t('ui.import_configuration')}}</button></form>
    <button v-if="mobile&&!camera" :disabled="busy" @click="start_camera">{{t('ui.scan_qr_code')}}</button><button v-if="camera" @click="stop_camera">{{t('ui.close_camera')}}</button>
    <label v-if="!mobile">{{t('network_import.file')}}<input type="file" accept=".json,.txt,.url,application/json,text/plain" :disabled="busy" @change="read_file"/></label>
    <p v-if="file_notice" role="status">{{file_notice}}</p>
    <video v-if="camera" ref="video" muted playsinline :aria-label="t('ui.qr_code_camera_preview')"/>
    <p v-if="camera_error" class="error" role="alert">{{notice_text(camera_error)}}</p>
    <p class="muted">{{t('ui.scanned_links_use_the_same_url_import_confirm')}}</p>
    <details :open="manual_open" @toggle="manual_open=($event.target as HTMLDetailsElement).open"><summary>{{t('ui.manual_configuration_no_public_server_needed')}}</summary><form class="vertical" @submit.prevent="join_manual"><label>{{t('ui.complete_joinconfig_json_includes_network_credentials')}}<textarea v-model="manual" rows="7" required spellcheck="false" autocomplete="off"/></label><button :disabled="busy">{{t('ui.import_manual_configuration')}}</button></form></details>
    </template>
    <div v-if="shared" class="secret"><h3>{{t('ui.share_network')}}</h3><p>{{t('ui.anyone_with_this_link_or_qr_code_can')}}</p><div v-if="qr_svg" class="qr" role="img" :aria-label="t('ui.network_sharing_qr_code')" v-html="qr_svg"/><textarea readonly :value="shared"/><p>{{t('ui.expires')}} {{format_datetime(expires)}}{{t('ui.expiration_does_not_revoke_members_who_have_already')}}</p><button @click="shared='';qr_svg='';expires=''">{{t('ui.hide_sharing_details')}}</button></div>
    <div v-if="card" class="secret network-card"><h3>{{t('network_form.card')}}</h3><p>{{t('network_form.name')}}：{{card.display_name}} · {{card.easytier.network_name}}</p><p>{{t('network_form.subnet')}}：{{card.easytier.dhcp?t('network_form.automatic'):card.easytier.ipv4_cidr}}</p><label>{{t('network_form.key')}}<input readonly :type="card_secret?'text':'password'" :value="card.easytier.network_secret"/></label><button @click="card_secret=!card_secret">{{t(card_secret?'network_form.hide':'network_form.show')}}</button><p>{{t('network_form.peers')}}：{{card.easytier.bootstrap_peers.join(', ')||t('network_form.no_peers')}}</p><details><summary>{{t('ui.network_configuration_sensitive')}}</summary><textarea readonly :value="exported" rows="8"/></details><button @click="exported=''">{{t('ui.hide_configuration')}}</button></div>
    <div v-if="editing" class="secret"><h3>{{t('ui.edit_network_configuration_explicitly')}}</h3><NetworkForm :key="editing" :initial="edit_config" :display-name="edit_name" :local-ipv4="edit_local" :busy="busy" :target="target" editing @save="save_edit"/><button @click="editing=''">{{t('ui.discard_changes')}}</button></div>
    <p class="muted">{{t('ui.saved_configuration_does_not_mean_connected_easytier_startup')}}</p>
  </section>
</template>

<style scoped>
.heading{display:flex;align-items:center;justify-content:space-between}.actions{display:flex;flex-wrap:wrap;gap:6px}.actions button{font-size:12px;padding:7px 10px}article{flex-wrap:wrap;gap:10px}details{margin:18px 0}summary{cursor:pointer;color:#416852}textarea{width:100%;box-sizing:border-box}.qr{width:min(320px,100%);margin:16px auto}.qr :deep(svg){width:100%;display:block}video{width:min(480px,100%);display:block;margin:12px 0;border-radius:8px}
</style>
