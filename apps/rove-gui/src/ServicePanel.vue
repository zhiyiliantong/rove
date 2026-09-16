<script setup lang="ts">
import {ref,shallowRef,onBeforeUnmount} from 'vue';
import {call as agent_call,type Target,type Json} from './api';
import {openUrl} from '@tauri-apps/plugin-opener';
import {browser_address,can_open} from './service-address';
interface Service {service_id:string;network_id:string;name:string;protocol:string;target:{host:string;port:number};listen_port:number;access_info:string;state:string;endpoints:string[];target_status:string;last_error:null|{message:string}}
const props=defineProps<{target?:Target}>();
const network=ref(props.target?.network_id??''),items=shallowRef<Service[]>([]),cursor=ref<string|null>(null);
const busy=ref(false),error=ref(''),editing=ref(''),name=ref(''),protocol=ref('http'),host=ref('127.0.0.1'),port=ref<number>(),listen_port=ref<number>(),access_info=ref(''),revealed=ref('');
let alive=true;
function call(...args:Parameters<typeof agent_call>){return agent_call(args[0],args[1],args[2],args[3],props.target);}
async function action(task:()=>Promise<void>){if(busy.value)return;busy.value=true;error.value='';try{await task();}catch(e){if(alive)error.value=e instanceof Error?e.message:'服务操作失败';}finally{busy.value=false;}}
async function load(append=false){const id=network.value;const page=await call('list_services',undefined,undefined,{network_id:id,...(append&&cursor.value?{cursor:cursor.value}:{})}) as unknown as {items:Service[];next_cursor:string|null};if(!alive||id!==network.value)return;items.value=append?[...items.value,...page.items]:page.items;cursor.value=page.next_cursor;}
function clear(){editing.value='';name.value='';protocol.value='http';host.value='127.0.0.1';port.value=undefined;listen_port.value=undefined;access_info.value='';}
function edit(service:Service){editing.value=service.service_id;network.value=service.network_id;name.value=service.name;protocol.value=service.protocol;host.value=service.target.host;port.value=service.target.port;listen_port.value=service.listen_port;access_info.value=service.access_info;}
async function save(){await action(async()=>{const body:Json={network_id:network.value,name:name.value,protocol:protocol.value,target:{host:host.value,port:port.value!},access_info:access_info.value,...(listen_port.value?{listen_port:listen_port.value}:{})};await call(editing.value?'update_service':'publish_service',body,editing.value?{service_id:editing.value}:undefined);if(!alive)return;clear();await load();});}
async function remove(service:Service){if(!window.confirm(`取消发布“${service.name}”？只关闭 Rove 代理，不卸载应用或删除数据。`))return;await action(async()=>{await call('unpublish_service',undefined,{service_id:service.service_id});if(!alive)return;if(editing.value===service.service_id)clear();revealed.value='';await load();});}
async function copy(address:string){await action(async()=>{if(!navigator.clipboard)throw new Error('剪贴板不可用，请手动复制地址');await navigator.clipboard.writeText(address);});}
async function open(address:string){await action(()=>openUrl(browser_address(address)));}
onBeforeUnmount(()=>{alive=false;access_info.value='';});
</script>
<template>
  <section>
    <h2>这台设备提供的服务</h2>
    <p class="muted">服务目录属于所选设备。应用认证由应用自身处理；Rove 仅保存说明并转发 TCP。</p>
    <p v-if="error" class="error" role="alert">{{error}}</p>
    <form @submit.prevent="action(()=>load())"><input v-model="network" required :disabled="busy||!!editing" placeholder="所属网络 UUID" @input="items=[];cursor=null;revealed=''"/><button :disabled="busy||!network">查看服务</button></form>
    <article v-for="service in items" :key="service.service_id" class="service-card">
      <div><h3>{{service.name}}</h3><p>{{service.state}} · 目标 {{service.target_status}}</p><small>{{service.service_id}}</small><p v-if="service.last_error" class="error">{{service.last_error.message}}</p></div>
      <div v-for="address in service.endpoints" :key="address" class="address"><input readonly :value="address" aria-label="服务访问地址"/><button v-if="can_open(address)" :disabled="busy" @click="open(address)">浏览器打开</button><button :disabled="busy" @click="copy(address)">复制地址</button></div>
      <p v-if="!service.endpoints.length" class="muted">当前没有可用入口，保存的定义不表示代理已运行。</p>
      <div class="actions"><button :disabled="busy" @click="edit(service)">修改发布</button><button @click="revealed=revealed===service.service_id?'':service.service_id">{{revealed===service.service_id?'隐藏认证说明':'查看认证说明'}}</button><button :disabled="busy" @click="remove(service)">取消发布</button></div>
      <pre v-if="revealed===service.service_id" class="secret">{{service.access_info||'未填写应用认证说明'}}</pre>
    </article>
    <button v-if="cursor" :disabled="busy" @click="action(()=>load(true))">加载更多</button>
    <h3>{{editing?'修改发布（保留服务 ID 和网络）':'发布已有本机服务'}}</h3>
    <form class="vertical" @submit.prevent="save">
      <label>名称<input v-model="name" required maxlength="128"/></label>
      <label>应用协议<select v-model="protocol"><option>http</option><option>https</option><option>tcp</option></select></label>
      <label>本机回环地址<select v-model="host"><option>127.0.0.1</option><option>::1</option></select></label>
      <label>目标端口<input v-model.number="port" type="number" min="1" max="65535" required/></label>
      <label>代理端口（首次留空自动分配）<input v-model.number="listen_port" type="number" min="1" max="65535"/></label>
      <label>应用认证说明（网络成员可查看）<textarea v-model="access_info" rows="3" maxlength="65536" autocomplete="off" spellcheck="false"/></label>
      <div><button :disabled="busy||!network">{{editing?'保存修改':'发布服务'}}</button> <button type="button" :disabled="busy" @click="clear">清空表单</button></div>
    </form>
    <p class="muted">正式发布需要已运行的 overlay 网络；当前适配未接通时会明确报错，不会开放物理网卡端口。</p>
  </section>
</template>
<style scoped>
.service-card{display:block;padding:18px 0}.service-card h3{margin:0}.actions,.address{display:flex;flex-wrap:wrap;gap:8px;margin:12px 0}.actions button{font-size:12px}.address input{flex:1;min-width:180px}.secret{white-space:pre-wrap;overflow-wrap:anywhere}select{padding:10px;border:1px solid #ccd7ca;border-radius:7px;background:#fafcf9}small{overflow-wrap:anywhere}
</style>
