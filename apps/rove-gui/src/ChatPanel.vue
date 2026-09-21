<script setup lang="ts">
import {show_suggestions} from './chat-preferences';
import {load_drafts,save_drafts} from './drafts';
import {t,format_datetime,notice_text} from './i18n';
import { ref, shallowRef, onMounted, onBeforeUnmount, computed, defineAsyncComponent, watch, nextTick } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { call as agent_call, type Target, type Json } from './api';
import {apply_events,type Run,type Snapshot,type EventBatch} from './run-events';
import {merge_run_page} from './run-pages';
import Button from 'primevue/button';
import Dialog from 'primevue/dialog';
import type {ModelCatalog} from './models';
import MessageCopy from './components/MessageCopy.vue';
const AssistantReply=defineAsyncComponent(()=>import('./components/AssistantReply.vue'));
const emit=defineEmits<{configureModel:[target?:Target];archives:[value:boolean];manageLocal:[];networkSetup:[];view:[value:{session_id:string;title:string;selected:boolean;detail:boolean;archived:boolean;busy:boolean;active:boolean}]}>();
const props=defineProps<{target?:Target;archived?:boolean;modelRevision?:number}>();
const catalog=ref<ModelCatalog>({connections:[],models:[],default_model_id:null});
const model_picker=ref(false),model_saving=ref(false);
const model_id=computed(()=>sessions.value.find(s=>s.session_id===selected.value)?.model_id??'');
async function choose_model(value:string){
  const session_id=selected.value;model_saving.value=true;
  await action(async()=>{const updated=await call('update_session',{model_id:value||null},{session_id}) as unknown as Session;sessions.value=sessions.value.map(s=>s.session_id===session_id?updated:s);pending.value[session_id]='';});
  model_saving.value=false;
}
const execution_target=computed(()=>sessions.value.find(s=>s.session_id===selected.value)?.execution_target??undefined);
async function load_models(){catalog.value=await agent_call('get_model_catalog',undefined,undefined,undefined,selected.value?execution_target.value:props.target) as unknown as ModelCatalog;}
watch(()=>props.modelRevision,()=>action(load_models));
async function open_models(){model_picker.value=true;await action(load_models);}
function call(...args:Parameters<typeof agent_call>){return agent_call(args[0],args[1],args[2],args[3]);}
interface Session {session_id:string; title:string;archived_at?:string|null;model_id?:string|null;kind?:string;execution_target?:Target|null}
interface PendingSubmission {request:{request_id:string;message:string;model_id?:string;network_id?:string};created_at:string;error?:{code:string;message:string}|null}
const unconfirmed=shallowRef<PendingSubmission[]>([]),pending_cursor=ref<string|null>(null);
async function load_pending(append=false){
  const id=selected.value,epoch=generation;if(!id)return;
  const page=await call('list_session_submissions',undefined,{session_id:id},append&&pending_cursor.value?{cursor:pending_cursor.value}:{}) as unknown as {items:PendingSubmission[];next_cursor:string|null};
  if(id===selected.value&&epoch===generation){unconfirmed.value=append?[...unconfirmed.value,...page.items]:page.items;pending_cursor.value=page.next_cursor;}
}
async function retry_submission(item:PendingSubmission){
  if(sending.value||selected_archived.value)return;sending.value=true;const id=selected.value;
  await action(async()=>{const run=await call('submit_run',item.request,{session_id:id}) as unknown as Run;if(selected.value===id){runs.value=merge_run_page(runs.value,[run]);await load_pending();await load_runs();await refresh_session();}});
  sending.value=false;
}
const sessions=shallowRef<Session[]>([]),runs=shallowRef<Run[]>([]),snapshots=shallowRef<Record<string,Snapshot>>({});
const selected=ref(''),error=ref(''),sending=ref(false),loading=ref(false);
const list_open=ref(true);
const delete_dialog=ref(false),archive_busy=ref(false),notice=ref('');
const delete_target=ref<Session>(),delete_checking=ref(false),delete_blocked=ref(true),delete_error=ref('');
async function ask_delete(session:Session){
  if(archive_busy.value||delete_checking.value)return;
  delete_target.value=session;delete_dialog.value=true;delete_checking.value=true;delete_blocked.value=true;delete_error.value='';error.value='';
  try{
    const pages=await Promise.all(['queued','running','cancelling'].map(status=>call('list_runs',undefined,undefined,{session_id:session.session_id,status,limit:1})));
    const pending=await call('list_session_submissions',undefined,{session_id:session.session_id},{limit:1}) as unknown as {items:unknown[]};
    if(!alive)return;
    if(pages.some(page=>!!(page as Record<string,Json>).sync_error)){delete_error.value=t('linked.cached');return;}
    delete_blocked.value=pending.items.length>0||pages.some(page=>(page as unknown as {items:Run[]}).items.some(run=>!terminal(run.status)));
    if(delete_blocked.value)delete_error.value=t('errors.session_has_active_runs');
  }catch(e){if(alive)delete_error.value=notice_text(String(e));}
  finally{if(alive)delete_checking.value=false;}
}
const selected_archived=computed(()=>!!sessions.value.find(s=>s.session_id===selected.value)?.archived_at);
const has_active_runs=computed(()=>unconfirmed.value.length>0||runs.value.some(run=>!terminal(run.status)));
async function lifecycle(operation:string,id=selected.value){
  if(archive_busy.value)return;archive_busy.value=true;
  await action(async()=>{
    await call(operation,undefined,{session_id:id});
    if(operation==='delete_session'){delete drafts.value[id];delete pending.value[id];}
    notice.value=operation==='archive_session'?t('ui.archived_you_can_restore_it_from_archived_conversations'):operation==='restore_session'?t('ui.restored_to_the_conversation_list'):t('ui.conversation_history_deleted_device_files_and_services_are');
    delete_dialog.value=false;selected.value='';generation++;runs.value=[];snapshots.value={};
    await load_sessions();if(selected.value)await select_session();list_open.value=true;
  });
  archive_busy.value=false;
}
watch(()=>props.archived,async()=>{selected.value='';generation++;watching.clear();runs.value=[];snapshots.value={};list_open.value=true;notice.value='';await action(async()=>{await load_sessions();if(selected.value)await select_session();});});
const draft_storage=(()=>{try{return localStorage;}catch{return undefined;}})();
const draft_save_failed=ref(false);
const drafts=ref<Record<string,string>>(load_drafts(draft_storage)),pending=ref<Record<string,string>>({});
watch(drafts,value=>{draft_save_failed.value=!save_drafts(draft_storage,value);},{deep:true,flush:'sync'});
const message=computed({get:()=>drafts.value[selected.value]??'',set:value=>{drafts.value[selected.value]=value;}});
const pending_request=computed({get:()=>pending.value[selected.value]??'',set:value=>{pending.value[selected.value]=value;}});
const suggestions=computed(()=>[
 {title:t('ui.bring_your_music_home'),text:t('ui.deploy_an_open_source_music_service_on_your'),prompt:t('ui.help_me_deploy_an_open_source_music_server')},
 {title:t('ui.explore_my_services'),text:t('ui.find_apps_already_available_on_your_devices'),prompt:t('ui.list_the_services_published_by_this_device')},
 {title:t('ui.build_your_private_cinema'),text:t('ui.host_open_source_video_services_and_watch_across'),prompt:t('ui.help_me_plan_an_open_source_media_server')},
 {title:t('ui.manage_multiple_coding_agents'),text:t('ui.deploy_and_access_open_source_coding_agents_on'),prompt:t('ui.help_me_deploy_a_coding_agent_on_this')},
 {title:t('ui.manage_rove'),text:t('ui.inspect_storage_adjust_locations_and_troubleshoot'),prompt:t('ui.check_rove_s_storage_usage_and_runtime_on')}
]);
async function choose_suggestion(prompt:string){if(!selected.value)await create_session();if(selected.value)message.value=message.value.trim()?message.value+'\n'+prompt:prompt;}
async function choose_session(id:string){selected.value=id;list_open.value=false;await select_session();}
async function prepare_prompt(prompt:string){const id=await create_session();if(id&&selected.value===id)message.value=prompt;return id;}
async function open_session(id:string){
  await action(async()=>{const session=await call('get_session',undefined,{session_id:id}) as unknown as Session;if(!sessions.value.some(s=>s.session_id===id))sessions.value=[session,...sessions.value];await choose_session(id);});
}
function back(){list_open.value=true;}
watch(()=>({session_id:selected.value,title:sessions.value.find(s=>s.session_id===selected.value)?.title??'',selected:!!selected.value,detail:!list_open.value,archived:selected_archived.value,busy:archive_busy.value,active:has_active_runs.value}),value=>emit('view',value),{immediate:true});
defineExpose({create_session,prepare_prompt,open_session,back,rename_title});
const transcript=ref<HTMLElement>(),follow_latest=ref(true);
function scrolled(){const el=transcript.value;if(el)follow_latest.value=el.scrollHeight-el.scrollTop-el.clientHeight<80;}
async function latest(){follow_latest.value=true;await nextTick();const el=transcript.value;if(el)el.scrollTop=el.scrollHeight;}
watch([runs,snapshots],()=>{if(follow_latest.value)void latest();},{flush:'post'});
watch([selected,list_open],()=>{void latest();});
let resize_observer:ResizeObserver|undefined;
onMounted(()=>{resize_observer=new ResizeObserver(()=>{if(follow_latest.value)void latest();});const content=transcript.value?.firstElementChild;if(content)resize_observer.observe(content);});
onBeforeUnmount(()=>resize_observer?.disconnect());
const next_session_cursor=ref<string|null>(null),next_run_cursor=ref<string|null>(null);
let alive=true,poll:ReturnType<typeof setTimeout>|undefined;
let generation=0;
let run_page_cursor:string|null=null,run_queue:Promise<void>=Promise.resolve();
const watching=new Map<string,number>();
function merge_snapshot(id:string,snapshot:Snapshot){
  const old=snapshots.value[id];
  if(!old||snapshot.snapshot_seq>=old.snapshot_seq)snapshots.value={...snapshots.value,[id]:snapshot};
}
async function watch_run(run_id:string,session_id:string,epoch:number){
  const current=()=>alive&&epoch===generation&&selected.value===session_id;
  try{
    while(current()){
      try{
        const snapshot=snapshots.value[run_id];if(!snapshot)return;
        const batch=await invoke<EventBatch>('agent_events',{runId:run_id,afterSeq:snapshot.snapshot_seq});
        if(!current())return;
        const updated=apply_events(snapshots.value[run_id],batch);merge_snapshot(run_id,updated);
        runs.value=runs.value.map(run=>run.run_id===run_id?updated.run:run);
        if(batch.terminal||terminal(updated.run.status)){
          const final=await call('get_run',undefined,{run_id}) as unknown as Snapshot;
          if(current())merge_snapshot(run_id,final);return;
        }
      }catch(e){
        if(!current())return;
        error.value=t('ui.event_connection_interrupted_restoring_a_snapshot_value',{p0:(e instanceof Error?e.message:String(e))});
        await new Promise(resolve=>setTimeout(resolve,1000));
        if(!current())return;
        try{const snapshot=await call('get_run',undefined,{run_id}) as unknown as Snapshot;if(current()){merge_snapshot(run_id,snapshot);if(terminal(snapshot.run.status))return;}}catch{/* Keep the explicit error and retry without resubmitting. */}
      }
    }
  }finally{if(watching.get(run_id)===epoch)watching.delete(run_id);}
}
function start_watchers(){
  for(const run of [...runs.value].sort((a,b)=>Number(a.status==='queued')-Number(b.status==='queued'))){
    if(watching.size>=8)break;
    if(terminal(run.status)||watching.has(run.run_id)||!snapshots.value[run.run_id])continue;
    watching.set(run.run_id,generation);void watch_run(run.run_id,selected.value,generation);
  }
}
let metadata_epoch=0;
const title_updates=new Set<string>();
async function rename_title(session_id:string,title:string){
  metadata_epoch++;title_updates.add(session_id);
  try{const updated=await call('update_session',{title},{session_id}) as unknown as Session;if(alive)sessions.value=sessions.value.map(s=>s.session_id===session_id?updated:s);}
  finally{metadata_epoch++;title_updates.delete(session_id);}
}
async function refresh_session(session_id=selected.value){
  if(!session_id||title_updates.has(session_id))return;
  const epoch=metadata_epoch;
  const updated=await call('get_session',undefined,{session_id}) as unknown as Session;
  if(alive&&epoch===metadata_epoch)sessions.value=sessions.value.map(s=>s.session_id===session_id?updated:s);
}
const terminal=(status:string)=>['succeeded','failed','cancelled','interrupted'].includes(status);
const status_name=(status:string)=>({queued:t('ui.queued'),running:t('ui.running'),cancelling:t('ui.cancelling'),succeeded:t('ui.completed'),failed:t('ui.failed'),cancelled:t('ui.cancelled'),interrupted:t('ui.interrupted')}[status]??status);
async function load_sessions(append=false){
  const epoch=generation;
  const page=await call('list_sessions',undefined,undefined,{order:'desc',archived:props.archived??false,...(append&&next_session_cursor.value?{cursor:next_session_cursor.value}:{})}) as unknown as {items:Session[];next_cursor:string|null};
  if(epoch!==generation)return;
  if(!alive)return;sessions.value=append?[...sessions.value,...page.items]:page.items;next_session_cursor.value=page.next_cursor;
  if(!selected.value&&sessions.value.length)selected.value=sessions.value[0].session_id;
}
function load_runs(append=false):Promise<void>{
  const target=selected.value,epoch=generation;
  const job=run_queue.then(()=>load_run_page(append,target,epoch));
  run_queue=job.catch(()=>{});return job;
}
async function load_run_page(append:boolean,target:string,epoch:number){
  const current=()=>alive&&target===selected.value&&epoch===generation;
  if(!target||!current()||(append&&!next_run_cursor.value))return;
  const cursor=append?next_run_cursor.value:run_page_cursor;
  const query:Record<string,string>={session_id:target};if(cursor)query.cursor=cursor;
  const page=await call('list_runs',undefined,undefined,query) as unknown as {items:Run[];next_cursor:string|null;sync_error?:{message:string}};
  if(!current())return;
  if(page.sync_error)error.value=t('linked.cached');
  runs.value=merge_run_page(runs.value,page.items);next_run_cursor.value=page.next_cursor;run_page_cursor=cursor;
  const pending=runs.value.filter(r=>!terminal(r.status)||!snapshots.value[r.run_id]||snapshots.value[r.run_id].run.status!==r.status);
  for(let offset=0;offset<pending.length;offset+=8){
    if(!current())return;
    const updates=await Promise.all(pending.slice(offset,offset+8).map(async run=>[run.run_id,await call('get_run',undefined,{run_id:run.run_id})] as const));
    if(!current())return;
    for(const [id,snapshot] of updates)merge_snapshot(id,snapshot as unknown as Snapshot);
  }
  if(current()){runs.value=runs.value.map(run=>snapshots.value[run.run_id]?.run??run);start_watchers();}
}
async function select_session(){generation++;watching.clear();run_page_cursor=null;runs.value=[];snapshots.value={};unconfirmed.value=[];next_run_cursor.value=null;await action(load_models);await action(()=>load_pending());await action(()=>load_runs());}
async function action(task:()=>Promise<void>){error.value='';try{await task();}catch(e){error.value=e instanceof Error?e.message:t('ui.operation_failed');}}
const creating=ref(false),creation_request=shallowRef<Json>();
async function create_session(){if(creating.value)return;creating.value=true;let created_id:string|undefined;await action(async()=>{creation_request.value??={session_id:crypto.randomUUID(),title:t('ui.new_conversation_2'),auto_title:true,...(props.target?{execution_target:{...props.target}}:{})};const session=await call('create_session',creation_request.value) as unknown as Session;creation_request.value=undefined;sessions.value=[session,...sessions.value.filter(s=>s.session_id!==session.session_id)];selected.value=session.session_id;created_id=session.session_id;list_open.value=false;await select_session();});creating.value=false;return created_id;}
async function send(){
  if(!selected.value||selected_archived.value||!message.value.trim()||sending.value||model_saving.value)return;
  sending.value=true;const target=selected.value;const input=message.value;
  // A lost response retains the same request_id until the input changes.
  const request_id=pending_request.value||crypto.randomUUID();pending_request.value=request_id;
  const chosen_model=model_id.value;
  await action(async()=>{const accepted=await call('submit_run',{request_id,message:input,...(chosen_model?{model_id:chosen_model}:{})},{session_id:target}) as unknown as Run;if(!alive)return;if(pending.value[target]===request_id)pending.value[target]='';if(drafts.value[target]===input)drafts.value[target]='';await refresh_session(target);if(selected.value===target){runs.value=merge_run_page(runs.value,[accepted]);await load_runs();}});
  await load_pending().catch(()=>{});sending.value=false;
}
async function cancel(run_id:string){await action(async()=>{await call('cancel_run',undefined,{run_id});await load_runs();});}
async function tick(){
  if(!alive)return;
  if(!loading.value){loading.value=true;await action(async()=>{await load_pending();await load_runs();await refresh_session();});loading.value=false;}
  if(alive)poll=setTimeout(tick,3000);
}
onMounted(async()=>{await action(load_sessions);await action(load_models);await tick();});
onBeforeUnmount(()=>{alive=false;generation++;watching.clear();if(poll)clearTimeout(poll);});
</script>

<template>
  <div class="chat-layout" :class="{'list-open':list_open,'detail-open':!list_open}">
    <aside class="conversation-list" :aria-label="t('ui.conversation_list')"><Button v-if="props.archived" :label="t('ui.back_to_conversations')" text @click="emit('archives',false)"/><p v-if="props.archived" class="muted">{{t('ui.keep_history_and_restore_it_anytime_deleting_a')}}</p><p v-if="notice" role="status">{{notice_text(notice)}}</p><div v-for="session in sessions" :key="session.session_id" class="session-row"><button class="session-item" :class="{selected:selected===session.session_id}" @click="choose_session(session.session_id)"><i class="pi pi-comment" aria-hidden="true"/><span>{{session.title}}</span></button><Button :label="props.archived?t('ui.restore'):undefined" :icon="props.archived?'pi pi-replay':'pi pi-inbox'" text :aria-label="`${props.archived?t('ui.restore'):t('ui.archive')} ${session.title}`" :disabled="archive_busy" @click="lifecycle(props.archived?'restore_session':'archive_session',session.session_id)"/><Button v-if="props.archived" :label="t('ui.delete')" icon="pi pi-trash" text severity="danger" :aria-label="t('ui.delete')+' '+session.title" :disabled="archive_busy||delete_checking" @click="ask_delete(session)"/></div><button v-if="next_session_cursor" class="secondary" @click="action(()=>load_sessions(true))">{{t('ui.load_more_conversations')}}</button><p v-if="!sessions.length" class="muted">{{props.archived?t('ui.no_archived_conversations'):t('ui.no_conversations_yet_use_the_add_button_to')}}</p></aside>
    <div class="conversation-detail">
      <p v-if="error" class="error" role="alert">{{notice_text(error)}}</p>
      <div ref="transcript" class="messages" role="log" :aria-label="t('ui.conversation_messages')" @scroll="scrolled"><div class="transcript-content">
      <p v-if="execution_target" class="notice">{{t('linked.owner')}}</p>
      <div v-if="unconfirmed.length" class="notice" role="status"><h3>{{t('linked.pending')}}</h3><p>{{t('linked.retry_help')}}</p><div v-for="item in unconfirmed" :key="item.request.request_id"><p class="pending-input">{{item.request.message}}</p><small>{{item.request.request_id}}</small><p v-if="item.error">{{notice_text(item.error.code+': '+item.error.message)}}</p><Button :label="t('linked.retry')" outlined :disabled="sending||selected_archived" @click="retry_submission(item)"/></div><Button v-if="pending_cursor" :label="t('linked.more')" text @click="action(()=>load_pending(true))"/></div>
      <p v-if="selected_archived" class="notice">{{t('ui.archived')}} {{format_datetime(sessions.find(s=>s.session_id===selected)?.archived_at)}} {{t('ui.history_is_read_only_tasks_continue_restore_to')}}</p><div v-if="!selected&&!props.archived" class="welcome-panel"><i class="pi pi-compass welcome-icon" aria-hidden="true"/><h1>{{t('ui.your_devices_wherever_you_roam')}}</h1><p class="data-slogan">{{t('ui.own_your_data_break_free_from_platforms')}}</p><p class="muted">{{t('ui.tell_rove_what_you_need_and_let_your')}}</p><Button :label="t('ui.start_a_conversation')" icon="pi pi-plus" @click="create_session"/></div>

        <div v-if="sessions.find(s=>s.session_id===selected)?.kind==='network_onboarding'" class="message assistant"><div class="message-content"><span class="message-author">{{t('ui.rove_agent_network_setup')}}</span><div class="message-bubble"><p>{{t('ui.your_models_are_ready_would_you_like_to')}}</p><p>{{t('ui.after_creating_a_network_export_its_network_card')}}</p><div class="actions"><Button :label="t('ui.create_network')" outlined @click="emit('networkSetup')"/><Button :label="t('ui.join_network')" outlined @click="emit('networkSetup')"/></div></div></div></div>
        <div v-for="run in runs" :key="run.run_id" class="run-conversation">
          <div class="message user"><div class="message-content"><span class="message-author">{{t('ui.you')}}</span><div class="message-bubble"><p>{{run.input_message}}</p></div><MessageCopy :text="run.input_message" :label="t('ui.copy_message')"/></div></div>
          <div class="message assistant"><div class="avatar"><i class="pi pi-compass" aria-hidden="true"/></div><div class="message-content"><span class="message-author">rove-agent</span><div class="message-bubble"><AssistantReply v-if="snapshots[run.run_id]?.output_tail" :text="snapshots[run.run_id].output_tail" :streaming="!terminal(run.status)"/><p v-else-if="run.status!=='succeeded'">{{status_name(run.status)}}</p><div v-if="run.status!=='succeeded'" class="bubble-meta"><span>{{status_name(run.status)}}</span><Button v-if="!terminal(run.status)&&!selected_archived" :label="t('ui.cancel_this_job')" text size="small" @click="cancel(run.run_id)"/></div><p v-if="snapshots[run.run_id]?.sync_error" class="notice">{{t('linked.cached')}}</p><p v-if="snapshots[run.run_id]?.output_truncated" class="muted">{{t('ui.only_the_retained_output_tail_is_shown_earlier')}}</p><div v-if="run.error" class="message-error"><p class="error">{{notice_text((run.error.code?run.error.code+': ':'')+run.error.message)}}</p><MessageCopy :text="notice_text((run.error.code?run.error.code+': ':'')+run.error.message)" :label="t('ui.copy_error')"/></div></div></div></div>
        </div>
        <div v-if="show_suggestions&&!runs.length&&!props.archived" class="suggestions" role="group" :aria-label="t('ui.conversation_suggestions')"><button v-for="suggestion in suggestions" :key="suggestion.title" @click="suggestion.title===t('ui.manage_rove')?emit('manageLocal'):choose_suggestion(suggestion.prompt)"><strong>{{suggestion.title}}</strong><span>{{suggestion.text}}</span></button></div>
        <Button v-if="next_run_cursor" :label="t('ui.load_more_messages')" text @click="action(()=>load_runs(true))"/>
      </div></div>
      <div class="composer-dock">
      <Button v-if="!follow_latest&&runs.length" class="latest-message" :label="t('chat.latest')" icon="pi pi-arrow-down" text @click="latest"/>
      <form v-if="selected&&!selected_archived" class="composer-area vertical" @submit.prevent="send"><p v-if="draft_save_failed" role="status" class="error">{{t('drafts.save_failed')}}</p><div class="composer"><textarea v-model="message" :aria-label="t('ui.message')" rows="2" maxlength="65536" :placeholder="t('ui.tell_this_device_what_you_want_to_do')" @input="pending_request=''" @keydown.ctrl.enter.prevent="send" @keydown.meta.enter.prevent="send"/><div class="composer-toolbar"><Button icon="pi pi-plus" text :aria-label="t('ui.model_settings')" type="button" @click="open_models"/><span class="muted">{{t('ui.ctrl_enter_to_send')}}</span><Button type="submit" icon="pi pi-arrow-up" :aria-label="t('ui.send_message')" :disabled="sending||model_saving||!message.trim()"/></div></div><p class="composer-footnote">{{model_id?(catalog.models.find(m=>m.model_id===model_id)?.name??t('ui.the_selected_model_was_removed_select_another_model')):(catalog.models.find(m=>m.model_id===catalog.default_model_id)?.name??t('ui.no_default_model'))}} {{t('ui.real_agent_execution_ai_can_modify_the_execution')}}</p></form>
      </div>
    </div>

    <Dialog v-model:visible="model_picker" modal :header="t('ui.choose_model')" :style="{width:'min(480px,94vw)'}"><label class="vertical">{{t('ui.model_for_this_conversation')}}<select :value="model_id" @change="choose_model(($event.target as HTMLSelectElement).value)" :disabled="sending||model_saving" :aria-label="t('ui.model_for_this_conversation')"><option value="">{{t('ui.follow_the_device_default')}}</option><option v-for="m in catalog.models" :key="m.model_id" :value="m.model_id">{{m.name}}</option><option v-if="model_id&&!catalog.models.some(m=>m.model_id===model_id)" :value="model_id" disabled>{{t('ui.selected_model_removed')}}</option></select></label><p class="muted">{{t('ui.other_conversations_and_running_jobs_are_not_changed')}}</p><template #footer><Button :label="t('ui.add_manage_models')" text @click="model_picker=false;emit('configureModel',execution_target)"/><Button :label="t('ui.done')" @click="model_picker=false"/></template></Dialog>
    <Dialog v-model:visible="delete_dialog" modal :closable="!archive_busy&&!delete_checking" :close-on-escape="!archive_busy&&!delete_checking" :header="t('ui.permanently_delete_conversation')" :style="{width:'min(440px,94vw)'}"><p>{{t('ui.permanently_delete')}}{{delete_target?.title}}{{t('ui.including_its_messages_drafts_and_job_history_this')}}</p><p v-if="delete_error||error" role="alert" class="error">{{delete_error||notice_text(error)}}</p><template #footer><Button :label="t('ui.cancel')" autofocus outlined :disabled="archive_busy||delete_checking" @click="delete_dialog=false"/><Button :label="t('ui.confirm_permanent_deletion')" severity="danger" :loading="delete_checking" :disabled="archive_busy||delete_checking||delete_blocked" @click="delete_target&&lifecycle('delete_session',delete_target.session_id)"/></template></Dialog>
  </div>
</template>
