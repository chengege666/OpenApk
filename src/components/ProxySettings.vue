<script setup lang="ts">
import { ref } from 'vue'
import { getProxyPrefix, setProxyPrefix, proxyEnabled, setProxyEnabled, allowUntrusted, setAllowUntrusted, DEFAULT_PROXY } from '../api/plugins'
import { store, pushLog } from '../store'

const enabled = ref(proxyEnabled())
const prefix = ref(getProxyPrefix())
const untrusted = ref(allowUntrusted())
const applyMsg = ref('')

function onToggle() {
  setProxyEnabled(enabled.value)
}
function onInput() {
  setProxyPrefix(prefix.value.trim() || DEFAULT_PROXY)
}
function onReset() {
  prefix.value = DEFAULT_PROXY
  setProxyPrefix(DEFAULT_PROXY)
}
function onToggleUntrusted() {
  setAllowUntrusted(untrusted.value)
  pushLog({ stream: 'sys', text: `==> 未信任 APK 安装: ${untrusted.value ? '已允许(--allow-untrusted)' : '已禁止(恢复默认校验)'}` })
}

/** 应用新链接: 为当前清单所有 GitHub 地址按当前前缀重算并输出统计 */
function applyProxyLinks() {
  const gh = store.plugins.filter((p) => p.url && p.url.includes('https://github.com/'))
  if (enabled.value) {
    setProxyPrefix(prefix.value.trim() || DEFAULT_PROXY)
    applyMsg.value = `共 ${gh.length} 个 GitHub 资源,已全部应用加速前缀`
    pushLog({ stream: 'sys', text: `==> 加速链接已批量应用: ${gh.length} 个 GitHub 资源将经 ${getProxyPrefix()} 下载` })
  } else {
    applyMsg.value = `共 ${gh.length} 个 GitHub 资源,加速已关闭,全部恢复直连`
    pushLog({ stream: 'sys', text: `==> 加速已关闭: ${gh.length} 个 GitHub 资源恢复直连下载` })
  }
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
      <button @click="applyProxyLinks"
        class="bg-teal-600 hover:bg-teal-700 text-white text-xs px-3 py-1.5 rounded">
        应用新链接
      </button>
    </div>
    <div v-if="applyMsg" class="mt-1.5 text-xs text-teal-600">{{ applyMsg }}</div>
    <div class="mt-2 flex items-center justify-between border-t border-slate-100 pt-2">
      <span class="text-xs text-slate-600">允许未信任 APK 安装(追加 --allow-untrusted)</span>
      <label class="inline-flex items-center cursor-pointer">
        <input type="checkbox" v-model="untrusted" @change="onToggleUntrusted" class="sr-only peer" />
        <div
          class="w-9 h-5 bg-slate-300 rounded-full peer peer-checked:bg-teal-600 relative after:content-[''] after:absolute after:top-0.5 after:left-0.5 after:bg-white after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:after:translate-x-4">
        </div>
      </label>
    </div>
    <div class="mt-2 text-xs text-slate-400">
      默认: {{ DEFAULT_PROXY }} · 修改后自动持久化保存,作用于插件菜单拉取与 GitHub 直链 APK 下载
    </div>
  </div>
</template>
