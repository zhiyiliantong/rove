<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue';
import Button from 'primevue/button';
import {t} from '../i18n';
const phase = ref(0), paused = ref(false), reduced = ref(false);
const steps = computed(() => ['create','share','join','connected'].map(key=>({title:t('intro.film_'+key),text:t('intro.film_'+key+'_text')})));
let timer: ReturnType<typeof setInterval>;
let motion: MediaQueryList;
function preference() { reduced.value = motion.matches; if (reduced.value) paused.value = true; }
onMounted(() => { motion = matchMedia('(prefers-reduced-motion: reduce)'); preference(); motion.addEventListener('change', preference); timer = setInterval(() => { if (!paused.value && !reduced.value && !document.hidden) phase.value = (phase.value + 1) % 4; }, 2600); });
onUnmounted(() => { clearInterval(timer); motion?.removeEventListener('change', preference); });
function select(index: number) { phase.value = index; paused.value = true; }
</script>
<template>
  <div class="network-walkthrough" :class="{ paused, 'reduced-motion': reduced }">
    <div class="network-film" :data-phase="phase" :aria-label="t('intro.film_aria')">
      <span class="film-label">{{ t('intro.film_label') }}</span>
      <div class="film-network" :class="{ linked: phase === 3 }"><i class="pi pi-compass" aria-hidden="true"/><span>{{ t('intro.film_space') }}</span></div>
      <div class="film-link link-left" :class="{ lit: phase >= 1 }"/><div class="film-link link-right" :class="{ lit: phase >= 2 }"/>
      <div class="film-device film-computer" :class="{ active: phase === 0 || phase === 3 }"><i class="pi pi-desktop" aria-hidden="true"/><strong>{{ t('intro.film_computer') }}</strong><small>{{ phase === 0 ? t('intro.create') : t('intro.film_ready') }}</small></div>
      <div class="film-card" :class="{ visible: phase >= 1 }"><i class="pi pi-qrcode" aria-hidden="true"/><span>{{ t('network_form.card') }}</span><small>QR / URL</small></div>
      <div class="film-device film-phone" :class="{ active: phase >= 2 }"><i :class="phase === 3 ? 'pi pi-check-circle' : 'pi pi-mobile'" aria-hidden="true"/><strong>{{ t('intro.film_other') }}</strong><small>{{ phase === 3 ? t('intro.film_same') : phase === 2 ? t('intro.film_scan') : t('intro.film_wait') }}</small></div>
      <div class="film-caption"><span>0{{ phase + 1 }}</span><strong>{{ steps[phase]!.title }}</strong></div>
    </div>
    <div class="film-controls"><span>{{ steps[phase]!.text }}</span><Button v-if="!reduced" text :icon="paused ? 'pi pi-play' : 'pi pi-pause'" :aria-label="paused ? t('intro.film_play') : t('intro.film_pause')" @click="paused = !paused"/></div>
    <ol class="film-steps" :aria-label="t('intro.film_steps')"><li v-for="(step, index) in steps" :key="step.title"><button :aria-current="phase === index ? 'step' : undefined" @click="select(index)"><span>{{ index + 1 }}</span>{{ step.title }}</button></li></ol>
  </div>
</template>
