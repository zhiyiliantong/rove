<script setup lang="ts">
import {ref,watch} from 'vue';
import Button from 'primevue/button';
import {t} from '../i18n';
const value=defineModel<string>({required:true});
defineProps<{label:string}>();
const visible=ref(false);
watch(value,value=>{if(!value)visible.value=false;});
</script>
<template>
  <div class="secret-input"><input v-model="value" :aria-label="label" :type="visible?'text':'password'" autocomplete="off" autocapitalize="off" :spellcheck="false" maxlength="16384"><Button type="button" :aria-label="t(visible?'model_test.hide_key':'model_test.show_key')" :aria-pressed="visible" :icon="visible?'pi pi-eye-slash':'pi pi-eye'" text @click="visible=!visible"/></div>
</template>
<style scoped>
.secret-input{display:flex;align-items:center;gap:4px;min-width:0;width:100%}.secret-input input{flex:1;min-width:0}.secret-input button{flex:none}
</style>
