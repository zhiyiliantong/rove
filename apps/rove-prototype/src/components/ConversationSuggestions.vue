<script setup lang="ts">
import { manageRovePrompt } from '../conversation-prompts';
defineEmits<{ select: [prompt: string] }>();
const suggestions = [
  { title: '让音乐住进家里', description: '在自己的设备上部署开源音乐服务', icon: 'pi-headphones', prompt: '在书房电脑上安装音乐播放器' },
  { title: '看看我的服务', description: '找到设备上已经准备好的应用', icon: 'pi-th-large', prompt: '查看当前网络中的服务' },
  { title: '建立自己的私人影院', description: '部署开源影音服务，多设备随时观看', icon: 'pi-video', prompt: '我想建立自己的私人影院，请帮我在自己的设备上安装和配置开源影音软件，并通过 Rove 访问。' },
  { title: '管理多台代码代理', description: '在多台设备上部署、访问开源代码代理', icon: 'pi-code', prompt: '我想管理多台设备上的代码代理，请帮我安装和配置第三方开源代码代理，并通过 Rove 访问这些服务。' },
  { title: '管理 Rove', description: '查看空间、清理缓存、调整存储位置、排查问题', icon: 'pi-sliders-h', prompt: manageRovePrompt },
];
</script>
<template>
  <div class="suggestions" role="group" aria-label="会话建议">
    <button v-for="item in suggestions" :key="item.title" type="button" @click="$emit('select', item.prompt)">
      <i aria-hidden="true" :class="`pi ${item.icon}`"/><strong>{{ item.title }}</strong><span>{{ item.description }}</span>
    </button>
  </div>
</template>
