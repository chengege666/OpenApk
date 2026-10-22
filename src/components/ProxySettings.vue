<script setup lang="ts">
import { ref } from 'vue'
import { DEFAULT_PROXY, getProxyPrefix, proxyEnabled, setProxyEnabled, setProxyPrefix } from '../api/plugins'

const enabled = ref(proxyEnabled())
const prefix = ref(getProxyPrefix())

function onToggle() {
  setProxyEnabled(enabled.value)
}

function onInput() {
  setProxyPrefix(prefix.value.trim())
}

function onReset() {
  prefix.value = DEFAULT_PROXY
  setProxyPrefix(DEFAULT_PROXY)
}
</script>

<template>
  <div class="bg-white rounded-lg shadow-sm border border-slate-200 p-4">
    <div class="flex items-center justify-between mb-3">
      <div class="font-semibold text-slate-700">GitHub 下载加速</div>
      <label class="inline-flex items-center cursor-pointer">
        <input type="checkbox" v-model="enabled" @change="onToggle" class="sr-only peer" />
        <div
          class="w-9 h-5 bg-slate-300 rounded-full peer peer-checked:bg-teal-600 relative after:content-[''] after:absolute after:top-0.5 after:left-0.5 after:bg-white after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:after:translate-x-4">
        </div>
      </label>
    </div>
    <div class="flex items-center gap-2">
      <input v-model="prefix" @input="onInput" :disabled="!enabled"
        class="flex-1 border rounded px-2 py-1.5 text-xs font-mono disabled:bg-slate-100 disabled:text-slate-400"
        placeholder="自定义代理前缀" />
      <button @click="onReset" :disabled="!enabled"
        class="bg-slate-500 hover:bg-slate-600 text-white text-xs px-3 py-1.5 rounded disabled:opacity-40">
        恢复默认
      </button>
    </div>
    <div class="mt-2 text-xs text-slate-400">
      默认: {{ DEFAULT_PROXY }} · 修改后自动持久化保存,作用于插件菜单拉取与 GitHub 直链 APK 下载
    </div>
  </div>
</template>
