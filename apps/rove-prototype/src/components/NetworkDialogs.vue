<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue';
import QRCode from 'qrcode';
import Dialog from 'primevue/dialog';
import Button from 'primevue/button';
import InputText from 'primevue/inputtext';
import Textarea from 'primevue/textarea';
import Select from 'primevue/select';
import { api, data, ui, perform, notice, navigate, addNetwork, mobilePlatform } from '../ui';
import { exportCard, cardUrl, parseCard } from '../mock';
import { demoId } from '../ids';
import { DHCP, generateNetworkKey, cidrRange } from '../network';
import type { PeerProbe } from '../domain';
const peerRow = (value = '') => ({ id: demoId(), value, testing: false, result: null as PeerProbe | null, generation: 0 });
const form = reactive({ name: '', subnet: '10.44.0.0/24', network_key: '', peers: [peerRow('tcp://relay.example.invalid:11010')] });
const addressMode = ref('dhcp');
const addressModes = [{ label: '自动获取（DHCP）', value: 'dhcp' }, { label: '手动指定网段', value: 'manual' }];
const showKey = ref(false);
const normalizedSubnet = computed(() => { try { return cidrRange(form.subnet).cidr; } catch { return ''; } });
const error = ref('');
const importValue = ref('');
const importFile = ref<HTMLInputElement>();
const qr = ref('');
const sharing = computed(() => data.value.networks.find(n => n.id === ui.shareNetwork));
const joining = ref(false);
watch(() => ui.networkDialog, open => { if (!open) { joining.value = false; return; } const n = data.value.networks.find(n => n.id === ui.editNetwork); addressMode.value = !n || n.subnet === DHCP ? 'dhcp' : 'manual'; showKey.value = false; Object.assign(form, { name: n?.name ?? `n-${demoId().slice(0, 6)}`, subnet: n && n.subnet !== DHCP ? n.subnet : '10.44.0.0/24', network_key: n?.network_key ?? generateNetworkKey(), peers: (n?.initial_peers ?? ['tcp://relay.example.invalid:11010']).map(peerRow) }); error.value = ''; });
watch(() => ui.joinDialog, () => { error.value = ''; importValue.value = ''; });
watch(() => ui.shareNetwork, async () => { qr.value = ''; if (sharing.value) qr.value = await QRCode.toDataURL(cardUrl(sharing.value), { width: 260, margin: 2 }); });
function connect(nid: string) { return perform(() => { api.connectNetwork(nid); return true; }); }
function save() { try { const nid = api.saveNetwork({ name: form.name, subnet: addressMode.value === 'dhcp' ? DHCP : form.subnet, network_key: form.network_key, initial_peers: form.peers.map(p => p.value.trim()) }, ui.editNetwork || undefined); perform(() => undefined); if (!ui.editNetwork) connect(nid); ui.networkDialog = false; ui.shareNetwork = nid; navigate(`/networks/${nid}`); joining.value = false; } catch (e) { error.value = (e as Error).message; } }
function join() { try { const card = parseCard(importValue.value); const existing = data.value.networks.find(n => n.name === card.name && n.network_key === card.network_key); const nid = existing?.id ?? api.saveNetwork(card); perform(() => undefined); const connected = connect(nid); ui.joinDialog = false; navigate(`/networks/${nid}`); if (connected) notice('演示网络配置已导入；没有连接真实 overlay 网络。'); } catch (e) { error.value = (e as Error).message; } }
function invalidatePeer(peer: ReturnType<typeof peerRow>) { peer.generation++; peer.testing = false; peer.result = null; }
async function testPeer(peer: ReturnType<typeof peerRow>) { const generation = ++peer.generation; const value = peer.value.trim(); peer.testing = true; peer.result = null; try { const result = await api.probePeer(value); if (peer.generation === generation && peer.value.trim() === value && form.peers.includes(peer)) peer.result = result; } finally { if (peer.generation === generation) peer.testing = false; } }
async function readFile(event: Event) { const input = event.target as HTMLInputElement; const file = input.files?.[0]; if (file) { if (file.size > 65536) error.value = '演示配置文件不能超过 64 KB。'; else { importValue.value = await file.text(); notice('文件已读取，请检查并导入。'); } } input.value = ''; }
async function copy() { if (!sharing.value) return; try { await navigator.clipboard.writeText(cardUrl(sharing.value)); notice('演示名片 URL 已复制。'); } catch { notice('剪贴板不可用，请手动选择并复制下方 URL。', true); } }
function download(kind: 'json' | 'png') { if (!sharing.value) return; const a = document.createElement('a'); const url = kind === 'json' ? URL.createObjectURL(new Blob([exportCard(sharing.value)], { type: 'application/json' })) : qr.value; a.href = url; a.download = `rove-demo-network.${kind}`; a.click(); if (kind === 'json') setTimeout(() => URL.revokeObjectURL(url), 1000); }
</script>
<template>
  <Dialog v-model:visible="ui.networkDialog" modal :header="ui.editNetwork ? '编辑网络' : joining ? '手动加入网络' : '创建网络'" :style="{ width: '38rem' }" :breakpoints="{ '640px': '94vw' }">
    <form class="stack-form" @submit.prevent="save">
      <p class="muted">仅用于原型演示，请勿填入真实网络密钥。</p>
      <label for="network-name">网络名称<InputText id="network-name" v-model="form.name" required maxlength="80"/></label>
      <label for="address-mode">虚拟网段<Select input-id="address-mode" aria-label="虚拟网段获取方式" v-model="addressMode" :options="addressModes" option-label="label" option-value="value"/></label>
      <template v-if="addressMode === 'manual'">
        <label for="subnet">网段地址<InputText id="subnet" v-model="form.subnet" required placeholder="192.168.100.0/24"/></label>
        <small v-if="normalizedSubnet">规范化网段：{{ normalizedSubnet }}。连接时检查与其他已连接网络是否重叠。</small>
      </template>
      <p v-else class="field-help">由 EasyTier 自动分配虚拟 IP，再显示实际网段。没有可用节点时等待；若实际网段重叠，只拒绝新连接，不影响其他网络。原型约 2 秒返回模拟结果。</p>
      <label for="network-key">网络密钥（演示）</label>
      <div class="secret-field">
        <InputText id="network-key" v-model="form.network_key" :type="showKey ? 'text' : 'password'" required autocomplete="off" spellcheck="false"/>
        <Button :icon="showKey ? 'pi pi-eye-slash' : 'pi pi-eye'" :aria-label="showKey ? '隐藏网络密钥' : '显示网络密钥'" :aria-pressed="showKey" text @click="showKey = !showKey"/>
        <Button icon="pi pi-refresh" aria-label="重新生成网络密钥" text @click="form.network_key = generateNetworkKey()"/>
      </div>
      <small>默认随机生成 32 字节（64 位十六进制）。EasyTier 不强制十六进制，手动加入时保留原密钥格式。</small>
      <fieldset class="peer-editor">
        <legend>初始节点</legend>
        <div v-for="(peer, index) in form.peers" :key="peer.id" class="peer-entry">
          <label :for="'peer-' + peer.id">节点 {{ index + 1 }}</label>
          <InputText :id="'peer-' + peer.id" v-model="peer.value" required placeholder="tcp://relay.example.invalid:11010" @update:model-value="invalidatePeer(peer)"/>
          <div class="peer-actions">
            <Button :label="peer.testing ? '测试中' : '测试连通（模拟）'" :loading="peer.testing" :disabled="!peer.value.trim()" outlined size="small" :aria-label="'测试节点 ' + (index + 1)" @click="testPeer(peer)"/>
            <Button label="移除" text size="small" :disabled="form.peers.length === 1" :aria-label="'移除节点 ' + (index + 1)" @click="form.peers.splice(index, 1)"/>
          </div>
          <p v-if="peer.result" role="status" class="field-help" :class="{ 'form-error': peer.result.status !== 'reachable' }">{{ peer.result.message }}</p>
        </div>
        <Button label="添加初始节点" icon="pi pi-plus" text @click="form.peers.push(peerRow())"/>
      </fieldset>
      <small>每条独立测试，编辑地址后结果失效。原型不发送 TCP / UDP 请求；可用 timeout.example.invalid 演示超时。</small>
      <p v-if="error" role="alert" class="form-error">{{ error }}</p>
      <div class="form-actions"><Button label="取消" text @click="ui.networkDialog = false"/><Button label="保存网络" type="submit"/></div>
    </form>
  </Dialog>
  <Dialog v-model:visible="ui.joinDialog" modal header="加入网络" :style="{ width: '35rem' }" :breakpoints="{ '640px': '94vw' }"><div class="stack-form"><p>从另一台设备拿到网络名片，即可加入同一个漫游空间。</p><div class="join-methods"><Button v-if="mobilePlatform" label="扫一扫（演示）" icon="pi pi-qrcode" outlined @click="notice('本原型不启用摄像头。可复制演示名片 URL，或导入演示配置文件。')"/><Button label="导入配置文件" icon="pi pi-upload" outlined @click="importFile?.click()"/></div><label for="join-url">演示网络 URL / JSON 配置<Textarea id="join-url" v-model="importValue" rows="4" placeholder="粘贴本原型导出的网络名片"/></label><p v-if="error" role="alert" class="form-error">{{ error }}</p><div class="form-actions"><Button label="手动配置" text @click="ui.joinDialog = false; addNetwork(); joining = true"/><Button label="导入并连接" :disabled="!importValue.trim()" @click="join"/></div><input ref="importFile" type="file" accept="application/json,.json" hidden @change="readFile"/></div></Dialog>
  <Dialog :visible="!!sharing" modal header="网络名片" @update:visible="ui.shareNetwork = ''" :style="{ width: '32rem' }" :breakpoints="{ '640px': '94vw' }"><div v-if="sharing" class="network-card"><span class="eyebrow">ROVE / YOUR SHARED SPACE</span><h2>{{ sharing.name }}</h2><p class="muted">{{ sharing.subnet === DHCP ? 'DHCP 自动获取 · 实际网段入网后确认' : sharing.subnet }}</p><img v-if="qr" :src="qr" alt="演示网络名片二维码" width="260" height="260"/><span class="demo-pill">仅供原型互相导入 · 非真实入网二维码</span><p>名片含入网密钥。分享即允许对方加入并控制网络内设备，请只发给信任的人。</p><label for="share-url">加入网络 URL</label><Textarea id="share-url" :model-value="cardUrl(sharing)" readonly rows="3"/><div class="join-methods"><Button label="复制 URL" icon="pi pi-copy" @click="copy"/><Button label="导出配置" outlined @click="download('json')"/><Button label="保存二维码" text @click="download('png')"/></div></div></Dialog>
</template>
