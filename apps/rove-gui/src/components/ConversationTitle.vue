<script setup lang="ts">
import {nextTick,onBeforeUnmount,ref,watch} from 'vue';
import {t,notice_text} from '../i18n';
const props=defineProps<{title:string;sessionId:string;disabled:boolean;save:(id:string,title:string)=>Promise<void>}>();
const editing=ref(false),draft=ref(''),saving=ref(false),error=ref(''),input=ref<HTMLInputElement>();
let alive=true;
onBeforeUnmount(()=>{alive=false;});
watch(()=>props.disabled,value=>{if(value)editing.value=false;});
async function edit(){
  if(props.disabled||saving.value)return;
  draft.value=props.title;error.value='';editing.value=true;
  await nextTick();input.value?.focus();input.value?.select();
}
function cancel(){if(saving.value)return;editing.value=false;error.value='';}
async function commit(){
  if(!editing.value||saving.value||props.disabled)return;
  const title=draft.value.trim();
  if(!title){error.value=t('title_edit.required');return;}
  // No-op edits still mark a placeholder title as manually chosen.
  saving.value=true;error.value='';
  try{await props.save(props.sessionId,title);if(alive)editing.value=false;}
  catch(e){if(alive)error.value=notice_text(String(e));}
  finally{if(alive)saving.value=false;}
}
function keydown(event:KeyboardEvent){
  if(event.isComposing||event.keyCode===229)return;
  if(event.key==='Escape'){event.preventDefault();cancel();}
  if(event.key==='Enter'){event.preventDefault();void commit();}
}
</script>
<template>
  <div class="inline-title">
    <input v-if="editing" ref="input" v-model="draft" class="title-input" :aria-label="t('ui.conversation_name')" :aria-invalid="!!error" :aria-busy="saving" :readonly="saving" maxlength="256" enterkeyhint="done" @keydown="keydown" @blur="commit"/>
    <button v-else class="chat-title" :disabled="disabled" :aria-label="t('ui.rename_conversation')" :title="title" @click="edit">{{title}}</button>
    <span v-if="error" class="title-error" role="alert">{{error}}</span>
  </div>
</template>
<style scoped>
.inline-title{position:relative;min-width:0;max-width:100%}.chat-title{max-width:100%;display:block}.title-input{width:100%;min-width:0;max-width:100%;height:36px;padding:4px 6px;font:inherit;font-weight:600;border:1px solid var(--accent);border-radius:6px;box-sizing:border-box}.title-error{position:absolute;left:0;top:100%;z-index:40;width:min(260px,60vw);background:var(--surface);color:var(--p-red-500,#b42318);border:1px solid var(--line);padding:8px;font-size:12px;overflow-wrap:anywhere}
</style>
