<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue';
import Button from 'primevue/button';
const phase = ref(0), paused = ref(false), reduced = ref(false);
const steps = [
  { title: '创建你的网络', text: '在任意一台设备上创建一个漫游空间。', icon: 'pi-desktop' },
  { title: '分享网络名片', text: '把二维码或配置 URL 发给另一台设备。', icon: 'pi-qrcode' },
  { title: '另一台设备加入', text: '手机扫一扫，电脑导入 URL，也可以手动配置。', icon: 'pi-mobile' },
  { title: '设备，从此相连', text: '加入同一网络，就能发现设备并访问服务。', icon: 'pi-check-circle' },
];
let timer: ReturnType<typeof setInterval>;
let motion: MediaQueryList;
function preference() { reduced.value = motion.matches; if (reduced.value) paused.value = true; }
onMounted(() => { motion = matchMedia('(prefers-reduced-motion: reduce)'); preference(); motion.addEventListener('change', preference); timer = setInterval(() => { if (!paused.value && !reduced.value && !document.hidden) phase.value = (phase.value + 1) % 4; }, 2600); });
onUnmounted(() => { clearInterval(timer); motion?.removeEventListener('change', preference); });
function select(index: number) { phase.value = index; paused.value = true; }
</script>
<template>
  <div class="network-walkthrough" :class="{ paused, 'reduced-motion': reduced }">
    <div class="network-film" :data-phase="phase" aria-label="连接设备的动画演示">
      <span class="film-label">连接方式演示 · 非真实连接</span>
      <div class="film-network" :class="{ linked: phase === 3 }"><i class="pi pi-compass" aria-hidden="true"/><span>我的漫游空间</span></div>
      <div class="film-link link-left" :class="{ lit: phase >= 1 }"/><div class="film-link link-right" :class="{ lit: phase >= 2 }"/>
      <div class="film-device film-computer" :class="{ active: phase === 0 || phase === 3 }"><i class="pi pi-desktop" aria-hidden="true"/><strong>电脑</strong><small>{{ phase === 0 ? '创建网络' : '已准备好' }}</small></div>
      <div class="film-card" :class="{ visible: phase >= 1 }"><i class="pi pi-qrcode" aria-hidden="true"/><span>网络名片</span><small>QR / URL</small></div>
      <div class="film-device film-phone" :class="{ active: phase >= 2 }"><i :class="phase === 3 ? 'pi pi-check-circle' : 'pi pi-mobile'" aria-hidden="true"/><strong>另一台设备</strong><small>{{ phase === 3 ? '加入同一网络' : phase === 2 ? '扫码 / 导入' : '等待加入' }}</small></div>
      <div class="film-caption"><span>0{{ phase + 1 }}</span><strong>{{ steps[phase]!.title }}</strong></div>
    </div>
    <div class="film-controls"><span>{{ steps[phase]!.text }}</span><Button v-if="!reduced" text :icon="paused ? 'pi pi-play' : 'pi pi-pause'" :aria-label="paused ? '播放连接动画' : '暂停连接动画'" @click="paused = !paused"/></div>
    <ol class="film-steps" aria-label="连接设备步骤"><li v-for="(step, index) in steps" :key="step.title"><button :aria-current="phase === index ? 'step' : undefined" @click="select(index)"><span>{{ index + 1 }}</span>{{ step.title }}</button></li></ol>
  </div>
</template>
