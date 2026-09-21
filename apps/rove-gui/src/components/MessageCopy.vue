<script setup lang="ts">
import {ref} from 'vue';
import Button from 'primevue/button';
import Textarea from 'primevue/textarea';
import {t} from '../i18n';
const props=defineProps<{text:string;label:string}>();
const manual=ref<string|null>(null);
async function copy(){
  const value=props.text;
  try{
    if(!navigator.clipboard?.writeText)throw new Error('clipboard unavailable');
    await navigator.clipboard.writeText(value);
    manual.value=null;
  }catch{manual.value=value;}
}
</script>
<template>
  <div class="message-copy">
    <Button :aria-label="label" :title="label" icon="pi pi-copy" text rounded severity="secondary" :disabled="!text" @click="copy"/>
    <div v-if="manual!==null" class="manual-copy">
      <small role="status">{{t('ui.clipboard_unavailable_select_the_text_below_and_copy')}}</small>
      <Textarea :model-value="manual" :aria-label="t('ui.text_for_manual_copy')" readonly rows="3" @focus="($event.target as HTMLTextAreaElement).select()"/>
      <Button :label="t('ui.close_manual_copy')" text size="small" @click="manual=null"/>
    </div>
  </div>
</template>
