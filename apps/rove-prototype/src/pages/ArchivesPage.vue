<script setup lang="ts">
import { computed, nextTick, ref } from 'vue';
import Button from 'primevue/button';
import Dialog from 'primevue/dialog';
import { api, data, navigate, notice, perform, restoreSession } from '../ui';
const archived = computed(() => data.value.sessions.filter(s => s.archived_at !== null).sort((a, b) => b.archived_at! - a.archived_at!));
const selectedId = ref('');
const selected = computed(() => data.value.sessions.find(s => s.id === selectedId.value));
const cancelButton = ref<{ $el: HTMLButtonElement }>();
const active = (id: string) => data.value.runs.some(r => r.session_id === id && ['queued', 'running', 'cancelling'].includes(r.status));
const formatTime = (value: number) => new Date(value).toLocaleString('zh-CN', { hour12: false });
function askDelete(id: string) {
  if (active(id)) { notice('会话仍有进行中的任务，请等待完成，或恢复会话后取消任务再删除。', true); return; }
  selectedId.value = id;
}
async function focusCancel() { await nextTick(); cancelButton.value?.$el.focus(); }
async function remove() {
  if (!perform(() => { api.deleteSession(selectedId.value); return true; })) return;
  selectedId.value = '';
  notice('会话及关联任务历史已永久删除，无法恢复。已部署服务不受影响。');
  await nextTick(); document.getElementById('archive-heading')?.focus();
}
</script>
<template>
  <div class="page-container archive-page">
    <Button label="返回会话列表" icon="pi pi-arrow-left" text @click="navigate('/sessions')"/>
    <div class="page-heading"><div><span class="eyebrow">ARCHIVED CONVERSATIONS</span><h1 id="archive-heading" tabindex="-1">归档管理</h1><p class="muted">{{ archived.length }} 个已归档会话 · 归档只是整理，不会取消任务。</p></div></div>
    <p class="muted">这里仅管理此浏览器的演示记录。恢复后可继续对话；永久删除不会移除设备上的服务。</p>
    <section v-if="archived.length" aria-label="已归档会话">
      <article v-for="s in archived" :key="s.id" class="settings-card archive-item" :aria-label="s.title">
        <div class="archive-summary"><h2>{{ s.title }}</h2><small>归档于 {{ formatTime(s.archived_at!) }}</small><p class="archive-excerpt">{{ s.messages.at(-1)?.text }}</p><span v-if="active(s.id)" class="status-label">任务进行中 · 仍会继续更新</span></div>
        <div class="archive-actions"><Button label="查看" icon="pi pi-eye" text @click="navigate(`/sessions/${s.id}`)"/><Button label="恢复" icon="pi pi-replay" outlined @click="restoreSession(s.id)"/><Button label="删除" icon="pi pi-trash" text severity="danger" @click="askDelete(s.id)"/></div>
      </article>
    </section>
    <div v-else class="empty-state"><i aria-hidden="true" class="pi pi-inbox"/><h2>暂无归档会话</h2><p>在会话的“更多”菜单中选择归档，之后可在这里恢复或删除。</p><Button label="去看会话" outlined @click="navigate('/sessions')"/></div>
    <Dialog :visible="!!selectedId" modal header="永久删除会话？" :style="{ width: '30rem' }" :breakpoints="{ '640px': '94vw' }" @update:visible="selectedId = ''" @show="focusCancel">
      <p class="delete-session-title">“{{ selected?.title }}”</p><p>将永久删除这条会话的消息、草稿和关联任务历史，删除后无法恢复。</p><p class="muted">仅删除当前浏览器演示数据，不会删除已部署服务或真实设备文件。</p>
      <template #footer><Button ref="cancelButton" label="取消" outlined autofocus @click="selectedId = ''"/><Button label="永久删除" severity="danger" @click="remove"/></template>
    </Dialog>
  </div>
</template>
