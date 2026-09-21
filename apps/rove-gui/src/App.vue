<script setup lang="ts">
import {show_suggestions,preference_error} from './chat-preferences';
import {t,ui_locale,locale_preference,set_locale,notice_text} from './i18n';
import {usePrimeVue} from 'primevue/config';
import {primevue_locale} from './primevue-locales';
const primevue=usePrimeVue();
watch(ui_locale,()=>{primevue.config.locale=primevue_locale();},{immediate:true});
import { ref, shallowRef, onMounted, onBeforeUnmount, computed, watch, nextTick } from 'vue';
import { call as agent_call, type Json, type Target } from './api';
import Button from 'primevue/button';
import { page_from_hash } from './navigation';
import ChatPanel from './ChatPanel.vue';
import ConversationTitle from './components/ConversationTitle.vue';
import IntroductionPage from './components/IntroductionPage.vue';
import type {ModelCatalog} from './models';
import {load_introduction,save_introduction} from './introduction';
import './introduction.css';
const intro_storage=(()=>{try{return localStorage;}catch{return undefined;}})();
const introduction=ref(load_introduction(intro_storage));
const intro_notice=ref(''),intro_starting=ref(false);
const showing_introduction=computed(()=>!introduction.value.completed);
function intro_step(step:number){introduction.value.step=step;if(!save_introduction(intro_storage,introduction.value))intro_notice.value=t('intro.storage_error');}
async function verified_intro_model(){
  try {
    const catalog=await agent_call('get_model_catalog') as unknown as ModelCatalog;
    const verified=catalog.models.find(m=>m.model_id===catalog.default_model_id&&m.tested_at)??catalog.models.find(m=>m.tested_at);
    if(!verified){intro_notice.value=t('model_test.required');return false;}
    if(catalog.default_model_id!==verified.model_id){await agent_call('set_default_model',{model_id:verified.model_id});model_revision.value++;}
    return true;
  } catch(e){intro_notice.value=notice_text(String(e));return false;}
}
async function finish_intro(){
  if(!await verified_intro_model())return false;
  const updated={...introduction.value,completed:true,reset_pending:false};
  if(!save_introduction(intro_storage,updated)){intro_notice.value=t('intro.storage_error');return false;}
  introduction.value=updated;intro_notice.value='';navigate('sessions');return true;
}
function reset_intro(){
  const pending={...introduction.value,reset_pending:true};
  intro_notice.value=save_introduction(intro_storage,pending)?t('intro.reset_done'):t('intro.storage_error');
}
async function start_intro(prompt:string){
  if(intro_starting.value)return;intro_starting.value=true;
  try{
    if(!await verified_intro_model())return;
    if(target.value)await choose_target(true);
    if(target.value||error.value){intro_notice.value=error.value||t('ui.operation_failed');return;}
    navigate('sessions');await nextTick();
    const id=prompt?await chat.value?.prepare_prompt(prompt):await chat.value?.create_session();
    if(id)await finish_intro();else intro_notice.value=t('ui.operation_failed');
  } finally {intro_starting.value=false;}
}
import ModelPanel from './ModelPanel.vue';
const model_revision=ref(0);
import NetworkPanel from './NetworkPanel.vue';
import ServicePanel from './ServicePanel.vue';
import DevicePicker from './DevicePicker.vue';
const page=ref(page_from_hash(location.hash));
const menu=ref(false),dark=ref(false);
const links=computed(()=>[{id:'sessions',label:t('ui.conversations'),icon:'pi-comments'},{id:'services',label:t('ui.services'),icon:'pi-th-large'},{id:'networks',label:t('ui.networks'),icon:'pi-share-alt'}]);
function navigate(value:string){location.hash=`/${value}`;page.value=page_from_hash(location.hash);menu.value=false;}
function hash_changed(){page.value=page_from_hash(location.hash);}
watch(dark,value=>{document.documentElement.classList.toggle('dark',value);try{localStorage.setItem('rove-gui-theme',value?'dark':'light');}catch{}});
onMounted(()=>{window.addEventListener('hashchange',hash_changed);try{dark.value=localStorage.getItem('rove-gui-theme')==='dark';}catch{}});
onBeforeUnmount(()=>window.removeEventListener('hashchange',hash_changed));
const chat=ref<InstanceType<typeof ChatPanel>>();
const chat_view=ref({title:'',session_id:'',selected:false,detail:false,archived:false,busy:false,active:false});
async function rename_chat(id:string,title:string){if(!chat.value)throw Error(t('ui.operation_failed'));await chat.value.rename_title(id,title);}
const chat_page=computed(()=>page.value==='sessions'||page.value==='archives');
async function open_onboarding(id:string){if(showing_introduction.value)return;navigate('sessions');await nextTick();if(target.value)await chat.value?.create_session();else await chat.value?.open_session(id);}
async function configure_models(value?:Target){target.value=value;await work(refresh);navigate('models');}
async function new_chat(){navigate('sessions');await nextTick();await chat.value?.create_session();}
async function manage_services(){
  if(busy.value)return;
  const state=await call('get_model_config').catch(e=>{error.value=String(e);return null;}) as {config:Json}|null;
  if(!state)return;
  if(!state.config){navigate('models');return;}
  navigate('sessions');await nextTick();
  await chat.value?.prepare_prompt(t('service_browser.manage_prompt'));
}
async function manage_local(){
  if(busy.value)return;
  if(target.value)await choose_target(true);
  if(error.value)return;
  const state=await agent_call('get_model_config').catch(e=>{error.value=String(e);return null;}) as {config:Json}|null;
  if(!state)return;
  if(!state.config){navigate('models');return;}
  navigate('sessions');await nextTick();
  await chat.value?.prepare_prompt(t('ui.check_rove_s_storage_usage_and_runtime_status'));
}
const device = shallowRef<Json>(null);
const host_mobile=ref(false);
const target=shallowRef<Target>(),network_id=ref(''),device_id=ref('');
const target_key=computed(()=>target.value?`${target.value.network_id}/${target.value.device_id}`:'local');
async function choose_discovered(value:Target){network_id.value=value.network_id;device_id.value=value.device_id;await choose_target();}
function call(...args:Parameters<typeof agent_call>){return agent_call(args[0],args[1],args[2],args[3],target.value);}
async function choose_target(local=false){
  if(busy.value)return;
  if(!local){const uuid=/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;if(!uuid.test(network_id.value)||!uuid.test(device_id.value)){error.value=t('ui.enter_complete_network_and_device_uuids');return;}}
  target.value=local?undefined:{network_id:network_id.value,device_id:device_id.value};
  device.value=null;config_server_url.value='';
  await work(refresh);
}
const error = ref(''), busy = ref(false);
const max_active_runs = ref(4), config_server_url = ref('');
async function work(action:()=>Promise<void>) {
  busy.value=true; error.value='';
  try { await action(); } catch(e) { error.value = e instanceof Error ? e.message : t('ui.operation_failed'); }
  finally { busy.value=false; }
}
async function refresh() {
  device.value=await call('get_device');
  if(!target.value)host_mobile.value=['android','ios'].includes(String((device.value as Record<string,Json>).os));
  const settings=await call('get_settings') as {max_active_runs:number;config_server_url:string|null};
  max_active_runs.value=settings.max_active_runs; config_server_url.value=settings.config_server_url??'';
}
async function save_settings() { await work(async()=>{await call('update_settings',{max_active_runs:max_active_runs.value,config_server_url:config_server_url.value||null});}); }
onMounted(()=>work(refresh));
</script>

<template>
  <div v-if="showing_introduction" class="app-shell intro-shell"><IntroductionPage :step="introduction.step" :mobile="host_mobile" @step="intro_step" @finish="finish_intro" @start="start_intro" @models-changed="model_revision++"/><p v-if="intro_notice" class="intro-notice notice" role="status">{{intro_notice}}</p></div>
  <div v-show="!showing_introduction" class="app-shell" :class="{'mobile-shell':host_mobile,'chat-shell':chat_page,'conversation-focused':chat_page&&chat_view.detail&&chat_view.selected}">
    <a class="skip-link" href="#main-content" @click.prevent="($refs.main as HTMLElement)?.focus()">{{t('ui.skip_to_main_content')}}</a>
    <aside class="sidebar"><a class="brand" href="#/sessions"><span class="brand-mark"><i class="pi pi-compass" aria-hidden="true"/></span><span>rove<small>{{t('ui.roamer')}}</small></span></a>
      <p class="nav-caption">{{t('ui.your_roaming_space')}}</p><nav :aria-label="t('ui.main_navigation')"><a v-for="link in links" :key="link.id" :href="`#/${link.id}`" :class="{active:(page===link.id||(page==='archives'&&link.id==='sessions'))}" :aria-current="(page===link.id||(page==='archives'&&link.id==='sessions'))?'page':undefined"><i :class="`pi ${link.icon}`" aria-hidden="true"/>{{link.label}}</a></nav>
      <div class="sidebar-bottom"><p>{{ device ? t('ui.agent_connected') : t('ui.waiting_for_agent') }}</p><a href="#/settings"><i class="pi pi-cog" aria-hidden="true"/> {{t('ui.settings')}}</a><small>{{t('ui.rove_application')}}</small></div>
    </aside>
    <div class="main-shell">
      <header class="topbar"><div><Button v-if="chat_page&&chat_view.detail" class="conversation-back" icon="pi pi-arrow-left" text :aria-label="t('ui.back_to_conversations')" @click="chat?.back()"/><ConversationTitle v-if="chat_page&&chat_view.selected" :key="chat_view.session_id" :class="{'list-title':!chat_view.detail}" :title="chat_view.title" :session-id="chat_view.session_id" :disabled="chat_view.archived||chat_view.busy" :save="rename_chat"/><strong :class="{'page-chat-title':chat_page&&chat_view.selected,'detail-title':chat_view.detail}">{{({sessions:t('ui.conversations'),archives:t('ui.archived_conversations'),services:t('ui.services'),networks:t('ui.networks'),settings:t('ui.settings'),models:t('ui.models'),devices:t('ui.devices')})[page]}}</strong><span v-if="page!=='sessions'&&page!=='archives'" class="target-context">{{target?t('ui.remote_execution')+target.device_id:t('ui.this_device_local_rove_agent')}}</span></div>
        <div class="topbar-actions"><Button text :icon="dark?'pi pi-sun':'pi pi-moon'" :aria-label="t('ui.toggle_light_and_dark_theme')" @click="dark=!dark"/><div class="global-menu"><Button icon="pi pi-plus" :aria-label="t('ui.add_menu')" :aria-expanded="menu" @click="menu=!menu"/><div v-if="menu" class="menu-panel" @keydown.esc="menu=false"><button @click="new_chat">{{t('ui.new_conversation')}}</button><button @click="navigate('archives')">{{t('ui.archived_conversations')}}</button><button @click="navigate('networks')">{{t('ui.create_join_a_network')}}</button><button @click="navigate('models')">{{t('ui.model_settings')}}</button><button @click="navigate('settings')">{{t('ui.settings')}}</button></div></div></div>
      </header>
      <p v-if="error" class="error app-error" role="alert">{{notice_text(error)}}</p>
      <p v-if="host_mobile && (page==='settings'||page==='networks')" class="notice">{{t('ui.mobile_development_build_a_local_agent_is_embedded')}}</p>
      <main id="main-content" ref="main" tabindex="-1">
        <ChatPanel ref="chat" @view="chat_view=$event" v-show="page==='sessions'||page==='archives'" :archived="page==='archives'" @archives="navigate($event?'archives':'sessions')" :target="target" :model-revision="model_revision" @configure-model="configure_models" @manage-local="manage_local" @network-setup="navigate('networks')"/>
        <div v-show="page==='networks'" class="page-container"><div class="page-heading"><h1>{{t('ui.networks')}}</h1><Button :label="t('ui.view_devices')" icon="pi pi-desktop" outlined @click="navigate('devices')"/></div><p class="muted">{{t('ui.connection_status_comes_from_easytier_the_backend_still')}}</p><NetworkPanel :key="`network/${target_key}`" :target="target" :active="!showing_introduction&&page==='networks'" :mobile="host_mobile"/></div>
        <div v-show="page==='services'" class="page-container"><div class="page-heading"><h1>{{t('ui.services')}}</h1><Button :label="t('ui.manage_in_a_conversation')" icon="pi pi-comment" outlined @click="manage_services"/></div><ServicePanel :key="`services/${target_key}`" :target="target"/></div>
        <div v-show="page==='devices'" class="page-container"><div class="page-heading"><h1>{{t('ui.devices')}}</h1><Button :label="t('ui.back_to_networks')" text @click="navigate('networks')"/></div><section><DevicePicker :disabled="busy" @choose="choose_discovered"/></section><section><h2>{{t('ui.execution_target')}}</h2><p>{{target?target.device_id:t('ui.this_device')}}</p><form class="vertical" @submit.prevent="choose_target()"><label>{{t('ui.network_id')}}<input v-model="network_id" required></label><label>{{t('ui.device_id')}}<input v-model="device_id" required></label><div class="actions"><button :disabled="busy">{{t('ui.select_remote_device')}}</button><button type="button" :disabled="busy" @click="choose_target(true)">{{t('ui.return_to_this_device')}}</button></div></form><p class="muted">{{t('ui.conversations_still_belong_to_the_selected_execution_device')}}</p></section><section><h2>{{t('ui.target_device_status')}}</h2><pre v-if="device">{{JSON.stringify(device,null,2)}}</pre><p v-else>{{t('ui.not_connected_check_the_local_agent_and_the')}}</p><Button :label="t('ui.refresh_status')" outlined :disabled="busy" @click="work(refresh)"/></section></div>
        <div v-show="page==='settings'" class="page-container"><h1>{{t('ui.settings')}}</h1><section><h2>{{t('intro.reset')}}</h2><p>{{t('intro.reset_hint')}}</p><Button :label="t('intro.reset')" outlined @click="reset_intro"/><p v-if="intro_notice" role="status">{{intro_notice}}</p></section><section><h2>{{t('locale.title')}}</h2><label>{{t('locale.preference')}}<select :value="locale_preference" :aria-label="t('locale.preference')" @change="set_locale(($event.target as HTMLSelectElement).value)"><option value="system">{{t('locale.system')}}</option><option value="zh-CN">简体中文</option><option value="en">English</option></select></label><p class="muted">{{t('locale.hint')}}</p></section><section><h2>{{t('ui.manage_rove_through_conversation')}}</h2><p>{{t('ui.inspect_this_device_s_storage_and_runtime_first')}}</p><Button :label="t('ui.manage_rove_in_a_conversation')" outlined @click="manage_local"/></section><section><h2>{{t('ui.conversation_archive')}}</h2><p>{{t('ui.view_archived_history_restore_conversations_or_delete_completed')}}</p><Button :label="t('ui.archived_conversations')" outlined @click="navigate('archives')"/></section><section><h2>{{t('ui.model_settings')}}</h2><p>{{t('ui.save_multiple_model_variants_under_one_api_connection')}}</p><Button :label="t('ui.manage_models')" icon="pi pi-arrow-right" outlined @click="navigate('models')"/></section><section><h2>{{t('chat.suggestions')}}</h2><label class="chat-suggestions-setting"><input v-model="show_suggestions" type="checkbox">{{t('chat.show_suggestions')}}</label><p class="muted">{{t('chat.suggestions_hint')}}</p><p v-if="preference_error" class="error" role="status">{{t('intro.storage_error')}}</p></section><section><h2>{{t('ui.appearance')}}</h2><Button :label="dark?t('ui.dark_mode'):t('ui.light_mode')" outlined @click="dark=!dark"/></section><section><h2>{{t('ui.device_settings')}}</h2><form class="vertical" @submit.prevent="save_settings"><label>{{t('ui.concurrent_conversation_limit')}}<input v-model.number="max_active_runs" type="number" min="1" required></label><label>{{t('ui.encrypted_configuration_server')}}<input v-model="config_server_url" type="url" placeholder="https://config.example.com"></label><button :disabled="busy">{{t('ui.save_settings')}}</button></form></section><Button :label="t('ui.devices_and_execution_targets')" outlined @click="navigate('devices')"/></div>
        <div v-show="page==='models'" class="page-container"><Button :label="t('ui.back_to_settings')" text @click="navigate('settings')"/><h1>{{t('ui.model_settings')}}</h1><p class="muted">{{t('ui.configuration_is_saved_on_the_selected_agent_not')}}</p><ModelPanel :key="`models/${target_key}`" :target="target" @changed="model_revision++" @onboarding="open_onboarding"/></div>
      </main>
    </div>
    <nav class="bottom-nav" :aria-label="t('ui.mobile_navigation')"><a v-for="link in links" :key="link.id" :aria-label="link.label" :title="link.label" :href="`#/${link.id}`" :class="{active:(page===link.id||(page==='archives'&&link.id==='sessions'))}" :aria-current="(page===link.id||(page==='archives'&&link.id==='sessions'))?'page':undefined"><i :class="`pi ${link.icon}`" aria-hidden="true"/></a></nav>
  </div>
</template>
