<script setup lang="ts">
import { ref } from 'vue'
import { runBatch } from '../api/tauri'
import { allowUntrusted } from '../api/plugins'
import { store, pushLog } from '../store'

const flag = () => (allowUntrusted() ? '--allow-untrusted ' : '')

interface InitItem {
  key: string
  label: string
  checked: boolean
  build: () => string
}

const items = ref<InitItem[]>([
  {
    key: 'tz',
    label: '时区设置 (Asia/Shanghai)',
    checked: true,
    build: () =>
      "uci set system.@system[0].timezone='CST-8' && uci set system.@system[0].zonename='Asia/Shanghai' && uci commit system"
  },
  {
    key: 'zh',
    label: '中文语言环境 (luci-i18n-base-zh-cn)',
    checked: true,
    build: () => `apk add ${flag()}luci-i18n-base-zh-cn`
  },
  {
    key: 'sftp',
    label: 'SFTP 服务 (openssh-sftp-server)',
    checked: true,
    build: () => `apk add ${flag()}openssh-sftp-server`
  },
  {
    key: 'tools',
    label: '常用系统工具 (htop / curl / nano)',
    checked: true,
    build: () => `apk add ${flag()}htop curl nano`
  }
])

/** 一键初始化: 先全局 apk update, 再逐项执行, 单项失败不中断后续 */
async function doInit() {
  const cmds = items.value.filter((i) => i.checked).map((i) => i.build())
  if (!store.connected || store.installing || cmds.length === 0) return
  store.installing = true
  pushLog({ stream: 'sys', text: `==> 开始系统基础初始化, 共 ${cmds.length} 个子项(失败不中断)` })
  await runBatch(cmds, false, false)
  store.installing = false
  pushLog({ stream: 'sys', text: '==> 系统基础初始化流程结束, 各子项结果见上方日志' })
}
</script>

<template>
  <div class="bg-white rounded-lg shadow-sm border border-slate-200 p-4">
    <div class="flex items-center justify-between mb-2">
      <div class="font-semibold text-slate-700">系统基础初始化</div>
      <button @click="doInit" :disabled="!store.connected || store.installing"
        class="bg-teal-600 hover:bg-teal-700 text-white text-xs px-3 py-1.5 rounded disabled:opacity-40">
        一键初始化
      </button>
    </div>
    <label v-for="i in items" :key="i.key"
      class="flex items-center gap-2 py-1 text-xs text-slate-600 cursor-pointer hover:bg-slate-50 rounded px-1">
      <input type="checkbox" v-model="i.checked" :disabled="store.installing" class="accent-teal-600" />
      {{ i.label }}
    </label>
  </div>
</template>
