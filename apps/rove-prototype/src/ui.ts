import { computed, nextTick, reactive, ref } from 'vue';
import { manageRovePrompt } from './conversation-prompts';
import { createMockApi } from './mock';
import type { PrototypeApi, Snapshot } from './domain';
import { loadIntroduction, saveIntroduction } from './introduction';
import { persistCleanReview } from './review-reset';
let storage: Storage | undefined;
try { storage = window.localStorage; } catch { /* Private browser mode. */ }
export const api: PrototypeApi = createMockApi(storage, () => Date.now(), 'empty');
export const introduction = reactive(loadIntroduction(storage));
export const showingIntroduction = computed(() => !introduction.completed);
const pendingIntroPrompt = ref<string | null>(null);
export function introStep(step: number) {
  introduction.step = Math.max(0, Math.min(3, step));
  saveIntroduction(storage, introduction);
}
export function finishIntroduction(path = '/sessions') {
  introduction.completed = true; pendingIntroPrompt.value = null;
  if (!saveIntroduction(storage, introduction)) notice('浏览器无法保存引导偏好，下次打开可能再次显示引导。', true);
  navigate(path);
}
export function resetIntroduction() {
  const saved = saveIntroduction(storage, { ...introduction, resetPending: true });
  if (saved) introduction.resetPending = true;
  notice(saved ? '已安排下次启动重新显示使用引导。原型刷新或重新打开即生效，配置与会话不会清除。' : '浏览器无法保存引导偏好，未能安排下次重置。请允许本地存储后重试。', !saved);
}
export function restartIntroductionNow() {
  Object.assign(introduction, { version: 1, completed: false, step: 0, resetPending: false });
  pendingIntroPrompt.value = null; saveIntroduction(storage, introduction);
}
export function clearPrototypeReview() {
  persistCleanReview(storage);
  api.reset('empty'); refresh(); restartIntroductionNow();
  Object.assign(ui, {modelDialog:false, networkDialog:false, joinDialog:false, editNetwork:'', editConnection:'', shareNetwork:'', servicePreview:'', deviceDetail:'', menu:false, reviewReset:false});
  navigate('/sessions');
  notice('演示数据已清空，无法恢复。现在从首次使用开始；正式 Rove 数据未受影响。');
}
export function startIntroConversation(prompt = '') {
  if (!data.value.models.length) {
    pendingIntroPrompt.value = prompt;
    notice('先添加一个模型，保存后会继续打开你选择的会话；不会自动发送。'); addModel(); return;
  }
  const sid = perform(() => api.createSession());
  if (!sid) return;
  if (prompt) perform(() => api.saveDraft(sid, prompt));
  finishIntroduction(`/sessions/${sid}`);
  nextTick(() => document.querySelector<HTMLTextAreaElement>('textarea[aria-label="消息"]')?.focus());
}
export function modelSavedInIntroduction(): boolean {
  if (!showingIntroduction.value) return false;
  if (pendingIntroPrompt.value !== null) startIntroConversation(pendingIntroPrompt.value);
  else introStep(2);
  return true;
}
export const platform = ref('browser');
const detectedMobile = /Android|iPhone|iPad|iPod/i.test(navigator.userAgent) || (navigator.platform === 'MacIntel' && navigator.maxTouchPoints > 1);
export const mobilePlatform = computed(() => platform.value === 'mobile' || (platform.value === 'browser' && detectedMobile));
export const data = ref<Snapshot>(api.snapshot());
export const ui = reactive({ notice: '', error: false, modelDialog: false, networkDialog: false, joinDialog: false, editNetwork: '', editConnection: '', shareNetwork: '', servicePreview: '', deviceDetail: '', menu: false, reviewReset:false });
export const route = ref(location.hash.slice(1) || '/sessions');
export function navigate(path: string) { location.hash = path; route.value = path; ui.menu = false; }
export function refresh() { data.value = api.snapshot(); }
export function notice(message: string, error = false) { ui.notice = message; ui.error = error; }
export function perform<T>(fn: () => T): T | undefined { try { const result = fn(); refresh(); return result; } catch (e) { refresh(); notice(e instanceof Error ? e.message : '操作失败，请重试。', true); } }
export const connectedNetworks = computed(() => data.value.networks.filter(n => n.connection_status === 'connected'));
export function isConnected(nid: string) { return connectedNetworks.value.some(n => n.id === nid); }
export const networkSummary = computed(() => connectedNetworks.value.length > 1 ? `${connectedNetworks.value.length} 个网络已连接` : connectedNetworks.value[0]?.name ?? '本机空间');
export function modelLabel(mid: string) { const m = data.value.models.find(m => m.id === mid); return m ? m.name : '未配置模型'; }
export function deviceName(did: string) { return did === 'local' ? '此设备' : data.value.devices.find(d => d.id === did)?.name ?? '未知设备'; }
export function newSession() { const sid = perform(() => api.createSession()); if (sid) navigate(`/sessions/${sid}`); }
export function archiveSession(id: string) {
  if (!perform(() => { api.archiveSession(id); return true; })) return;
  if (route.value === `/sessions/${id}`) navigate('/sessions');
  notice('会话已归档，可在归档管理恢复。归档不会取消进行中的任务。');
  nextTick(() => document.querySelector<HTMLButtonElement>('button[aria-label="全局添加"]')?.focus());
}
export function restoreSession(id: string) {
  if (!perform(() => { api.restoreSession(id); return true; })) return;
  notice('会话已恢复，原有消息和草稿已保留。');
}
export async function startRoveManagement() {
  if (!data.value.models.length) { notice('请先添加模型，再通过对话管理 Rove。手动设置仍可使用；当前仅为演示。'); addModel(); return; }
  const sid = perform(() => api.createSession());
  if (!sid) return;
  perform(() => { api.setSessionTarget(sid, 'local'); api.saveDraft(sid, manageRovePrompt); });
  navigate(`/sessions/${sid}`);
  notice('已准备本机检查草稿，请检查后发送。当前仅演示入口，不会扫描磁盘、清理文件或修改真实配置。');
  await nextTick();
  document.querySelector<HTMLTextAreaElement>('textarea[aria-label="消息"]')?.focus();
}
export function addModel() { ui.editConnection = ''; ui.modelDialog = true; }
export function addNetwork() { ui.editNetwork = ''; ui.networkDialog = true; }
