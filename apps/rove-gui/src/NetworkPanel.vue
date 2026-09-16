<script setup lang="ts">
import {ref, shallowRef, onMounted, onBeforeUnmount, nextTick} from 'vue';
import {invoke} from '@tauri-apps/api/core';
import jsQR from 'jsqr';
import {call as agent_call, type Json, type Target} from './api';
const props=defineProps<{target?:Target}>();
function call(...args:Parameters<typeof agent_call>){return agent_call(args[0],args[1],args[2],args[3],props.target);}
interface Network {network_id:string;display_name:string;state:string;last_error:null|{message:string}}
const networks=shallowRef<Network[]>([]), next_cursor=ref<string|null>(null);
const name=ref(''),bootstrap=ref(''),url=ref(''),manual=ref(''),error=ref(''),busy=ref(false);
const shared=ref(''),expires=ref(''),qr_svg=ref(''),exported=ref(''),editing=ref(''),replacement=ref('');
const camera=ref(false),video=ref<HTMLVideoElement>(),camera_error=ref('');
let media:MediaStream|undefined,frame:number|undefined,alive=true,camera_generation=0;
async function action(task:()=>Promise<void>){busy.value=true;error.value='';try{await task();}catch(e){error.value=e instanceof Error?e.message:'操作失败';}finally{busy.value=false;}}
async function load(append=false){const page=await call('list_networks',undefined,undefined,append&&next_cursor.value?{cursor:next_cursor.value}:{}) as unknown as {items:Network[];next_cursor:string|null};if(!alive)return;networks.value=append?[...networks.value,...page.items]:page.items;next_cursor.value=page.next_cursor;}
async function create(){await action(async()=>{const peers=bootstrap.value.split(/\s+/).filter(Boolean);await call('create_network',{display_name:name.value,...(peers.length?{bootstrap_peers:peers}:{})});name.value='';bootstrap.value='';await load();});}
async function join(){await action(async()=>{await call('import_network',{source:'url',url:url.value.trim()});url.value='';await load();});}
async function join_manual(){await action(async()=>{let config:Json;try{config=JSON.parse(manual.value);}catch{throw new Error('入网配置不是有效的 JSON');}await call('import_network',{source:'manual',config});manual.value='';await load();});}
async function share(id:string){await action(async()=>{const result=await call('create_network_share',{}, {network_id:id}) as {url:string;expires_at:string};shared.value=result.url;expires.value=result.expires_at;qr_svg.value='';qr_svg.value=await invoke<string>('share_qr',{url:result.url});});}
async function network_action(operation:string,id:string){await action(async()=>{await call(operation,undefined,{network_id:id});await load();});}
async function remove(network:Network){if(!window.confirm(`删除本机的“${network.display_name}”配置？不会卸载应用或撤销其他成员。`))return;await network_action('delete_network',network.network_id);}
async function export_config(id:string){await action(async()=>{exported.value=JSON.stringify(await call('get_network_join_config',undefined,{network_id:id}),null,2);});}
async function edit(network:Network){await action(async()=>{const config=await call('get_network_join_config',undefined,{network_id:network.network_id}) as Record<string,Json>;editing.value=network.network_id;replacement.value=JSON.stringify({display_name:config.display_name,easytier:config.easytier},null,2);});}
async function save_edit(){await action(async()=>{let config:Json;try{config=JSON.parse(replacement.value);}catch{throw new Error('修改后的配置不是有效 JSON');}await call('update_network',config,{network_id:editing.value});editing.value='';replacement.value='';await load();});}
function stop_camera(){camera_generation++;camera.value=false;if(frame!==undefined)cancelAnimationFrame(frame);frame=undefined;media?.getTracks().forEach(track=>track.stop());media=undefined;}
async function start_camera(){
  stop_camera();const generation=camera_generation;camera_error.value='';
  if(!navigator.mediaDevices?.getUserMedia){camera_error.value='当前平台未提供相机接口，请粘贴分享 URL。';return;}
  camera.value=true;
  try{
    const stream=await navigator.mediaDevices.getUserMedia({video:{facingMode:{ideal:'environment'}},audio:false});
    if(!alive||generation!==camera_generation){stream.getTracks().forEach(track=>track.stop());return;}
    media=stream;await nextTick();if(!video.value){stop_camera();return;}video.value.srcObject=media;await video.value.play();
    if(!alive||generation!==camera_generation)return;
    const canvas=document.createElement('canvas');const context=canvas.getContext('2d',{willReadFrequently:true});
    if(!context)throw new Error('无法读取相机图像，请使用 URL 导入');
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
      } catch { stop_camera();camera_error.value='无法解析相机图像，请粘贴分享 URL。'; }
    };
    frame=requestAnimationFrame(scan);
  }catch{if(generation===camera_generation){stop_camera();camera_error.value='相机不可用或授权被拒绝，请粘贴分享 URL。';}}
}
onMounted(()=>action(()=>load()));
onBeforeUnmount(()=>{alive=false;stop_camera();});
</script>

<template>
  <section>
    <div class="heading"><h2>我的网络</h2><button :disabled="busy" @click="action(()=>load())">刷新</button></div>
    <p class="muted">可保存和分享多个网络；每台设备同时只加入一个。创建或导入仅保存配置，切换前请先停止当前网络，再启动另一个。</p>
    <p v-if="error" class="error" role="alert">{{error}}</p>
    <form @submit.prevent="create"><input v-model="name" required maxlength="128" placeholder="网络名称"><button :disabled="busy">创建网络</button></form>
    <details><summary>可选：初始连接节点</summary><textarea v-model="bootstrap" placeholder="EasyTier bootstrap URL，每行一个" rows="2"/></details>
    <article v-for="network in networks" :key="network.network_id">
      <div><strong>{{network.display_name}}</strong><p>{{network.state}} · {{network.network_id}}</p><p v-if="network.last_error" class="error">{{network.last_error.message}}</p></div>
      <div class="actions"><button :disabled="busy" @click="share(network.network_id)">分享</button><button :disabled="busy" @click="network_action('start_network',network.network_id)">启动</button><button :disabled="busy" @click="network_action('stop_network',network.network_id)">停止</button><button :disabled="busy" @click="edit(network)">修改</button><button :disabled="busy" @click="export_config(network.network_id)">导出配置</button><button :disabled="busy" @click="remove(network)">删除</button></div>
    </article>
    <p v-if="!networks.length" class="muted">还没有网络。创建一个，或导入分享的配置。</p>
    <button v-if="next_cursor" :disabled="busy" @click="action(()=>load(true))">加载更多网络</button>
    <form @submit.prevent="join"><input v-model="url" type="url" required placeholder="含密钥的分享 URL"><button :disabled="busy">导入配置</button></form>
    <button v-if="!camera" :disabled="busy" @click="start_camera">扫描二维码</button><button v-else @click="stop_camera">关闭相机</button>
    <video v-if="camera" ref="video" muted playsinline aria-label="二维码扫描画面"/>
    <p v-if="camera_error" class="error" role="alert">{{camera_error}}</p>
    <p class="muted">扫描结果填入同一 URL 入口，确认后导入。相机图像只在本机处理。</p>
    <details><summary>手动配置（无需公网服务）</summary><form class="vertical" @submit.prevent="join_manual"><label>完整 JoinConfig JSON（包含入网凭据）<textarea v-model="manual" rows="7" required spellcheck="false" autocomplete="off"/></label><button :disabled="busy">导入手动配置</button></form></details>
    <div v-if="shared" class="secret"><h3>分享网络</h3><p>持有此链接或二维码的人可加入并操作网络内设备，请勿公开。</p><div v-if="qr_svg" class="qr" role="img" aria-label="网络分享二维码" v-html="qr_svg"/><textarea readonly :value="shared"/><p>有效期至 {{expires}}；到期不撤销已经加入的成员。</p><button @click="shared='';qr_svg='';expires=''">隐藏分享</button></div>
    <div v-if="exported" class="secret"><label>入网配置（敏感）<textarea readonly :value="exported" rows="8"/></label><button @click="exported=''">隐藏配置</button></div>
    <form v-if="editing" class="vertical secret" @submit.prevent="save_edit"><label>显式修改网络配置<textarea v-model="replacement" rows="8" required spellcheck="false"/></label><button :disabled="busy">保存修改</button><button type="button" @click="editing='';replacement=''">放弃修改</button></form>
    <p class="muted">配置已保存不代表已联网。EasyTier 正式启动适配仍在开发，不可用操作会明确报错。</p>
  </section>
</template>

<style scoped>
.heading{display:flex;align-items:center;justify-content:space-between}.actions{display:flex;flex-wrap:wrap;gap:6px}.actions button{font-size:12px;padding:7px 10px}article{flex-wrap:wrap;gap:10px}details{margin:18px 0}summary{cursor:pointer;color:#416852}textarea{width:100%;box-sizing:border-box}.qr{width:min(320px,100%);margin:16px auto}.qr :deep(svg){width:100%;display:block}video{width:min(480px,100%);display:block;margin:12px 0;border-radius:8px}
</style>
