<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue'
import TopBar from './components/TopBar.vue'
import MirrorPanel from './components/MirrorPanel.vue'
import ProxySettings from './components/ProxySettings.vue'
import PluginList from './components/PluginList.vue'
import TerminalPanel from './components/TerminalPanel.vue'
import { onLog } from './api/tauri'
import { pushLog } from './store'

let unlisten: (() => void) | null = null

onMounted(async () => {
  unlisten = await onLog(pushLog)
})

onUnmounted(() => {
  if (unlisten) unlisten()
})
</script>

<template>
  <div class="h-full flex flex-col">
    <TopBar />
    <div class="flex-1 grid grid-cols-12 gap-4 p-4 min-h-0">
      <div class="col-span-4 space-y-4 overflow-y-auto term-scroll">
        <MirrorPanel />
        <ProxySettings />
      </div>
      <div class="col-span-5 min-h-0">
        <PluginList />
      </div>
      <div class="col-span-3 min-h-0">
        <TerminalPanel />
      </div>
    </div>
  </div>
</template>
