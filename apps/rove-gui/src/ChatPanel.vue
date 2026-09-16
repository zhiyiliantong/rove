<script setup lang="ts">
import { ref, shallowRef, onMounted, onBeforeUnmount } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { call as agent_call, type Target, type Json } from './api';
import {apply_events,type Run,type Snapshot,type EventBatch} from './run-events';
import {merge_run_page} from './run-pages';
const props=defineProps<{target?:Target}>();
function call(...args:Parameters<typeof agent_call>){return agent_call(args[0],args[1],args[2],args[3],props.target);}
interface Session {session_id:string; title:string}
interface Message {message_id:string;role:string;parts:({kind:'text';text:string}|{kind:'tool_call';call:Json}|{kind:'tool_result';result:Json})[]}
const history=shallowRef<Message[]>([]),history_cursor=ref<string|null>(null),title=ref('');
const sessions=shallowRef<Session[]>([]),runs=shallowRef<Run[]>([]),snapshots=shallowRef<Record<string,Snapshot>>({});
const selected=ref(''),message=ref(''),error=ref(''),sending=ref(false),loading=ref(false),pending_request=ref('');
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
        const batch=await invoke<EventBatch>('agent_events',{runId:run_id,afterSeq:snapshot.snapshot_seq,target:props.target});
        if(!current())return;
        const updated=apply_events(snapshots.value[run_id],batch);merge_snapshot(run_id,updated);
        runs.value=runs.value.map(run=>run.run_id===run_id?updated.run:run);
        if(batch.terminal||terminal(updated.run.status)){
          const final=await call('get_run',undefined,{run_id}) as unknown as Snapshot;
          if(current())merge_snapshot(run_id,final);return;
        }
      }catch(e){
        if(!current())return;
        error.value=`事件连接中断，正在恢复快照：${e instanceof Error?e.message:String(e)}`;
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
async function load_history(append=false){
  if(!selected.value||!alive)return;const session_id=selected.value,epoch=generation;
  const page=await call('list_messages',undefined,{session_id},append&&history_cursor.value?{cursor:history_cursor.value}:{}) as unknown as {items:Message[];next_cursor:string|null};
  if(alive&&session_id===selected.value&&epoch===generation){history.value=append?[...history.value,...page.items]:page.items;history_cursor.value=page.next_cursor;}
}
async function rename(){const session_id=selected.value;await action(async()=>{const updated=await call('update_session',{title:title.value},{session_id}) as unknown as Session;if(alive)sessions.value=sessions.value.map(session=>session.session_id===session_id?updated:session);});}
const terminal=(status:string)=>['succeeded','failed','cancelled','interrupted'].includes(status);
const status_name=(status:string)=>({queued:'排队中',running:'运行中',cancelling:'正在取消',succeeded:'已完成',failed:'失败',cancelled:'已取消',interrupted:'已中断'}[status]??status);
async function load_sessions(append=false){
  const page=await call('list_sessions',undefined,undefined,append&&next_session_cursor.value?{cursor:next_session_cursor.value}:{}) as unknown as {items:Session[];next_cursor:string|null};
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
  const page=await call('list_runs',undefined,undefined,query) as unknown as {items:Run[];next_cursor:string|null};
  if(!current())return;
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
async function select_session(){generation++;watching.clear();run_page_cursor=null;runs.value=[];snapshots.value={};history.value=[];history_cursor.value=null;title.value=sessions.value.find(s=>s.session_id===selected.value)?.title??'';next_run_cursor.value=null;pending_request.value='';await action(()=>load_runs());}
async function action(task:()=>Promise<void>){error.value='';try{await task();}catch(e){error.value=e instanceof Error?e.message:'操作失败';}}
async function create_session(){await action(async()=>{const session=await call('create_session',{title:'新会话'}) as unknown as Session;sessions.value=[...sessions.value,session];selected.value=session.session_id;await select_session();});}
async function send(){
  if(!selected.value||!message.value.trim()||sending.value)return;
  sending.value=true;const target=selected.value;const input=message.value;
  // A lost response retains the same request_id until the input changes.
  const request_id=pending_request.value||crypto.randomUUID();pending_request.value=request_id;
  await action(async()=>{const accepted=await call('submit_run',{request_id,message:input},{session_id:target}) as unknown as Run;if(pending_request.value===request_id)pending_request.value='';if(alive&&selected.value===target){runs.value=merge_run_page(runs.value,[accepted]);if(message.value===input)message.value='';await load_runs();}});
  sending.value=false;
}
async function cancel(run_id:string){await action(async()=>{await call('cancel_run',undefined,{run_id});await load_runs();});}
async function tick(){
  if(!alive)return;
  if(!loading.value){loading.value=true;await action(()=>load_runs());loading.value=false;}
  if(alive)poll=setTimeout(tick,3000);
}
onMounted(async()=>{await action(load_sessions);title.value=sessions.value.find(s=>s.session_id===selected.value)?.title??'';await tick();});
onBeforeUnmount(()=>{alive=false;generation++;watching.clear();if(poll)clearTimeout(poll);});
</script>

<template>
  <section class="chat-panel">
    <div class="chat-heading"><div><h2>和 rove-agent 对话</h2><p class="muted">执行目标：{{target?target.device_id:'本机'}} · 不同会话可并发，关闭窗口不会停止作业。</p></div><button @click="create_session">新会话</button></div>
    <p v-if="error" class="error" role="alert">{{error}}</p>
    <label>会话<select v-model="selected" @change="select_session"><option value="" disabled>创建或选择会话</option><option v-for="session in sessions" :key="session.session_id" :value="session.session_id">{{session.title}} · {{session.session_id.slice(0,8)}}</option></select></label>
    <button v-if="next_session_cursor" class="secondary" @click="action(()=>load_sessions(true))">加载更多会话</button>
    <form v-if="selected" @submit.prevent="rename"><input v-model="title" required maxlength="128" aria-label="会话名称"/><button>重命名会话</button></form>
    <details v-if="selected"><summary>已归档消息与工具结果</summary><p class="muted">按需读取该会话历史，排队中的输入不提前进入上下文。</p><button @click="action(()=>load_history())">刷新历史</button><div v-for="entry in history" :key="entry.message_id" class="run-card"><small>{{entry.role}} · {{entry.message_id}}</small><pre v-for="(part,index) in entry.parts" :key="index" class="run-output">{{part.kind==='text'?part.text:JSON.stringify(part.kind==='tool_call'?part.call:part.result,null,2)}}</pre></div><button v-if="history_cursor" @click="action(()=>load_history(true))">加载更多消息</button></details>
    <div v-for="run in runs" :key="run.run_id" class="run-card">
      <div class="chat-heading"><span class="run-status">{{status_name(run.status)}}</span><button v-if="!terminal(run.status)" class="secondary" @click="cancel(run.run_id)">取消此作业</button></div>
      <p class="user-message">{{run.input_message}}</p><small>{{run.run_id}}</small>
      <pre v-if="snapshots[run.run_id]?.output_tail" class="run-output">{{snapshots[run.run_id].output_tail}}</pre>
      <p v-if="snapshots[run.run_id]?.output_truncated" class="muted">仅展示保留的输出尾部，历史已截断。</p>
      <p v-if="run.error" class="error">{{run.error.message}}</p>
    </div>
    <button v-if="next_run_cursor" class="secondary" @click="action(()=>load_runs(true))">加载更多作业</button>
    <form class="vertical" @submit.prevent="send"><textarea v-model="message" :disabled="!selected" rows="3" maxlength="65536" placeholder="告诉这台设备你想做什么…" @input="pending_request=''"/><div class="chat-heading"><p class="muted">AI 可通过 shell 修改本机系统，请确认任务内容。</p><button :disabled="sending||!selected||!message.trim()">{{sending?'提交中…':'发送任务'}}</button></div></form>
  </section>
</template>

<style scoped>
.chat-heading{display:flex;align-items:center;justify-content:space-between;gap:16px}.chat-heading h2{margin-bottom:8px}.chat-heading .muted{margin:6px 0 18px}select{padding:10px;border:1px solid #ccd7ca;background:#fafcf9;border-radius:7px;color:inherit;max-width:100%}.run-card{border:1px solid #e2e9df;border-radius:10px;padding:18px;margin:18px 0;background:#fbfcf9}.run-status{font-size:12px;color:#416852}.user-message{white-space:pre-wrap;overflow-wrap:anywhere;font-size:14px;line-height:1.6}.run-output{white-space:pre-wrap;overflow-wrap:anywhere;max-height:340px;overflow:auto;background:#eff3ec;border-radius:8px;padding:14px}.secondary{color:#345740;background:#e7efdf;font-size:12px;padding:8px 12px}textarea{margin-top:22px;resize:vertical}.run-card small{font-size:10px;color:#85917f}
</style>
