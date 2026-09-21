<script setup lang="ts">
import {t,notice_text,ui_locale} from '../i18n';
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
  try { if (!navigator.clipboard?.writeText) throw new Error(); await navigator.clipboard.writeText(value); manualCopy.value = null; feedback.value = ''; }
  catch { manualCopy.value = value; feedback.value = t('ui.clipboard_unavailable_select_the_text_below_and_copy'); }
}
function segment(event: MouseEvent) {
  const button = (event.target as Element).closest<HTMLButtonElement>('button[data-copy-index]');
  if (!button || !content.value?.contains(button)) return;
  const value = rich.value.copies[Number(button.dataset.copyIndex)]; if (value !== undefined) void copy(value);
}
function stop() { if (utterance) { utterance.onend = null; utterance.onerror = null; window.speechSynthesis?.cancel(); utterance = null; } speaking.value = false; }
function read() {
  if (speaking.value) { stop(); return; }
  if (!window.speechSynthesis || !window.SpeechSynthesisUtterance) { feedback.value = t('ui.this_browser_cannot_read_aloud_copy_the_text'); return; }
  const voice = window.speechSynthesis.getVoices().find(v => v.localService && v.lang.toLowerCase().startsWith(ui_locale.value.split('-')[0])) ?? window.speechSynthesis.getVoices().find(v => v.localService);
  if (!voice) { feedback.value = t('ui.no_local_speech_voice_is_available_install_a'); return; }
  const clone = content.value?.cloneNode(true) as HTMLElement | undefined;
  clone?.querySelectorAll('button, .katex-html').forEach(el => el.remove());
  const text = clone?.textContent?.trim() || props.text;
  // Limit one utterance so browsers that reject overly long text report a useful boundary.
  if (text.length > 4000) { feedback.value = t('ui.this_reply_is_too_long_copy_the_relevant'); return; }
  window.speechSynthesis.cancel();
  utterance = new SpeechSynthesisUtterance(text); utterance.voice = voice; utterance.lang = voice.lang;
  utterance.onend = () => { speaking.value = false; utterance = null; };
  utterance.onerror = () => { speaking.value = false; utterance = null; feedback.value = t('ui.reading_failed_check_system_voices_and_try_again'); };
  try { speaking.value = true; window.speechSynthesis.speak(utterance); } catch { stop(); feedback.value = t('ui.unable_to_start_system_speech'); }
}
watch(() => props.text, () => { if (speaking.value) stop(); });
onBeforeUnmount(stop);
</script>
<template>
  <div class="assistant-reply">
    <div ref="content" class="rich-text" @click="segment" v-html="rich.html"/>
    <span v-if="streaming" class="streaming-label" role="status">{{t('ui.receiving_output')}}</span>
    <div class="reply-actions" role="group" :aria-label="t('ui.reply_actions')">
      <Button :aria-label="t('ui.copy_reply')" :title="t('ui.copy_reply')" icon="pi pi-copy" text rounded severity="secondary" :disabled="!text" @click="copy(text)"/>
      <Button :aria-label="speaking ? t('ui.stop_reading') : t('ui.read_reply_aloud')" :title="speaking ? t('ui.stop_reading') : t('ui.read_reply_aloud')" :aria-pressed="speaking" :icon="speaking ? 'pi pi-stop' : 'pi pi-volume-up'" text rounded severity="secondary" :disabled="!text || streaming" @click="read"/>
    </div>
    <small v-if="feedback" role="status">{{notice_text(feedback)}}</small>
    <div v-if="manualCopy !== null" class="manual-copy"><Textarea :model-value="manualCopy" :aria-label="t('ui.text_for_manual_copy')" readonly rows="3" @focus="($event.target as HTMLTextAreaElement).select()"/><Button :label="t('ui.close_manual_copy')" text size="small" @click="manualCopy = null; feedback = ''"/></div>
  </div>
</template>
