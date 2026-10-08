<script setup lang="ts">
import {computed} from 'vue';
import {notice_text,t} from '../i18n';
import MessageCopy from './MessageCopy.vue';
const props=defineProps<{message:string}>();
const copyText=computed(()=>{const translated=notice_text(props.message);return translated===props.message?props.message:`${translated}\n${props.message}`;});
</script>
<template>
  <div class="error network-error" role="alert"><i class="pi pi-exclamation-circle" aria-hidden="true"/><span>{{notice_text(message)}}</span><MessageCopy :key="message" :text="copyText" :label="t('ui.copy_error')"/></div>
</template>
<style scoped>
.network-error{display:grid;grid-template-columns:auto minmax(0,1fr) 44px;align-items:start;gap:8px;min-width:0;overflow-wrap:anywhere}.network-error>i{margin-top:3px}.network-error>span{min-width:0}.network-error :deep(.message-copy){display:contents}.network-error :deep(.message-copy>.p-button){grid-column:3;grid-row:1;margin:0}.network-error :deep(.manual-copy){grid-column:1/-1;min-width:0}.network-error :deep(textarea){box-sizing:border-box;width:100%}
</style>
