<script setup lang="ts">
import { computed } from 'vue';
import Button from 'primevue/button';
import ConversationSuggestions from '../components/ConversationSuggestions.vue';
import NetworkWalkthrough from '../components/NetworkWalkthrough.vue';
import { introduction, introStep, finishIntroduction, startIntroConversation, data, addModel, addNetwork, ui } from '../ui';
const pages = ['欢迎', '添加模型', '连接设备', '开始对话'];
const network = computed(() => data.value.networks[0]);
</script>
<template>
  <section id="main-content" tabindex="-1" class="introduction" aria-label="使用引导">
    <header class="intro-header"><span class="intro-brand"><i class="pi pi-compass" aria-hidden="true"/> rove</span><span class="intro-demo">交互原型 · 模拟数据</span><Button label="跳过引导" text @click="finishIntroduction()"/></header>
    <nav class="intro-progress" aria-label="引导步骤"><button v-for="(name, index) in pages" :key="name" :aria-current="introduction.step === index ? 'step' : undefined" @click="introStep(index)"><span>{{ index + 1 }}</span><strong>{{ name }}</strong></button></nav>
    <div class="intro-content" :class="`intro-step-${introduction.step}`">
      <template v-if="introduction.step === 0">
        <div class="intro-hero" aria-hidden="true"><div class="hero-orbit orbit-one"/><div class="hero-orbit orbit-two"/><span class="hero-center"><i class="pi pi-compass"/></span><span class="hero-device hero-pc"><i class="pi pi-desktop"/></span><span class="hero-device hero-mobile"><i class="pi pi-mobile"/></span><span class="hero-device hero-server"><i class="pi pi-server"/></span><span class="hero-device hero-code"><i class="pi pi-code"/></span></div>
        <span class="eyebrow">YOUR DEVICES. YOUR WORLD.</span><h1>你的设备，<span>随你漫游</span></h1><p class="intro-slogan">数据自己掌握，跳出平台控制</p><p class="intro-description">连接手机、电脑和家里的服务器，<br/>用一句话，让自己的设备为你协作。</p>
        <Button label="开始" icon="pi pi-arrow-right" icon-pos="right" @click="introStep(1)"/>
      </template>
      <template v-else-if="introduction.step === 1">
        <span class="intro-symbol"><i class="pi pi-sparkles" aria-hidden="true"/></span><span class="eyebrow">01 / YOUR AI</span><h1>先认识你的 AI</h1><p class="intro-description">添加一个模型，让 Rove 理解你想做什么。<br/>配置一次连接，可以选择多个型号。</p>
        <div class="intro-model-card"><i class="pi pi-comments" aria-hidden="true"/><div><h2>{{ data.models.length ? '模型已准备好' : '从一个模型开始' }}</h2><p>{{ data.models.length ? `已添加 ${data.models.length} 个型号，现有配置会保留。` : '选择服务商，使用 API 密钥或受支持的账号登录。' }}</p><small>原型只接受 demo-key 或模拟登录，不输入真实凭据。</small></div><i v-if="data.models.length" class="pi pi-check-circle" aria-label="已配置"/></div>
        <div class="intro-actions"><Button :label="data.models.length ? '再添加模型' : '添加模型'" icon="pi pi-plus" :outlined="!!data.models.length" @click="addModel"/><Button v-if="data.models.length" label="下一步" icon="pi pi-arrow-right" icon-pos="right" @click="introStep(2)"/><Button v-else label="稍后添加" text @click="introStep(2)"/></div>
        <div v-if="data.models.length" class="intro-review-reset"><small>想从没有配置的首次使用重新审核？</small><Button label="清空演示数据并重新开始" text size="small" @click="ui.reviewReset=true"/></div>
      </template>
      <template v-else-if="introduction.step === 2">
        <span class="eyebrow">02 / CONNECT YOUR WORLD</span><h1>让设备，彼此相连</h1><p class="intro-description">当前设备已准备好。创建或加入网络，连接你的其他设备。</p>
        <NetworkWalkthrough/>
        <a class="tutorial-download" href="/tutorial/network-join-v1.gif" download="rove-连接网络与设备.gif"><i class="pi pi-download" aria-hidden="true"/> 下载演示 GIF</a>
        <div class="intro-network-status" v-if="network"><i class="pi pi-check-circle" aria-hidden="true"/><span>已保存 {{ data.networks.length }} 个演示网络</span><Button label="查看网络名片" text size="small" @click="ui.shareNetwork = network!.id"/></div>
        <div class="intro-actions"><Button label="创建网络" icon="pi pi-plus" @click="addNetwork"/><Button label="加入网络" icon="pi pi-sign-in" outlined @click="ui.joinDialog = true"/></div><p class="intro-trust">网络名片含入网密钥，只分享给你信任的人。</p>
        <Button :label="network ? '下一步' : '稍后设置'" text icon="pi pi-arrow-right" icon-pos="right" @click="introStep(3)"/>
      </template>
      <template v-else>
        <span class="intro-symbol"><i class="pi pi-comment" aria-hidden="true"/></span><span class="eyebrow">03 / MAKE IT YOURS</span><h1>想做什么，聊聊就好</h1><p class="intro-description">从一个想法开始，也可以开启空白对话。<br/>Rove 负责连接设备，开源软件带来更多可能。</p>
        <ConversationSuggestions @select="startIntroConversation"/>
        <p class="intro-draft-note">点击提示会准备一份草稿，由你确认发送。</p><Button label="新会话" icon="pi pi-plus" @click="startIntroConversation()"/>
      </template>
    </div>
    <footer class="intro-footer"><Button v-if="introduction.step > 0" label="上一步" icon="pi pi-arrow-left" text @click="introStep(introduction.step - 1)"/><span v-else>每台设备，都是起点。</span><span>{{ introduction.step + 1 }} / 4</span><Button v-if="introduction.step === 3" label="进入会话列表" text @click="finishIntroduction()"/><span v-else>随时可以稍后设置</span></footer>
  </section>
</template>
