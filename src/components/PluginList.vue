<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { loadPlugins, buildInstallCommand, buildUpgradeCommand, buildUninstallCommand, isArchCompatible } from '../api/plugins'
import { api, runBatch } from '../api/tauri'
import { store, pushLog } from '../store'
import type { PluginItem } from '../store'

const loading = ref(true)
const keyword = ref('')
const activeCategory = ref('全部')
const selected = ref<Set<string>>(new Set())

onMounted(async () => {
  loading.value = true
  try {
    const r = await loadPlugins()
    store.plugins = r.list
    store.pluginsSource = r.source
    pushLog({ stream: 'sys', text: `==> 插件菜单已加载 [${r.source}], 共 ${r.list.length} 项` })
  } finally {
    loading.value = false
  }
  await refreshInstalled()
})

watch(
  () => store.connected,
  (v) => {
    if (v) refreshInstalled()
    else store.installed = []
  }
)

/** 刷新路由器已安装包列表(apk list --installed) */
async function refreshInstalled() {
  if (!store.connected) {
    store.installed = []
    return
  }
  try {
    const raw = await api.listInstalled()
    store.installed = raw
      .split('\n')
      .map((l) => (l.trim().split(/\s+/)[0] || '').replace(/-\d.*$/, ''))
      .filter(Boolean)
  } catch {
    /* 查询失败不影响界面 */
  }
}

function isInstalled(p: PluginItem): boolean {
  const first = (p.pkg || p.name).split(/\s+/)[0]
  return store.installed.includes(first) || store.installed.includes(p.name)
}

const categories = computed(() => {
  const set = new Set(store.plugins.map((p) => p.category || '其他'))
  return ['全部', ...set]
})

const filtered = computed(() =>
  store.plugins.filter((p) => {
    const matchCat = activeCategory.value === '全部' || (p.category || '其他') === activeCategory.value
    const kw = keyword.value.trim().toLowerCase()
    const matchKw = !kw || p.name.toLowerCase().includes(kw) || (p.desc || '').toLowerCase().includes(kw)
    return matchCat && matchKw
  })
)

function compatible(p: PluginItem): boolean {
  return isArchCompatible(p, store.device?.arch || '')
}

const selectableFiltered = computed(() => filtered.value.filter(compatible))

const allSelected = computed(
  () => selectableFiltered.value.length > 0 && selectableFiltered.value.every((p) => selected.value.has(p.name))
)

function toggle(p: PluginItem) {
  if (!compatible(p)) return
  if (selected.value.has(p.name)) selected.value.delete(p.name)
  else selected.value.add(p.name)
  selected.value = new Set(selected.value)
}

function toggleAll() {
  if (allSelected.value) selected.value = new Set()
  else selected.value = new Set(selectableFiltered.value.map((p) => p.name))
}

async function doInstall() {
  if (!store.connected || store.installing || selected.value.size === 0) return
  const items = store.plugins.filter((p) => selected.value.has(p.name))
  const cmds = items.map(buildInstallCommand)
  store.installing = true
  pushLog({ stream: 'sys', text: `==> 开始批量安装 ${items.length} 个插件(串行,单条限时 120s)` })
  await runBatch(cmds, false, true)
  store.installing = false
  selected.value = new Set()
  await refreshInstalled()
}

async function doAbort() {
  await api.abort().catch(() => {})
  pushLog({ stream: 'err', text: '==> 已发送中止信号...' })
}

async function doUpgrade(items: PluginItem[], label: string) {
  if (!store.connected || store.installing || items.length === 0) return
  const cmds = items.map(buildUpgradeCommand)
  store.installing = true
  pushLog({ stream: 'sys', text: `==> 开始${label} ${items.length} 个插件(串行,单条限时 120s)` })
  await runBatch(cmds, false, false)
  store.installing = false
  selected.value = new Set()
  await refreshInstalled()
}

function doUpgradeSelected() {
  doUpgrade(store.plugins.filter((p) => selected.value.has(p.name)), '更新选中')
}

function doUpgradeAll() {
  doUpgrade(store.plugins.filter((p) => compatible(p)), '更新所有插件')
}

/** 卸载插件: apk del + 清理配置目录, 完成后刷新安装状态 */
async function doUninstall(p: PluginItem) {
  if (!store.connected || store.installing) return
  if (!window.confirm(`确认卸载 ${p.name}?\n将同时清理 /etc/config 与 /etc 下相关配置文件,该操作不可恢复。`)) return
  store.installing = true
  pushLog({ stream: 'sys', text: `==> 卸载插件: ${p.name} (含配置清理)` })
  await runBatch([buildUninstallCommand(p)], true, false)
  store.installing = false
  pushLog({ stream: 'sys', text: `==> ${p.name} 卸载流程结束` })
  await refreshInstalled()
}
</script>

<template>
  <div class="bg-white rounded-lg shadow-sm border border-slate-200 p-4 flex flex-col h-full">
    <div class="flex items-center justify-between mb-3 flex-wrap gap-2">
      <div class="font-semibold text-slate-700">
        插件列表
        <span v-if="store.pluginsSource" class="ml-2 text-xs font-normal bg-slate-100 text-slate-500 rounded px-2 py-0.5">
          {{ store.pluginsSource }}
        </span>
      </div>
      <div class="flex items-center gap-2">
        <input v-model="keyword" placeholder="搜索插件..."
          class="border rounded px-2 py-1 text-sm w-40" />
        <button @click="doAbort" v-if="store.installing"
          class="bg-red-600 hover:bg-red-700 text-white text-sm px-3 py-1.5 rounded">
          中止任务
        </button>
        <button @click="doUpgradeAll" :disabled="!store.connected || store.installing"
          class="bg-indigo-600 hover:bg-indigo-700 text-white text-sm px-3 py-1.5 rounded disabled:opacity-40">
          更新所有插件
        </button>
        <button @click="doUpgradeSelected"
          :disabled="!store.connected || store.installing || selected.size === 0"
          class="bg-sky-600 hover:bg-sky-700 text-white text-sm px-3 py-1.5 rounded disabled:opacity-40">
          更新选中 ({{ selected.size }})
        </button>
        <button @click="doInstall"
          :disabled="!store.connected || store.installing || selected.size === 0"
          class="bg-teal-600 hover:bg-teal-700 text-white text-sm px-4 py-1.5 rounded disabled:opacity-40">
          {{ store.installing ? '安装中...' : `一键安装 (${selected.size})` }}
        </button>
      </div>
    </div>

    <div class="flex items-center gap-1 mb-3 flex-wrap">
      <button v-for="c in categories" :key="c" @click="activeCategory = c"
        :class="['text-xs px-3 py-1 rounded-full border', activeCategory === c ? 'bg-teal-600 text-white border-teal-600' : 'bg-white text-slate-600 border-slate-200 hover:border-teal-400']">
        {{ c }}
      </button>
    </div>

    <label class="flex items-center gap-2 text-xs text-slate-500 mb-2 cursor-pointer">
      <input type="checkbox" :checked="allSelected" @change="toggleAll" class="accent-teal-600" />
      全选当前筛选结果 ({{ selectableFiltered.length }} 项可选)
    </label>

    <div class="flex-1 overflow-y-auto term-scroll border-t border-slate-100">
      <div v-if="loading" class="text-sm text-slate-400 py-6 text-center">插件菜单加载中...</div>
      <div v-else-if="!filtered.length" class="text-sm text-slate-400 py-6 text-center">无匹配插件</div>
      <label v-for="p in filtered" :key="p.name"
        :class="['flex items-start gap-3 px-2 py-2.5 border-b border-slate-50', compatible(p) ? 'cursor-pointer hover:bg-slate-50' : 'opacity-50 cursor-not-allowed']">
        <input type="checkbox" :checked="selected.has(p.name)" :disabled="!compatible(p)"
          @change="toggle(p)" class="mt-1 accent-teal-600" />
        <div class="flex-1">
          <div class="text-sm font-medium">
            {{ p.name }}
            <span v-if="isInstalled(p)" class="ml-1 text-[10px] bg-emerald-100 text-emerald-700 rounded px-1.5 py-0.5">已安装</span>
            <span v-if="p.type === 'external'" class="ml-1 text-[10px] bg-amber-100 text-amber-700 rounded px-1.5 py-0.5">第三方APK</span>
            <span v-else class="ml-1 text-[10px] bg-teal-50 text-teal-600 rounded px-1.5 py-0.5">源内</span>
          </div>
          <div class="text-xs text-slate-500">{{ p.desc }}</div>
          <div v-if="!compatible(p)" class="text-xs text-red-500 mt-0.5">
            已禁用: 不兼容当前架构 {{ store.device?.arch || '未知' }} (要求 {{ (p.arch || []).join(' / ') }})
          </div>
        </div>
        <button v-if="isInstalled(p)" @click.stop.prevent="doUninstall(p)"
          :disabled="!store.connected || store.installing"
          class="self-center bg-red-50 hover:bg-red-100 text-red-600 border border-red-200 text-xs px-2.5 py-1 rounded disabled:opacity-40">
          卸载
        </button>
      </label>
    </div>
  </div>
</template>
