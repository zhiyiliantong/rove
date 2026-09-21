<script setup lang="ts">
import { computed, defineAsyncComponent, onMounted, onUnmounted, ref, watch } from 'vue';
import Button from 'primevue/button';
import Select from 'primevue/select';
import Dialog from 'primevue/dialog';
import { api, data, ui, route, navigate, refresh, connectedNetworks, networkSummary, platform, mobilePlatform, newSession, addModel, addNetwork, notice } from './ui';
import ChatPage from './pages/ChatPage.vue';
import IntroductionPage from './pages/IntroductionPage.vue';
import { showingIntroduction } from './ui';
const ArchivesPage = defineAsyncComponent(() => import('./pages/ArchivesPage.vue'));
const ResourcesPage = defineAsyncComponent(() => import('./pages/ResourcesPage.vue'));
const SettingsPage = defineAsyncComponent(() => import('./pages/SettingsPage.vue'));
import ModelDialog from './components/ModelDialog.vue';
import NetworkDialogs from './components/NetworkDialogs.vue';
import ReviewResetDialog from './components/ReviewResetDialog.vue';
const links = [{ id: 'sessions', label: '会话', icon: 'pi-comments' }, { id: 'services', label: '服务', icon: 'pi-th-large' }, { id: 'networks', label: '网络', icon: 'pi-share-alt' }];
const section = computed(() => route.value === '/review-reset' ? 'sessions' : route.value.split('/')[1] || 'sessions');
const title = computed(() => ({ archives: '归档管理', sessions: '会话', services: '服务', networks: '网络', models: '模型', settings: '设置' }[section.value] ?? '会话'));
const navSection = computed(() => section.value === 'archives' ? 'sessions' : section.value);
const dark = ref(false);

const platforms = [{ label: '浏览器', value: 'browser' }, { label: '桌面外观', value: 'desktop' }, { label: '移动外观', value: 'mobile' }];
watch(dark, value => { document.documentElement.classList.toggle('dark', value); try { localStorage.setItem('rove-prototype-theme', value ? 'dark' : 'light'); } catch {} });
const hash = () => { route.value = location.hash.slice(1) || '/sessions'; };
function focusMain() { document.getElementById('main-content')?.focus(); }
let timer: ReturnType<typeof setInterval>;
onMounted(() => { try { dark.value = localStorage.getItem('rove-prototype-theme') === 'dark'; } catch {} window.addEventListener('hashchange', hash); timer = setInterval(refresh, 150); });
onUnmounted(() => { clearInterval(timer); window.removeEventListener('hashchange', hash); });
</script>

<template>
  <div class="app-shell" :data-platform="platform" :class="{ 'mobile-platform': mobilePlatform, 'intro-shell': showingIntroduction }">
    <IntroductionPage v-if="showingIntroduction"/>
    <div v-if="showingIntroduction && ui.notice" class="notice intro-notice" :class="{error: ui.error}" :role="ui.error ? 'alert' : 'status'"><span>{{ ui.notice }}</span><Button text icon="pi pi-times" aria-label="关闭提示" @click="ui.notice = ''"/></div>
    <a class="skip-link" href="#main-content" @click.prevent="focusMain">跳到主要内容</a>
    <aside v-if="!showingIntroduction" class="sidebar">
      <a class="brand" href="#/sessions"><span class="brand-mark"><i aria-hidden="true" class="pi pi-compass" /></span><span>rove<span class="brand-caption">漫游者</span></span></a>
      <div class="nav-label">你的漫游空间</div>
      <nav aria-label="主导航"><a v-for="link in links" :key="link.id" :href="`#/${link.id}`" :class="{ active: navSection === link.id }" :aria-current="navSection === link.id ? 'page' : undefined"><i aria-hidden="true" :class="`pi ${link.icon}`" />{{ link.label }}<i aria-hidden="true" v-if="navSection === link.id" class="pi pi-angle-right nav-arrow" /></a></nav>
      <div class="sidebar-bottom"><div class="connection-summary"><span class="status-dot" :class="{ off: !connectedNetworks.length }"/><div><strong>{{ connectedNetworks.length ? networkSummary : '尚未连接网络' }}</strong><small>{{ connectedNetworks.length ? '本机已连接 · 模拟' : '从一段会话开始' }}</small></div></div><a class="settings-link" href="#/settings"><i aria-hidden="true" class="pi pi-cog"/>设置</a><span class="version">ROVE PROTOTYPE / 0.6</span></div>
    </aside>
    <div v-if="!showingIntroduction" class="main-shell">
      <div v-if="platform === 'desktop'" class="window-chrome"><span>Rove · 桌面外观演示</span><div><Button v-for="(icon, i) in ['pi-minus', 'pi-window-maximize', 'pi-times']" :key="icon" text :icon="`pi ${icon}`" :aria-label="['模拟最小化','模拟最大化','模拟关闭'][i]" @click="notice('窗口控件仅演示外观，浏览器窗口不会被改变。')"/></div></div>
      <header class="topbar"><div class="topbar-title"><span class="mobile-brand"><i aria-hidden="true" class="pi pi-compass"/></span><span>{{ title }}</span><template v-if="section !== 'sessions'"><span class="separator">/</span><span class="network-context">{{ networkSummary }}</span></template></div><div class="topbar-actions"><span class="demo-pill">交互原型 · 模拟数据</span><Button text rounded :icon="dark ? 'pi pi-sun' : 'pi pi-moon'" aria-label="切换明暗主题" @click="dark = !dark"/><div class="global-menu"><Button icon="pi pi-plus" rounded aria-label="全局添加" aria-haspopup="menu" :aria-expanded="ui.menu" @click="ui.menu = !ui.menu"/><div v-if="ui.menu" class="menu-panel" role="menu" @keydown.esc="ui.menu = false"><button role="menuitem" @click="newSession">新会话</button><button role="menuitem" @click="navigate('/archives')">归档管理</button><button role="menuitem" @click="addNetwork(); ui.menu = false">添加网络</button><button role="menuitem" @click="addModel(); ui.menu = false">添加模型</button><button role="menuitem" @click="ui.joinDialog = true; ui.menu = false">加入网络</button><button role="menuitem" @click="navigate('/settings')">设置</button></div></div></div></header>
      <div v-if="ui.notice" class="notice" :class="{ error: ui.error }" :role="ui.error ? 'alert' : 'status'"><span>{{ ui.notice }}</span><Button text icon="pi pi-times" aria-label="关闭提示" @click="ui.notice = ''"/></div>
      <main id="main-content" tabindex="-1">
        <ArchivesPage v-if="section === 'archives'"/>
        <ChatPage v-else-if="section === 'sessions'"/>
        <ResourcesPage v-else-if="['networks','services'].includes(section)" :section="section"/>
        <SettingsPage v-else-if="['settings','models'].includes(section)" :section="section" v-model:dark="dark"/>
        <div v-else class="empty-state"><h1>页面不存在</h1><Button label="返回会话" @click="navigate('/sessions')"/></div>
      </main>
      <footer class="preview-footer"><span>每台设备，都是起点。</span><label for="platform-mode">外观预览</label><Select input-id="platform-mode" aria-label="外观预览" v-model="platform" :options="platforms" option-label="label" option-value="value" size="small"/></footer>
    </div>
    <nav v-if="!showingIntroduction" class="bottom-nav" aria-label="移动主导航"><a v-for="link in links" :key="link.id" :href="`#/${link.id}`" :class="{ active: navSection === link.id }" :aria-current="navSection === link.id ? 'page' : undefined"><i aria-hidden="true" :class="`pi ${link.icon}`"/><span>{{ link.label }}</span></a></nav>
    <ModelDialog/><NetworkDialogs/><ReviewResetDialog/>
    <Dialog :visible="!!ui.servicePreview" modal header="服务访问演示" @update:visible="ui.servicePreview = ''" :style="{ width: '32rem' }" :breakpoints="{ '640px': '94vw' }"><div class="service-demo"><i aria-hidden="true" class="pi pi-headphones"/><h2>{{ data.services.find(s => s.id === ui.servicePreview)?.name }}</h2><p>这里将打开设备上的服务应用。</p><p>这是静态预览，没有连接任何真实 IP，也不会发送认证信息。</p><Button label="返回 Rove" outlined @click="ui.servicePreview = ''"/></div></Dialog>
  </div>
</template>
