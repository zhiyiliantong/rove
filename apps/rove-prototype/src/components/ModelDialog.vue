<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue';
import Dialog from 'primevue/dialog';
import Button from 'primevue/button';
import InputText from 'primevue/inputtext';
import Select from 'primevue/select';
import Checkbox from 'primevue/checkbox';
import RadioButton from 'primevue/radiobutton';
import { api, data, ui, perform, navigate, notice } from '../ui';
import { validateEndpoint } from '../mock';
import { providerConfig, RIG_VERSION } from '../providers';
import { generatedModelNames } from '../model-catalog';
import type { Connection } from '../domain';
const providers = api.listProviders();
const form = reactive({ name: '', provider: 'openai', base_url: '', auth_kind: 'api_key' as Connection['auth_kind'], key: 'demo-key', selected: [] as string[], manual: '', asDefault: false });
const verified = ref(false);
const customNames = reactive<Record<string, string>>({});
const error = ref('');
const catalog = ref(api.listModelCatalog({ provider: 'openai', auth_kind: 'api_key', refresh: false }));
const selectedTypes = computed(() => [...new Set([...form.selected, ...form.manual.split(/[,，\n]/).map(s => s.trim()).filter(Boolean)])]);
const generatedNames = computed(() => generatedModelNames(form.provider, selectedTypes.value, data.value.models.map(m => m.name)));
const modelNames = computed(() => selectedTypes.value.map((model, index) => customNames[model] ?? generatedNames.value[index]!));
const editing = computed(() => !!ui.editConnection);
const provider = computed(() => providerConfig(form.provider));
const legacyProvider = computed(() => editing.value && !providers.some(p => p.id === form.provider));
const providerOptions = computed(() => legacyProvider.value ? [...providers, { ...provider.value, id: form.provider, label: `${provider.value.id === form.provider ? provider.value.label : form.provider}（旧配置）` }] : providers);
watch(() => ui.modelDialog, open => {
  if (!open) { form.key = ''; return; }
  const c = data.value.connections.find(c => c.id === ui.editConnection);
  const providerId = c?.provider ?? 'openai';
  Object.assign(form, { name: c?.name ?? '', provider: providerId, base_url: c?.base_url ?? providerConfig(providerId).base_url, auth_kind: c?.auth_kind ?? 'api_key', key: 'demo-key', selected: [], manual: '', asDefault: !data.value.models.length });
  invalidate();
});
function invalidate() { verified.value = false; form.manual = ''; for (const model of Object.keys(customNames)) delete customNames[model]; error.value = ''; catalog.value = api.listModelCatalog({ provider: form.provider, auth_kind: form.auth_kind, refresh: false }); form.selected = catalog.value.models.map(model => model.id); }
function changeProvider() { form.base_url = provider.value.base_url; form.auth_kind = 'api_key'; form.key = 'demo-key'; invalidate(); }
function changeAuth() { form.key = form.auth_kind === 'api_key' ? 'demo-key' : ''; invalidate(); }
function checkCredentials() {
  if (form.auth_kind === 'official_agent') {
    if (!provider.value.account_adapter) throw new Error('此服务商尚未配置账号登录适配器，请使用 API 密钥。');
  } else if (form.key !== 'demo-key' && !(provider.value.allow_empty_key && !form.key)) throw new Error('请只使用 demo-key；本原型不接收真实 API 密钥。');
}
function verify() {
  try { validateEndpoint(form.base_url); checkCredentials(); catalog.value = api.listModelCatalog({ provider: form.provider, auth_kind: form.auth_kind, refresh: true }); verified.value = true; if (!form.selected.length) form.selected = catalog.value.models.map(m => m.id); error.value = ''; }
  catch (e) { verified.value = false; catalog.value = api.listModelCatalog({ provider: form.provider, auth_kind: form.auth_kind, refresh: false }); error.value = (e as Error).message; }
}
function save() {
  error.value = '';
  try {
    validateEndpoint(form.base_url);
    if (editing.value) api.editConnection(ui.editConnection, { name: form.name, base_url: form.base_url });
    else {
      checkCredentials();
      if (modelNames.value.some(name => !name.trim())) throw new Error('模型名称不能为空。');
      if (form.auth_kind === 'official_agent' && !verified.value) throw new Error('请先完成模拟账号登录。');
      const modelIds = api.importModels({ name: modelNames.value[0] ?? '', provider: form.provider, base_url: form.base_url, auth_kind: form.auth_kind, models: selectedTypes.value, model_names: Object.fromEntries(selectedTypes.value.map((model, index) => [model, modelNames.value[index]!])) });
      if (form.asDefault && modelIds[0]) api.setDefaultModel(modelIds[0]);
    }
    perform(() => undefined); ui.modelDialog = false;
    const onboarding = data.value.sessions.find(s => s.onboarding);
    if (onboarding && !data.value.networks.length) navigate(`/sessions/${onboarding.id}`); else navigate('/models');
    notice(editing.value ? '共享连接已更新，引用它的型号同步生效。' : '演示模型已保存，没有发送接口请求或保存真实密钥。');
  } catch (e) { error.value = (e as Error).message; }
}
</script>
<template>
  <Dialog v-model:visible="ui.modelDialog" modal :header="editing ? '编辑共享连接' : '添加模型'" :style="{ width: '35rem' }" :breakpoints="{ '640px': '94vw' }">
    <form class="stack-form" @submit.prevent="save">
      <p class="muted">配置一次连接，添加多个型号。此处仅使用演示数据。</p>
      <label v-if="editing" for="model-name">配置名称<InputText id="model-name" v-model="form.name" required maxlength="80"/></label>
      <label for="provider">服务商<Select input-id="provider" aria-label="服务商" v-model="form.provider" :options="providerOptions" option-label="label" option-value="id" filter :filter-fields="['label', 'id', 'keywords', 'category']" :pt="{ pcFilter: { root: { 'aria-label': '搜索服务商' } } }" filter-placeholder="搜索厂商、型号系列或平台" empty-filter-message="没有匹配的 Rig 供应商，请更换关键词" :disabled="editing" @update:model-value="changeProvider"/></label>
      <small class="provider-note">{{ provider.category }} · {{ provider.note }}</small>
      <small v-if="legacyProvider" class="warning-text">此供应商已不在新增目录中；保留旧配置、型号和会话，可编辑连接，不会自动转换为其他供应商。</small>
      <small v-else class="rig-provider-note">依据 Rig {{ RIG_VERSION }} / {{ provider.rig_provider }} 筛选；仅表示 Rig 有适配器，Rove 尚未实接，型号能力仍待验证。</small>
      <label for="endpoint">接口地址<InputText id="endpoint" v-model="form.base_url" required type="url" @update:model-value="invalidate"/></label>
      <small>切换服务商会填入默认接口地址，也可以自行修改。原型不会访问此地址。</small>
      <small v-if="form.base_url.includes('localhost') || form.base_url.includes('127.0.0.1')" class="warning-text">同步到其他设备后，本地地址会指向目标设备自己。</small>
      <fieldset class="auth-methods">
        <legend>认证方式（二选一）</legend>
        <label class="check-row" for="auth-api"><RadioButton input-id="auth-api" v-model="form.auth_kind" value="api_key" :disabled="editing" @update:model-value="changeAuth"/>API 密钥</label>
        <label class="check-row" for="auth-account"><RadioButton input-id="auth-account" v-model="form.auth_kind" value="official_agent" :disabled="editing || !provider.account_adapter" @update:model-value="changeAuth"/>账号登录</label>
        <small v-if="!provider.account_adapter">此服务商尚未接入账号登录，使用 API 密钥。</small>
      </fieldset>
      <template v-if="!editing">
        <template v-if="form.auth_kind === 'api_key'">
          <label for="api-key">API 密钥（仅 demo-key）<InputText id="api-key" v-model="form.key" type="password" autocomplete="off" placeholder="仅输入 demo-key" @update:model-value="invalidate"/></label>
          <small v-if="provider.allow_empty_key">本地免密服务演示可留空；不要在本原型输入真实密钥。</small>
          <Button label="模拟验证并获取型号" icon="pi pi-refresh" outlined @click="verify"/>
        </template>
        <template v-else>
          <p class="inline-info">通过 {{ provider.account_adapter }} 官方适配器登录，使用对应账号权益。会员登录不等于通用 API 授权，接口地址不用于接收账号凭据。本原型不打开登录网站。</p>
          <Button :label="`模拟 ${provider.account_adapter} 账号登录`" icon="pi pi-external-link" outlined @click="verify"/>
        </template>
        <span v-if="verified" class="success-text">演示认证完成 · 未验证真实模型可用性</span>
        <fieldset class="catalog-list"><legend>选择型号（可多选）</legend>
          <p class="field-help">{{ catalog.message }}</p><small>参考目录核对日期：{{ catalog.checked_on }}。型号 ID 保持厂商原值。</small>
          <label v-for="model in catalog.models" :key="model.id" class="check-row catalog-option" :for="'catalog-' + model.id"><Checkbox v-model="form.selected" :value="model.id" :input-id="'catalog-' + model.id"/><span><strong>{{ model.label }}</strong><code>{{ model.id }}</code><small>{{ model.note }}</small></span></label>
          <p v-if="!catalog.models.length" class="field-help">自定义接口没有通用预置清单，请在下方填写其支持的型号。</p>
        </fieldset>
        <label for="manual-model">手动补充型号<InputText id="manual-model" v-model="form.manual" placeholder="多个型号用逗号分隔"/></label>
        <div class="model-name-preview"><strong>模型名称（自动生成，可修改）</strong><p v-if="!generatedNames.length" class="field-help">选择型号后，按“提供商-型号-序号”生成名称。</p>
          <div v-for="(model, index) in selectedTypes" :key="model" class="model-name-field">
            <label :for="'model-name-' + index"><code>{{ model }}</code><InputText :id="'model-name-' + index" :aria-label="'模型名称 ' + model" :model-value="modelNames[index]" @update:model-value="customNames[model] = $event ?? ''"/></label>
            <Button v-if="customNames[model] !== undefined" label="恢复自动名称" :aria-label="'恢复自动名称 ' + model" text size="small" @click="delete customNames[model]"/>
          </div>
          <small>每个型号可单独改名，名称不能为空或重复；修改名称不改变型号 ID，共享本次连接凭据。</small>
        </div>
        <label class="check-row"><Checkbox v-model="form.asDefault" binary input-id="default-new-model"/>将本次第一个型号设为默认</label>
      </template>
      <p v-if="editing" class="inline-info">修改将影响该连接下的所有型号。真实凭据更新将在正式接入阶段实现。</p>
      <p v-if="error" role="alert" class="form-error">{{ error }}</p>
      <div class="form-actions"><Button label="取消" text @click="ui.modelDialog = false"/><Button type="submit" label="保存模型"/></div>
    </form>
  </Dialog>
</template>
