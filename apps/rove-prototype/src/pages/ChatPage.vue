<script setup lang="ts">
import { computed, defineAsyncComponent, nextTick, ref } from 'vue';
import Button from 'primevue/button';
import Select from 'primevue/select';
import Textarea from 'primevue/textarea';
import SessionActions from '../components/SessionActions.vue';
import ConversationSuggestions from '../components/ConversationSuggestions.vue';
import { manageRovePrompt } from '../conversation-prompts';
import { startRoveManagement } from '../ui';
import { chatTimeline } from '../chat-timeline';
import type { Run } from '../domain';
const AssistantReply = defineAsyncComponent(() => import('../components/AssistantReply.vue'));
import { api, data, ui, route, navigate, newSession, perform, modelLabel, deviceName, addModel, addNetwork, notice, isConnected, mobilePlatform, archiveSession, restoreSession } from '../ui';
const sid = computed(() => route.value.split('/')[2]);
const current = computed(() => data.value.sessions.find(s => s.id === sid.value));
const activeSessions = computed(() => data.value.sessions.filter(s => s.archived_at === null));
const archived = computed(() => current.value?.archived_at != null);
const selectedTarget = computed({ get: () => current.value?.target_device_id ?? 'local', set: value => { if (current.value) perform(() => api.setSessionTarget(current.value!.id, value)); } });
const modelPicker = ref<{ $el: HTMLElement }>();
function chooseModel() { attachmentMenu.value = false; const control = modelPicker.value?.$el.querySelector<HTMLElement>('[role="combobox"]'); control?.focus(); control?.click(); }
const modelOptions = computed(() => data.value.models.map(m => ({ label: modelLabel(m.id), value: m.id })));
const draft = computed({ get: () => current.value?.draft ?? '', set: value => { if (current.value) perform(() => api.saveDraft(current.value!.id, value)); } });
const runs = computed(() => data.value.runs.filter(r => r.session_id === sid.value));
const timeline = computed(() => chatTimeline(current.value?.messages ?? [], runs.value));
const showSuggestions = computed(() => current.value && !current.value.onboarding && !current.value.messages.some(m => m.role === 'user') && !runs.value.length);
async function chooseSuggestion(prompt: string) {
  if (prompt === manageRovePrompt) { await startRoveManagement(); return; }
  if (!current.value) newSession();
  if (!current.value) return;
  draft.value = draft.value.trim() ? `${draft.value}\n${prompt}` : prompt;
  await nextTick();
  document.querySelector<HTMLTextAreaElement>('textarea[aria-label="消息"]')?.focus();
}
function runText(run: Run) {
  if (run.waiting_connection) return '暂时联系不上执行设备，连接恢复后会继续获取状态。';
  if (run.error) return run.error;
  if (run.status === 'queued') return '请求已收到，正在等待前一项任务或设备空闲。';
  if (run.status === 'running') return `正在${deviceName(run.device_id)}上处理你的请求。你可以继续对话或稍后回来查看。`;
  if (run.status === 'cancelled') return '任务已取消；已经完成的操作不会自动撤销。';
  if (run.status === 'succeeded') return '这项演示任务已完成。';
  return statuses[run.status];
}
const statuses: Record<string, string> = { queued: '排队中', running: '执行中', succeeded: '已完成', failed: '执行失败', cancelled: '已取消', interrupted: '已中断', cancelling: '取消中' };
const attachmentMenu = ref(false);
const fileInput = ref<HTMLInputElement>();
const attachmentNames = ref<string[]>([]);
function files(event: Event) { const input = event.target as HTMLInputElement; attachmentNames.value = [...input.files ?? []].map(f => f.name); notice('仅记录附件名称，不读取或上传文件内容。'); input.value = ''; }
function send() {
  if (!current.value || archived.value) return;
  let target = selectedTarget.value;
  const mentioned = data.value.devices.filter(d => d.id !== 'local' && draft.value.includes(d.name));
  if (mentioned.length > 1) { notice('请在消息中明确指定一台目标设备。', true); return; }
  if (mentioned[0]) { target = mentioned[0].id; selectedTarget.value = target; }
  const services = data.value.services.filter(s => (isConnected(s.network_id) || (s.device_id === 'local' && !s.network_id)) && draft.value.includes(s.name));
  if (services.length > 1) { notice('请一次指定一个服务。', true); return; }
  if (!mentioned.length && services[0]) { target = services[0].device_id; selectedTarget.value = target; }
  const result = perform(() => api.submit(current.value!.id, draft.value + (attachmentNames.value.length ? `\n[演示附件：${attachmentNames.value.join('、')}]` : ''), target));
  if (result) attachmentNames.value = [];
}
</script>
<template>
  <div class="chat-layout" :class="{ 'has-session': !!current }">
    <section class="conversation-list" aria-label="会话列表">
      <div v-for="s in activeSessions" :key="s.id" class="session-row">
        <button class="session-item" :class="{ selected: current?.id === s.id }" @click="navigate(`/sessions/${s.id}`)"><span class="session-icon"><i aria-hidden="true" :class="s.onboarding ? 'pi pi-compass' : 'pi pi-comment'"/></span><span><strong>{{ s.title }}</strong><small>{{ s.messages.at(-1)?.text }}</small></span></button>
        <SessionActions :id="s.id" :title="s.title"/>
      </div>
      <div v-if="!activeSessions.length" class="list-empty">暂无会话<br/>点击右上角 + 开始。</div>
    </section>
    <section v-if="current" class="conversation-detail" aria-label="会话详情">
      <div class="conversation-header"><Button icon="pi pi-arrow-left" text aria-label="返回会话列表" @click="navigate('/sessions')"/><div class="conversation-title"><h2>{{ current.title }}</h2><small>本机会话 · 执行设备：{{ deviceName(selectedTarget) }}</small></div><span class="local-badge">本机 agent</span><Button v-if="!archived" icon="pi pi-inbox" text aria-label="归档会话" @click="archiveSession(current.id)"/></div>
      <div v-if="archived" class="archived-banner"><div><strong>会话已归档</strong><p>可查看历史与任务进度。恢复后可继续对话，归档不会取消任务。</p></div><div class="archive-actions"><Button label="恢复会话" icon="pi pi-replay" @click="restoreSession(current.id)"/><Button label="归档管理" text @click="navigate('/archives')"/></div></div>
      <div class="messages" role="log" aria-label="对话消息" aria-live="polite">
        <template v-for="entry in timeline" :key="entry.id">
          <div v-if="entry.kind === 'message'" class="message" :class="entry.message.role">
            <div class="avatar"><i aria-hidden="true" :class="entry.message.role === 'assistant' ? 'pi pi-compass' : 'pi pi-user'"/></div>
            <div class="message-content"><span class="message-author">{{ entry.message.role === 'assistant' ? 'rove-agent' : '你' }}</span><div class="message-bubble"><AssistantReply v-if="entry.message.role === 'assistant'" :text="entry.message.text"/><p v-else>{{ entry.message.text }}</p></div></div>
          </div>
          <div v-else class="message assistant task-message" :data-testid="`run-${entry.run.id}`">
            <div class="avatar"><i aria-hidden="true" class="pi pi-compass"/></div>
            <div class="message-content"><span class="message-author">rove-agent</span>
              <div class="message-bubble">
                <AssistantReply :text="entry.reply ?? (entry.run.output_text || runText(entry.run))" :streaming="entry.run.status === 'running' && !entry.run.waiting_connection"/>
                <div class="bubble-meta"><span class="status-label" :class="{ bad: entry.run.status === 'failed' }">{{ entry.run.waiting_connection ? '等待连接 · 最后已知状态' : statuses[entry.run.status] }}</span><small>{{ deviceName(entry.run.device_id) }} · 模拟</small></div>
                <details class="execution-details"><summary>执行详情</summary><p>执行设备：{{ deviceName(entry.run.device_id) }}<br/>模型：{{ modelLabel(entry.run.model_id) }}<br/>任务 ID：{{ entry.run.id }}</p><small>模拟任务保存在此浏览器；退出页面不会取消演示任务，不代表真实后台执行。</small></details>
                <Button v-if="!archived && ['queued','running'].includes(entry.run.status)" label="取消任务" text size="small" @click="perform(() => api.cancelRun(entry.run.id))"/>
              </div>
            </div>
          </div>
        </template>
        <ConversationSuggestions v-if="!archived && showSuggestions" @select="chooseSuggestion"/>
        <div v-if="!archived && current.onboarding && !data.networks.length" class="onboarding-actions"><Button label="创建我的网络" icon="pi pi-plus" @click="addNetwork"/><Button label="加入已有网络" icon="pi pi-qrcode" outlined @click="ui.joinDialog = true"/></div>
      </div>
      <div v-if="!archived" class="composer-area"><div v-if="attachmentNames.length" class="attachment-strip"><span v-for="name in attachmentNames" :key="name"><i aria-hidden="true" class="pi pi-paperclip"/> {{ name }}</span><Button icon="pi pi-times" text aria-label="移除附件" @click="attachmentNames = []"/></div><div class="composer"><Textarea v-model="draft" aria-label="消息" placeholder="例如：在书房电脑上安装音乐播放器…" rows="2" auto-resize @keydown.ctrl.enter="send" @keydown.meta.enter="send"/><div class="composer-toolbar"><div class="attachment-control"><Button text icon="pi pi-plus" aria-label="添加附件或选择模型" :aria-expanded="attachmentMenu" @click="attachmentMenu = !attachmentMenu"/><div v-if="attachmentMenu" class="menu-panel attachment-menu"><button v-if="mobilePlatform" @click="notice('拍照入口演示：本原型不调用相机，可使用照片或文件示例。'); attachmentMenu = false">拍照</button><button @click="fileInput?.click(); attachmentMenu = false">{{ mobilePlatform ? '照片 / 文件' : '文件' }}</button><button @click="chooseModel">模型</button></div></div><span class="composer-hint">Ctrl / ⌘ + Enter 发送</span><Button text icon="pi pi-microphone" aria-label="语音输入示例" @click="draft = '在书房电脑上安装音乐播放器'; notice('语音转文字示例已填入，请检查后发送；未录音。')"/><Button icon="pi pi-arrow-up" aria-label="发送消息" :disabled="!draft.trim() || !data.models.length" @click="send"/></div></div><div class="composer-context"><label>模型<Select ref="modelPicker" :model-value="current.model_id" :options="modelOptions" option-label="label" option-value="value" aria-label="会话模型" placeholder="选择模型" @update:model-value="value => perform(() => api.setSessionModel(current!.id, value))"/></label></div><p class="composer-footnote">演示数据 · 进度保存在此浏览器，不代表真实后台执行</p><input ref="fileInput" type="file" multiple hidden @change="files"/></div>
    </section>
    <section v-else-if="sid" class="empty-state"><h2>会话不存在或已删除</h2><p>这条会话无法打开，请返回列表。</p><Button label="返回会话列表" @click="navigate('/sessions')"/></section>
    <section v-else class="conversation-placeholder"><i class="pi pi-comments" aria-hidden="true"/><h2>从一段对话开始</h2><p>选择左侧会话，或从右上角 + 新建会话。</p></section>
  </div>
</template>
