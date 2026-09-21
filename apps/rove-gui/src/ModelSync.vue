<script setup lang="ts">
import {computed,onBeforeUnmount,onMounted,ref} from 'vue';
import Button from 'primevue/button';
import Dialog from 'primevue/dialog';
import DevicePicker from './DevicePicker.vue';
import {call,type Target} from './api';
import {t,notice_text,provider_label} from './i18n';
import type {ModelCatalog} from './models';
const props=defineProps<{networkId:string;networkName:string}>();
const emit=defineEmits<{close:[];synced:[]}>();
const local=ref<ModelCatalog>({connections:[],models:[],default_model_id:null});
const remote=ref<ModelCatalog>({connections:[],models:[],default_model_id:null});
const source_id=ref(''),replacement=ref(''),confirmed=ref(false),busy=ref(false),error=ref(''),success=ref('');
const destination=ref<Target>();
const initialized=ref(false);
const eligible=computed(()=>local.value.connections.filter(c=>c.api_key_configured));
const models=computed(()=>local.value.models.filter(m=>m.connection_id===source_id.value));
let alive=true;
onBeforeUnmount(()=>{alive=false;});
async function choose(target:Target){
  if(target.network_id!==props.networkId)return;
  if(busy.value)return;busy.value=true;error.value='';success.value='';destination.value=undefined;replacement.value='';confirmed.value=false;
  try{
    const value=await call('get_model_catalog',undefined,undefined,undefined,target) as unknown as ModelCatalog;
    if(alive){remote.value=value;destination.value={...target};}
  }catch(e){if(alive)error.value=String(e);}finally{if(alive)busy.value=false;}
}
async function sync(){
  if(busy.value||!source_id.value||!destination.value||destination.value.network_id!==props.networkId||(replacement.value&&!confirmed.value))return;
  busy.value=true;error.value='';success.value='';
  const target={...destination.value},id=source_id.value;
  try{
    // Source is always the local agent. The browser never fetches stored keys.
    const value=await call('sync_model_connection',{target,overwrite:confirmed.value,...(replacement.value?{replace_connection_id:replacement.value}:{})},{connection_id:id}) as unknown as ModelCatalog;
    if(alive){remote.value=value;success.value=t('model_sync.success');confirmed.value=false;replacement.value='';emit('synced');}
  }catch(e){if(alive)error.value=String(e);}finally{if(alive)busy.value=false;}
}
onMounted(async()=>{
  busy.value=true;
  try{const value=await call('get_model_catalog') as unknown as ModelCatalog;if(alive){local.value=value;source_id.value=eligible.value[0]?.connection_id??'';}}
  catch(e){if(alive)error.value=String(e);}finally{if(alive)busy.value=false;}
  if(alive)initialized.value=true;
});
</script>
<template>
  <Dialog :visible="true" modal :header="t('model_sync.title')" :closable="!busy" :close-on-escape="!busy" :style="{width:'min(680px,94vw)'}" @update:visible="emit('close')">
    <div class="vertical">
      <p>{{t('model_sync.hint')}}</p>
      <p>{{t('ui.networks')}}: {{networkName}}</p>
      <label>{{t('model_sync.source')}}<select v-model="source_id" :disabled="busy" @change="confirmed=false;replacement='';success=''">
        <option value="">{{t('model_sync.select')}}</option>
        <option v-for="c in eligible" :key="c.connection_id" :value="c.connection_id">{{provider_label(c.provider)}} · {{local.models.find(m=>m.connection_id===c.connection_id)?.name}} · {{c.base_url}}</option>
      </select></label>
      <p v-if="!busy&&!eligible.length" class="notice">{{t('model_sync.empty')}}</p>
      <ul><li v-for="m in models" :key="m.model_id">{{m.name}} · {{m.model}}</li></ul>
      <DevicePicker v-if="initialized" :network-id="networkId" :disabled="busy" @choose="choose"/>
      <template v-if="destination">
        <p class="sync-target">{{t('model_sync.target')}}: {{destination.device_id}}<br>{{t('ui.networks')}}: {{destination.network_id}}</p>
        <label>{{t('model_sync.replace')}}<select v-model="replacement" :disabled="busy" @change="confirmed=false;success=''">
          <option value="">{{t('model_sync.automatic')}}</option>
          <option v-for="c in remote.connections" :key="c.connection_id" :value="c.connection_id">{{provider_label(c.provider)}} · {{remote.models.filter(m=>m.connection_id===c.connection_id).map(m=>m.name).join(', ')}}</option>
        </select></label>
        <label class="check-row"><input v-model="confirmed" type="checkbox" :disabled="busy">{{t('model_sync.confirm')}}</label>
      </template>
      <p v-if="error" role="alert" class="error">{{notice_text(error)}}</p>
      <p v-if="success" role="status">{{success}}</p>
      <div class="actions"><Button :label="t('ui.cancel')" outlined :disabled="busy" @click="emit('close')"/><Button :label="t('model_sync.send')" :loading="busy" :disabled="busy||!source_id||!destination||!!replacement&&!confirmed" @click="sync"/></div>
    </div>
  </Dialog>
</template>
<style scoped>
select{width:100%;min-width:0;min-height:42px}.sync-target,li{overflow-wrap:anywhere}.check-row{display:flex;align-items:center;gap:.6rem}.check-row input{width:auto;flex:none}
</style>
