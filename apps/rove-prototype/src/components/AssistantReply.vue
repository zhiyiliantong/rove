<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import Button from 'primevue/button';
import Textarea from 'primevue/textarea';
import { renderRichText } from '../rich-text';
import 'katex/dist/katex.min.css';
const props = defineProps<{ text: string; streaming?: boolean }>();
const rich = computed(() => renderRichText(props.text));
const content = ref<HTMLElement>();
const feedback = ref('');
const manualCopy = ref<string | null>(null);
const speaking = ref(false);
let utterance: SpeechSynthesisUtterance | null = null;
async function copy(value: string) {
  try { if (!navigator.clipboard?.writeText) throw new Error(); await navigator.clipboard.writeText(value); manualCopy.value = null; feedback.value = '已复制'; }
  catch { manualCopy.value = value; feedback.value = '剪贴板不可用，请选择下方文本手动复制。'; }
}
function segment(event: MouseEvent) {
  const button = (event.target as Element).closest<HTMLButtonElement>('button[data-copy-index]');
  if (!button || !content.value?.contains(button)) return;
  const value = rich.value.copies[Number(button.dataset.copyIndex)]; if (value !== undefined) void copy(value);
}
function stop() { if (utterance) { utterance.onend = null; utterance.onerror = null; window.speechSynthesis?.cancel(); utterance = null; } speaking.value = false; }
function read() {
  if (speaking.value) { stop(); return; }
  if (!window.speechSynthesis || !window.SpeechSynthesisUtterance) { feedback.value = '此浏览器不支持朗读，请复制文本使用系统朗读。'; return; }
  const voice = window.speechSynthesis.getVoices().find(v => v.localService && /^zh/i.test(v.lang)) ?? window.speechSynthesis.getVoices().find(v => v.localService);
  if (!voice) { feedback.value = '没有可用的本地朗读语音；请安装系统语音后重试。本原型不会把回复发送到云端朗读。'; return; }
  const clone = content.value?.cloneNode(true) as HTMLElement | undefined;
  clone?.querySelectorAll('button, .katex-html').forEach(el => el.remove());
  const text = clone?.textContent?.trim() || props.text;
  // Limit one utterance so browsers that reject overly long text report a useful boundary.
  if (text.length > 4000) { feedback.value = '这条回复较长，请复制需要的段落使用系统朗读（单次最多 4000 字符）。'; return; }
  window.speechSynthesis.cancel();
  utterance = new SpeechSynthesisUtterance(text); utterance.voice = voice; utterance.lang = voice.lang;
  utterance.onend = () => { speaking.value = false; utterance = null; };
  utterance.onerror = () => { speaking.value = false; utterance = null; feedback.value = '朗读失败，请检查系统语音后重试。'; };
  try { speaking.value = true; window.speechSynthesis.speak(utterance); } catch { stop(); feedback.value = '无法启动系统朗读。'; }
}
watch(() => props.text, () => { if (speaking.value) stop(); });
onBeforeUnmount(stop);
</script>
<template>
  <div class="assistant-reply">
    <div ref="content" class="rich-text" @click="segment" v-html="rich.html"/>
    <span v-if="streaming" class="streaming-label" role="status">正在回复… · 模拟流式</span>
    <div class="reply-actions" role="group" aria-label="回复操作">
      <Button aria-label="复制回复" title="复制回复" icon="pi pi-copy" text rounded severity="secondary" :disabled="!text" @click="copy(text)"/>
      <Button :aria-label="speaking ? '停止朗读' : '朗读回复'" :title="speaking ? '停止朗读' : '朗读回复'" :aria-pressed="speaking" :icon="speaking ? 'pi pi-stop' : 'pi pi-volume-up'" text rounded severity="secondary" :disabled="!text || streaming" @click="read"/>
    </div>
    <small v-if="feedback" role="status">{{ feedback }}</small>
    <div v-if="manualCopy !== null" class="manual-copy"><Textarea :model-value="manualCopy" aria-label="手动复制文本" readonly rows="3" @focus="($event.target as HTMLTextAreaElement).select()"/><Button label="关闭手动复制" text size="small" @click="manualCopy = null; feedback = ''"/></div>
  </div>
</template>
