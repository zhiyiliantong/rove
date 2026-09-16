<script setup lang="ts">
import { computed, ref } from 'vue';
import Button from 'primevue/button';
import Menu from 'primevue/menu';
import { archiveSession } from '../ui';
const props = defineProps<{ id: string; title: string }>();
const menu = ref<InstanceType<typeof Menu>>();
const expanded = ref(false);
const items = computed(() => [{ label: '归档会话', icon: 'pi pi-inbox', command: () => archiveSession(props.id) }]);
</script>
<template>
  <Button class="session-more" icon="pi pi-ellipsis-h" text :aria-label="`更多操作：${title}`" aria-haspopup="menu" :aria-expanded="expanded" :aria-controls="`session-menu-${id}`" @click="menu?.toggle($event)"/>
  <Menu ref="menu" :id="`session-menu-${id}`" :model="items" popup @show="expanded = true" @hide="expanded = false"/>
</template>
