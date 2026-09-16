<script setup lang="ts">
import { ref, shallowRef, onMounted, computed } from 'vue';
import { call as agent_call, type Json, type Target } from './api';
import ChatPanel from './ChatPanel.vue';
import NetworkPanel from './NetworkPanel.vue';
import ServicePanel from './ServicePanel.vue';
import DevicePicker from './DevicePicker.vue';
const device = shallowRef<Json>(null);
const target=shallowRef<Target>(),network_id=ref(''),device_id=ref('');
const target_key=computed(()=>target.value?`${target.value.network_id}/${target.value.device_id}`:'local');
async function choose_discovered(value:Target){network_id.value=value.network_id;device_id.value=value.device_id;await choose_target();}
function call(...args:Parameters<typeof agent_call>){return agent_call(args[0],args[1],args[2],args[3],target.value);}
async function choose_target(local=false){
  if(busy.value)return;
  if(!local){const uuid=/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;if(!uuid.test(network_id.value)||!uuid.test(device_id.value)){error.value='请输入完整的网络和设备 UUID';return;}}
  target.value=local?undefined:{network_id:network_id.value,device_id:device_id.value};
  device.value=null;api_key.value='';model.value='';base_url.value='';config_server_url.value='';
  await work(refresh);
}
const error = ref(''), busy = ref(false);
const max_active_runs = ref(4), config_server_url = ref('');
const provider = ref('openai_compatible'), base_url = ref('http://127.0.0.1:11434/v1'), model = ref(''), api_key = ref('');
async function work(action:()=>Promise<void>) {
  busy.value=true; error.value='';
  try { await action(); } catch(e) { error.value = e instanceof Error ? e.message : '操作失败'; }
  finally { busy.value=false; }
}
async function refresh() {
  device.value=await call('get_device');
  const settings=await call('get_settings') as {max_active_runs:number;config_server_url:string|null};
  max_active_runs.value=settings.max_active_runs; config_server_url.value=settings.config_server_url??'';
  const configured=await call('get_model_config') as {config:null|{provider:string;base_url:string;model:string}};
  provider.value=configured.config?.provider??'openai_compatible';base_url.value=configured.config?.base_url??'';model.value=configured.config?.model??'';
}
async function save_settings() { await work(async()=>{await call('update_settings',{max_active_runs:max_active_runs.value,config_server_url:config_server_url.value||null});}); }
async function save_model() { await work(async()=>{await call('set_model_config',{provider:provider.value,base_url:base_url.value,model:model.value,api_key:api_key.value||null});api_key.value='';}); }
onMounted(()=>work(refresh));
</script>

<template>
  <div class="shell">
    <aside><div class="brand">rove<span>漫游者</span></div><nav>设备与网络</nav><p class="note">每台设备都是对等节点。<br>加入网络即信任网络中的所有成员。</p></aside>
    <main>
      <header><div><small>YOUR PERSONAL DEVICE SPACE</small><h1>设备与网络</h1></div><button :disabled="busy" @click="work(refresh)">刷新</button></header>
      <p v-if="error" class="error" role="alert">{{error}}</p>
      <p v-if="device && ['android','ios'].includes(String((device as Record<string,Json>).os))" class="note">移动开发版：本机 agent 已嵌入；native VPN、后台常驻和本机命令尚未提供。关闭应用后的运行受系统限制。</p>
      <section><h2>执行目标</h2><p>{{target?`设备 ${target.device_id} · 网络 ${target.network_id}`:'本机 rove-agent'}}</p><form class="vertical" @submit.prevent="choose_target()"><label>网络 ID<input v-model="network_id" required></label><label>设备 ID<input v-model="device_id" required></label><div><button :disabled="busy">选择远端设备</button> <button type="button" :disabled="busy" @click="choose_target(true)">回到本机</button></div></form><p class="muted">远端不可达时明确报错，不会退回本机执行。当前构建尚未接通 overlay 发现与路由。</p></section>
      <section><DevicePicker :disabled="busy" @choose="choose_discovered" /></section>
      <section><h2>目标设备状态</h2><pre v-if="device">{{JSON.stringify(device,null,2)}}</pre><p v-else>未连接。请检查本机 agent 和所选目标。</p></section>
      <ChatPanel :key="`chat/${target_key}`" :target="target" />
      <NetworkPanel :key="`network/${target_key}`" :target="target" />
      <ServicePanel :key="`services/${target_key}`" :target="target" />
      <section><h2>设备设置</h2><form class="vertical" @submit.prevent="save_settings"><label>跨会话并发上限<input v-model.number="max_active_runs" type="number" min="1" required></label><label>密文配置服务器<input v-model="config_server_url" type="url" placeholder="https://config.example.com"></label><button :disabled="busy">保存设置</button></form></section>
      <section><h2>目标设备默认模型</h2><form class="vertical" @submit.prevent="save_model"><label>提供方<input v-model="provider" required></label><label>接口地址<input v-model="base_url" type="url" required></label><label>模型<input v-model="model" required></label><label>API key（留空表示无需密钥，全量替换）<input v-model="api_key" type="password" autocomplete="off"></label><button :disabled="busy">保存模型配置</button><button type="button" :disabled="busy" @click="work(async()=>{await call('clear_model_config');api_key='';await refresh();})">清除模型配置</button></form></section>
    </main>
  </div>
</template>
