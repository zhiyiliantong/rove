<script setup lang="ts">
import { ref, watch } from 'vue';
import Button from 'primevue/button';
import Dialog from 'primevue/dialog';
import { data, ui, route, navigate, clearPrototypeReview } from '../ui';
const error=ref('');
watch(route,path=>{if(path==='/review-reset')ui.reviewReset=true;},{immediate:true});
watch(()=>ui.reviewReset,()=>{error.value='';});
function close() {ui.reviewReset=false;if(route.value==='/review-reset')navigate('/sessions');}
function clear() {try {clearPrototypeReview();}catch(e){error.value=e instanceof Error?e.message:'清空失败，请重试。';}}
</script>
<template>
  <Dialog :visible="ui.reviewReset" modal header="清空演示数据并重新开始？" :style="{width:'32rem'}" :breakpoints="{'640px':'94vw'}" @update:visible="close">
    <p>这不是“重置使用引导”。确认后将删除当前浏览器中本原型的：</p>
    <ul class="review-reset-counts"><li>{{ data.models.length }} 个模型型号及其连接</li><li>{{ data.networks.length }} 个网络、{{ data.devices.length }} 台演示设备、{{ data.services.length }} 个演示服务</li><li>{{ data.sessions.length }} 条会话（含归档与草稿）、{{ data.runs.length }} 条演示任务</li></ul>
    <p>删除后无法恢复。不影响正式 Rove、其他网站数据或明暗主题。清空后立即回到引导第一页。</p>
    <p class="inline-info">请先关闭这个浏览器中其他 Rove 原型标签页，避免旧页面再次写回数据。</p>
    <p v-if="error" class="form-error" role="alert">{{error}}</p>
    <template #footer><Button label="取消，保留数据" outlined autofocus @click="close"/><Button label="确认清空并重新开始" severity="danger" @click="clear"/></template>
  </Dialog>
</template>
