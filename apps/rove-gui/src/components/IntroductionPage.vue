<script setup lang="ts">
import {computed,onMounted,ref} from 'vue';
import Button from 'primevue/button';
import Dialog from 'primevue/dialog';
import {t,notice_text} from '../i18n';
import {call} from '../api';
import type {ModelCatalog} from '../models';
import ModelPanel from '../ModelPanel.vue';
import NetworkPanel from '../NetworkPanel.vue';
import NetworkWalkthrough from './NetworkWalkthrough.vue';
import {conversation_suggestions} from '../suggestions';
const props=defineProps<{step:number;mobile:boolean}>();
const emit=defineEmits<{step:[value:number];finish:[];start:[prompt:string];modelsChanged:[]}>();
const pages=computed(()=>['welcome','model','network','chat'].map(key=>t('intro.'+key)));
const suggestions=computed(conversation_suggestions);
const count=ref(0),verified=ref(0),loading=ref(true),error=ref('');
const model_open=ref(false),network_mode=ref<'create'|'join'>('create'),network_open=ref(false);
const pending_prompt=ref<string|null>(null);
async function refresh(){loading.value=true;error.value='';try{const catalog=await call('get_model_catalog') as unknown as ModelCatalog;count.value=catalog.models.length;verified.value=catalog.models.filter(m=>m.tested_at).length;}catch(e){error.value=String(e);}finally{loading.value=false;}}
function add_model(){pending_prompt.value=null;model_open.value=true;}
async function saved(){await refresh();emit('modelsChanged');if(error.value||!verified.value)return;model_open.value=false;if(pending_prompt.value!==null){const prompt=pending_prompt.value;pending_prompt.value=null;emit('start',prompt);}else emit('step',2);}
async function start(prompt=''){await refresh();if(error.value)return;if(!verified.value){pending_prompt.value=prompt;model_open.value=true;}else emit('start',prompt);}
async function finish(){await refresh();if(verified.value&&!error.value)emit('finish');}
function network(mode:'create'|'join'){network_mode.value=mode;network_open.value=true;}
onMounted(refresh);
</script>
<template>
  <section id="introduction-content" tabindex="-1" class="introduction" :aria-label="t('intro.title')">
    <header class="intro-header"><span class="intro-brand"><i class="pi pi-compass" aria-hidden="true"/> rove</span><span class="intro-demo"/><Button :label="t('intro.skip')" text  :disabled="loading||!verified||!!error" @click="finish"/></header>
    <nav class="intro-progress" :aria-label="t('intro.title')"><button v-for="(name,index) in pages" :key="name" :aria-current="step===index?'step':undefined" @click="emit('step',index)"><span>{{index+1}}</span><strong>{{name}}</strong></button></nav>
    <p v-if="error" class="error" role="alert">{{notice_text(error)}} <Button :label="t('ui.refresh')" text @click="refresh"/></p>
    <div class="intro-content" :class="`intro-step-${step}`">
      <template v-if="step===0">
        <div class="intro-hero" aria-hidden="true"><div class="hero-orbit orbit-one"/><div class="hero-orbit orbit-two"/><span class="hero-center"><i class="pi pi-compass"/></span><span class="hero-device hero-pc"><i class="pi pi-desktop"/></span><span class="hero-device hero-mobile"><i class="pi pi-mobile"/></span><span class="hero-device hero-server"><i class="pi pi-server"/></span><span class="hero-device hero-code"><i class="pi pi-code"/></span></div>
        <span class="eyebrow">YOUR DEVICES. YOUR WORLD.</span><h1>{{t('ui.your_devices_wherever_you_roam')}}</h1><p class="intro-slogan">{{t('ui.own_your_data_break_free_from_platforms')}}</p><p class="intro-description">{{t('intro.welcome_description')}}</p><Button :label="t('intro.start')" icon="pi pi-arrow-right" icon-pos="right" @click="emit('step',1)"/>
      </template>
      <template v-else-if="step===1">
        <span class="intro-symbol"><i class="pi pi-sparkles" aria-hidden="true"/></span><span class="eyebrow">01 / YOUR AI</span><h1>{{t('intro.model_title')}}</h1><p class="intro-description">{{t('intro.model_description')}}</p>
        <div class="intro-model-card"><i class="pi pi-comments" aria-hidden="true"/><div><h2>{{t(verified?'intro.model_ready':'intro.model_empty')}}</h2><p v-if="count">{{t('intro.model_count',{n:count})}}</p><small>{{t('intro.model_hint')}}</small></div><i v-if="verified" class="pi pi-check-circle" aria-hidden="true"/></div>
        <p v-if="!verified" class="notice" role="status">{{t('model_test.required')}}</p><div class="intro-actions"><Button :label="t(count?'intro.model_add_more':'intro.model')" icon="pi pi-plus" :disabled="loading" @click="add_model"/><Button :label="t('intro.next')" :disabled="loading||!verified||!!error" text @click="emit('step',2)"/></div>
      </template>
      <template v-else-if="step===2">
        <span class="eyebrow">02 / CONNECT YOUR WORLD</span><h1>{{t('intro.network_title')}}</h1><p class="intro-description">{{t('intro.network_description')}}</p><NetworkWalkthrough/>
        <div class="intro-actions"><Button :label="t('intro.create')" icon="pi pi-plus" @click="network('create')"/><Button :label="t('intro.join')" icon="pi pi-sign-in" outlined @click="network('join')"/></div><p class="intro-trust">{{t('intro.network_trust')}}</p>
        <p v-if="mobile" class="notice">{{t('ui.mobile_development_build_a_local_agent_is_embedded')}}</p><Button :label="t('intro.next')" text icon="pi pi-arrow-right" icon-pos="right" @click="emit('step',3)"/>
      </template>
      <template v-else>
        <span class="intro-symbol"><i class="pi pi-comment" aria-hidden="true"/></span><span class="eyebrow">03 / MAKE IT YOURS</span><h1>{{t('intro.chat_title')}}</h1><p class="intro-description">{{t('intro.chat_description')}}</p>
        <div class="suggestions" role="group" :aria-label="t('ui.conversation_suggestions')"><button v-for="item in suggestions" :key="item.title" :disabled="loading" @click="start(item.prompt)"><i :class="`pi ${item.icon}`" aria-hidden="true"/><strong>{{item.title}}</strong><span>{{item.text}}</span></button></div>
        <p class="intro-draft-note">{{t('intro.draft_hint')}}</p><Button :label="t('ui.new_conversation')" icon="pi pi-plus" :disabled="loading" @click="start()"/>
      </template>
    </div>
    <footer class="intro-footer"><Button v-if="step>0" :label="t('intro.back')" icon="pi pi-arrow-left" text @click="emit('step',step-1)"/><span v-else>{{t('intro.tagline')}}</span><span>{{step+1}} / 4</span><Button v-if="step===3" :label="t('intro.list')" text :disabled="loading||!verified||!!error" @click="finish"/><span v-else>{{t(verified?'intro.anytime':'model_test.required')}}</span></footer>
    <ModelPanel v-if="model_open" auto-add form-only require-test @changed="refresh();emit('modelsChanged')" @saved="saved" @close="model_open=false;pending_prompt=null"/>
    <Dialog v-model:visible="network_open" modal :header="t('intro.'+network_mode)" :style="{width:'min(760px,94vw)'}"><NetworkPanel v-if="network_open" :mode="network_mode" active :mobile="mobile"/><template #footer><Button :label="t('intro.close')" text @click="network_open=false"/></template></Dialog>
  </section>
</template>
