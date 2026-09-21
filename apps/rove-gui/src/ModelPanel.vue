<script setup lang="ts">
import {t,notice_text,provider_label,format_number} from './i18n';
import {ref,computed,onMounted,watch} from 'vue';
import SecretInput from './components/SecretInput.vue';
import Button from 'primevue/button';
import Dialog from 'primevue/dialog';
import {call as agent_call,type Target,type Json} from './api';
import {model_presets} from './model-presets';
import {generated_names,type ModelCatalog,type ModelEntry,type ModelConnection} from './models';
const props=defineProps<{target?:Target;autoAdd?:boolean;formOnly?:boolean;requireTest?:boolean}>();
const emit=defineEmits<{changed:[];onboarding:[sessionId:string];saved:[];close:[]}>();
const catalog=ref<ModelCatalog>({connections:[],models:[],default_model_id:null});
const busy=ref(false),error=ref(''),notice=ref(''),adding=ref(false);
const provider=ref(model_presets[0].id),base_url=ref(model_presets[0].base_url),api_key=ref(''),manual=ref('');
const selected=ref<string[]>([...model_presets[0].models]),names=ref<Record<string,string>>({}),set_default=ref(true);
const preset=computed(()=>model_presets.find(p=>p.id===provider.value)!);
const fetched=ref<string[]|null>(null),listing_truncated=ref(false);
const listing_notice=computed(()=>fetched.value===null?'':t('ui.retrieved_value_model_variantsvalue_listing_does_not_guarantee',{p0:format_number(fetched.value.length),p1:listing_truncated.value?t('common.truncated'):''}));
const available=computed(()=>[...new Set([...(fetched.value??preset.value.models),...models.value])]);
const discovering=ref(false),discovery_error=ref('');
let discovery_epoch=0,last_discovery='';
watch([provider,base_url,api_key],()=>{discovery_epoch++;discovering.value=false;fetched.value=null;listing_truncated.value=false;discovery_error.value='';last_discovery='';},{flush:'sync'});
watch(adding,value=>{if(!value){discovery_epoch++;last_discovery='';}});
function auto_discover(){
  if(!adding.value||(!api_key.value.trim()&&provider.value!=='ollama'))return;
  const identity=JSON.stringify([provider.value,base_url.value,api_key.value]);
  if(identity===last_discovery)return;last_discovery=identity;void discover();
}
async function discover(){
  if(discovering.value)return;
  last_discovery=JSON.stringify([provider.value,base_url.value,api_key.value]);
  const epoch=++discovery_epoch;
  const input={provider:provider.value,base_url:base_url.value,api_key:api_key.value||null};
  discovering.value=true;discovery_error.value='';
  try{
    const result=await call('discover_models',input) as {models:string[];truncated:boolean};
    if(epoch!==discovery_epoch||!adding.value)return;
    const first=fetched.value===null,previous=new Set(available.value);
    fetched.value=result.models;
    selected.value=first?[...new Set([...result.models,...selected.value.filter(id=>names.value[id])])]:[...selected.value,...result.models.filter(id=>!previous.has(id))];
    listing_truncated.value=result.truncated;
  }catch{
    if(epoch===discovery_epoch)discovery_error.value=t('chat.listing_failed');
  }finally{if(epoch===discovery_epoch)discovering.value=false;}
}
const test_model_id=ref(''),tested=ref<Record<string,string>>({}),test_notice=ref('');
const test_choice=computed(()=>models.value.includes(test_model_id.value)?test_model_id.value:models.value[0]??'');
const has_tested=computed(()=>models.value.some(model=>!!tested.value[model]));
watch([provider,base_url,api_key],()=>{tested.value={};test_notice.value='';},{flush:'sync'});
async function test_model(model_id?:string){
  const config={provider:provider.value,base_url:base_url.value,api_key:api_key.value||null,model:test_choice.value};
  test_notice.value='';
  if(!model_id)delete tested.value[config.model];
  await work(async()=>{
    try {
      const result=await call('test_model',model_id?{model_id}:config) as {tested_at:string};
      if(!model_id&&config.provider===provider.value&&config.base_url===base_url.value&&config.api_key===(api_key.value||null))tested.value[config.model]=result.tested_at;
      test_notice.value=t('model_test.passed',{model:model_id?catalog.value.models.find(m=>m.model_id===model_id)?.model??'':config.model});
    } finally {await refresh();emit('changed');}
  });
}
function deepseek_protocol(event:Event){base_url.value=(event.target as HTMLSelectElement).value==='anthropic'?'https://api.deepseek.com/anthropic':'https://api.deepseek.com';}
const models=computed(()=>[...new Set([...selected.value,...manual.value.split(/[,，\n]/).map(s=>s.trim()).filter(Boolean)])]);
const generated=computed(()=>generated_names(provider.value,available.value,catalog.value.models.map(m=>m.name)));
const editing=ref<ModelEntry>(),edit_name=ref(''),removing=ref<ModelConnection>();
const connection=ref<ModelConnection>(),edit_url=ref(''),edit_key=ref('');
function call(operation:string,body?:Json,path?:Record<string,string>){return agent_call(operation,body,path,undefined,props.target);}
async function work(action:()=>Promise<void>){if(busy.value)return;busy.value=true;error.value='';try{await action();}catch(e){error.value=String(e);}finally{busy.value=false;}}
async function refresh(){catalog.value=await call('get_model_catalog') as unknown as ModelCatalog;}
async function mutate(operation:string,body?:Json,path?:Record<string,string>){const before=catalog.value.onboarding_session_id;catalog.value=await call(operation,body,path) as unknown as ModelCatalog;emit('changed');if(operation==='import_models'&&!before&&catalog.value.onboarding_session_id)emit('onboarding',catalog.value.onboarding_session_id);}
function change_provider(){base_url.value=preset.value.base_url;selected.value=[...preset.value.models];api_key.value='';manual.value='';names.value={};fetched.value=null;listing_truncated.value=false;}
function add(){adding.value=true;api_key.value='';tested.value={};test_notice.value='';set_default.value=!catalog.value.default_model_id;}
async function save(){if(props.requireTest&&!has_tested.value){error.value=t('model_test.required');return;}await work(async()=>{await mutate('import_models',{provider:provider.value,base_url:base_url.value,api_key:api_key.value||null,models:models.value.map(model=>({model,name:names.value[model]?.trim()||generated.value[available.value.indexOf(model)]})),set_default:set_default.value});api_key.value='';adding.value=false;names.value={};notice.value=t('ui.model_configuration_saved_availability_depends_on_the_provider');emit('saved');});}
function rename(model:ModelEntry){editing.value=model;edit_name.value=model.name;}
function edit_connection(value:ModelConnection){connection.value=value;edit_url.value=value.base_url;edit_key.value='';}
onMounted(async()=>{if(props.autoAdd)add();await work(refresh);if(props.autoAdd)set_default.value=!catalog.value.default_model_id;});
</script>
<template>
  <div class="model-panel">
    <template v-if="!formOnly">
    <div class="actions"><Button :label="t('ui.add_models')" icon="pi pi-plus" :disabled="busy" @click="add"/><Button :label="t('ui.refresh_models')" text :disabled="busy" @click="work(refresh)"/></div>
    <p v-if="error" class="error" role="alert">{{notice_text(error)}}</p><p v-if="notice" role="status">{{notice_text(notice)}}</p><p v-if="test_notice&&!adding" role="status">{{test_notice}}</p>
    <p v-if="!catalog.models.length" class="notice">{{t('ui.no_models_yet_add_one_api_connection_to')}}</p>
    <section v-for="c in catalog.connections" :key="c.connection_id"><h2>{{provider_label(c.provider)}}</h2><p class="model-url">{{c.base_url}}</p><p class="muted">{{c.api_key_configured?t('ui.api_key_saved_not_displayed'):t('ui.no_api_key_configured')}}</p>
      <div v-for="m in catalog.models.filter(m=>m.connection_id===c.connection_id)" :key="m.model_id" class="model-entry"><div><strong>{{m.name}}</strong><small>{{m.model}}</small><small>{{t(m.tested_at?'model_test.verified':'model_test.unverified')}}</small><span v-if="m.model_id===catalog.default_model_id">{{t('ui.default')}}</span></div><div class="actions"><Button :label="t('model_test.test')" :aria-label="t('model_test.test')+' '+m.name" outlined :disabled="busy" @click="test_model(m.model_id)"/><Button :label="t('ui.rename')" text :aria-label="t('ui.rename_value',{p0:(m.name)})" :disabled="busy" @click="rename(m)"/><Button v-if="m.model_id!==catalog.default_model_id" :label="t('ui.set_as_default')" text :aria-label="t('ui.set_as_default_value',{p0:(m.name)})" :disabled="busy" @click="work(()=>mutate('set_default_model',{model_id:m.model_id}))"/></div></div>
      <div class="actions"><Button :label="t('ui.update_connection')" outlined :disabled="busy" @click="edit_connection(c)"/><Button :label="t('ui.remove_connection')" text severity="danger" :disabled="busy" @click="removing=c"/></div>
    </section>
    <Button v-if="catalog.default_model_id" :label="t('ui.clear_default_model')" text :disabled="busy" @click="work(()=>mutate('set_default_model',{model_id:null}))"/>
    </template>
    <Dialog v-model:visible="adding" modal :closable="!busy" :close-on-escape="!busy" :header="t('ui.add_models')" :style="{width:'min(640px,94vw)'}" @hide="api_key=''" @update:visible="value=>{if(!value&&formOnly)emit('close')}">
      <form class="vertical" @submit.prevent="save" @focusout="auto_discover"><label>{{t('ui.provider')}}<select :aria-label="t('ui.provider')" v-model="provider" @change="change_provider"><option v-for="p in model_presets" :key="p.id" :value="p.id">{{provider_label(p.id)}}</option></select></label><label v-if="provider==='deepseek'">{{t('model_test.protocol')}}<select :aria-label="t('model_test.protocol')" :value="/\/anthropic(?:\/v1)?\/?$/.test(base_url)?'anthropic':'openai'" @change="deepseek_protocol"><option value="openai">OpenAI</option><option value="anthropic">Anthropic</option></select></label><label>{{t('ui.api_base_url')}}<input v-model="base_url" type="url" required></label><p v-if="provider==='deepseek'" class="muted">{{t('model_test.deepseek_hint')}}</p><label>{{t('ui.api_key')}}<SecretInput v-if="adding" v-model="api_key" :label="t('ui.api_key')"/></label><p class="muted">{{t('ui.account_login_adapters_are_not_connected_yet_there')}}</p>
        <fieldset class="model-test"><legend>{{t('model_test.test')}}</legend><p class="muted">{{t('model_test.hint')}}</p><label>{{t('model_test.variant')}}<select :value="test_choice" @change="test_model_id=($event.target as HTMLSelectElement).value" :aria-label="t('model_test.variant')" :disabled="busy"><option v-for="model in models" :key="model" :value="model">{{model}}</option></select></label><Button :label="t(busy?'model_test.testing':'model_test.test')" icon="pi pi-bolt" outlined :disabled="busy||!test_choice" @click="test_model()"/><p v-if="test_notice" role="status">{{test_notice}}</p><p v-if="requireTest&&!has_tested" class="muted">{{t('model_test.required')}}</p><p v-if="error" class="error" role="alert">{{notice_text(error)}}</p></fieldset>
        <p class="muted">{{t('chat.auto_models')}}</p><Button :label="t(discovering?'chat.listing':'ui.fetch_available_models')" outlined :disabled="busy||discovering" @click="discover"/><p v-if="discovery_error" role="status" class="muted">{{discovery_error}}</p><p v-if="listing_notice" role="status">{{listing_notice}}</p><fieldset class="model-choices"><legend>{{t('ui.select_models_all_selected_by_default')}}</legend><p class="muted">{{t('chat.model_names')}}</p><div v-for="(model,index) in available" :key="model" class="model-choice"><label class="check-row"><input v-model="selected" type="checkbox" :value="model">{{model}}</label><label v-if="models.includes(model)" class="model-choice-name">{{t('ui.configuration_name')}}<input :value="names[model]??generated[index]" :aria-label="model+' '+t('ui.configuration_name')" maxlength="256" required @input="names[model]=($event.target as HTMLInputElement).value"></label></div></fieldset><p class="muted">{{fetched?t('ui.live_provider_list_not_every_model_supports_conversations'):t('ui.local_reference_list_from_the_2026_09_16')}}{{t('ui.you_can_enter_actual_model_ids_manually_add')}}</p>
        <label>{{t('ui.add_model_ids_manually')}}<input v-model="manual" :placeholder="t('ui.separate_multiple_model_ids_with_commas')"></label>
        <label class="check-row"><input v-model="set_default" type="checkbox">{{t('ui.make_the_first_model_in_this_batch_the')}}</label><div class="actions"><Button :label="t('ui.cancel')" outlined :disabled="busy" @click="adding=false;api_key='';if(formOnly)emit('close')"/><Button :label="t('ui.save_models')" type="submit" :disabled="busy||!models.length||models.length>100||(requireTest&&!has_tested)"/></div>
      </form>
    </Dialog>
    <Dialog :visible="!!editing" modal :header="t('ui.rename_model_configuration')" :style="{width:'min(440px,94vw)'}" @update:visible="editing=undefined"><form class="vertical" @submit.prevent="work(async()=>{await mutate('rename_model',{name:edit_name},{model_id:editing!.model_id});editing=undefined;})"><label>{{t('ui.model_name')}}<input v-model="edit_name" required maxlength="256"></label><p v-if="error" class="error">{{notice_text(error)}}</p><Button :label="t('ui.save_name')" type="submit" :disabled="busy"/></form></Dialog>
    <Dialog :visible="!!connection" modal :header="t('ui.update_connection')" :style="{width:'min(520px,94vw)'}" @update:visible="connection=undefined;edit_key=''" @hide="edit_key=''">
      <form class="vertical" @submit.prevent="work(async()=>{await mutate('update_model_connection',{provider:connection!.provider,base_url:edit_url,api_key:edit_key||null},{connection_id:connection!.connection_id});connection=undefined;edit_key='';})"><p>{{t('ui.changes_affect_all_models_sharing_this_connection_running')}}</p><label>{{t('ui.api_base_url')}}<input v-model="edit_url" type="url" required></label><label>{{t('ui.new_api_key_leave_blank_to_clear_the')}}<SecretInput v-if="connection" v-model="edit_key" :label="t('ui.new_api_key_leave_blank_to_clear_the')"/></label><p v-if="error" class="error">{{notice_text(error)}}</p><Button :label="t('ui.save_connection')" type="submit" :disabled="busy"/></form>
    </Dialog>
    <Dialog :visible="!!removing" modal :header="t('ui.remove_model_connection')" :style="{width:'min(440px,94vw)'}" @update:visible="removing=undefined"><p>{{t('ui.remove_this_connection_s_key_and_all_its')}}</p><p v-if="error" class="error">{{notice_text(error)}}</p><template #footer><Button :label="t('ui.cancel')" outlined :disabled="busy" @click="removing=undefined"/><Button :label="t('ui.confirm_removal')" severity="danger" :disabled="busy" @click="work(async()=>{await mutate('delete_model_connection',undefined,{connection_id:removing!.connection_id});removing=undefined;})"/></template></Dialog>
  </div>
</template>
<style scoped>
.model-choice{padding:.5rem 0;border-bottom:1px solid var(--line)}.model-choice-name{margin:.6rem 0 0 1.6rem;color:var(--muted)}.model-entry{display:flex;align-items:center;justify-content:space-between;gap:1rem;padding:1rem 0;border-bottom:1px solid var(--border,#ddd);flex-wrap:wrap}.model-entry strong,.model-entry small{display:block;overflow-wrap:anywhere}.model-entry span{font-size:.8rem;color:var(--p-primary-color)}.model-url{overflow-wrap:anywhere}.check-row{display:flex;align-items:center;gap:.7rem}.check-row input{width:auto;flex:none}fieldset{display:grid;gap:.7rem}select{width:100%;min-height:42px}
</style>
