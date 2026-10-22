<script setup lang="ts">
import { ref } from 'vue'
import { api, runBatch } from '../api/tauri'
import { store, pushLog } from '../store'
import type { MirrorLatency } from '../store'

interface Mirror {
  name: string
  domain: string
  url: string
}

const MIRRORS: Mirror[] = [
  { name: '官方源', domain: 'downloads.openwrt.org', url: 'https://downloads.openwrt.org' },
  { name: '清华大学 TUNA', domain: 'mirrors.tuna.tsinghua.edu.cn', url: 'https://mirrors.tuna.tsinghua.edu.cn/openwrt' },
  { name: '中科大 USTC', domain: 'mirrors.ustc.edu.cn', url: 'https://mirrors.ustc.edu.cn/openwrt' },
  { name: '阿里云', domain: 'mirrors.aliyun.com', url: 'https://mirrors.aliyun.com/openwrt' }
]

const results = ref<MirrorLatency[]>([])
const testing = ref(false)
const switching = ref('')

function urlOf(domain: string): string {
  return MIRRORS.find((m) => m.domain === domain)?.url || ''
}

async function doTest() {
  if (!store.connected || testing.value) return
  testing.value = true
  results.value = []
  pushLog({ stream: 'sys', text: '==> 开始路由器端实时测速...' })
  try {
    results.value = await api.testMirrors(MIRRORS.map((m) => ({ name: m.name, domain: m.domain })))
    pushLog({ stream: 'sys', text: '==> 测速完成(已按延迟从低到高排序)' })
  } catch (e) {
    pushLog({ stream: 'err', text: `==> 测速失败: ${String(e)}` })
  } finally {
    testing.value = false
  }
}

async function doSwitch(m: Mirror) {
  if (!store.connected || switching.value) return
  switching.value = m.domain
  pushLog({ stream: 'sys', text: `==> 切换软件源到 [${m.name}] ${m.url} (已自动备份)` })
  try {
    await api.switchMirror(m.url)
    pushLog({ stream: 'sys', text: `==> 换源成功: ${m.name}, apk update 校验通过` })
  } catch (e) {
    pushLog({ stream: 'err', text: `==> 换源失败并已自动回滚: ${String(e)}` })
  } finally {
    switching.value = ''
  }
}
/** 更新软件包: apk update && apk upgrade(命令自带 update, 跳过全局更新) */
async function doUpgradePackages() {
  if (!store.connected || store.installing) return
  store.installing = true
  pushLog({ stream: 'sys', text: '==> 更新软件包: apk update && apk upgrade' })
  await runBatch(['apk update && apk upgrade'], true, true)
  store.installing = false
}
</script>

<template>
  <div class="bg-white rounded-lg shadow-sm border border-slate-200 p-4">
    <div class="flex items-center justify-between mb-3">
      <div class="font-semibold text-slate-700">镜像源测速与切换</div>
      <div class="flex gap-2">
        <button @click="doUpgradePackages" :disabled="!store.connected || store.installing"
          class="bg-sky-600 hover:bg-sky-700 text-white text-xs px-3 py-1.5 rounded disabled:opacity-40">
          更新软件包
        </button>
        <button @click="doTest" :disabled="!store.connected || testing"
          class="bg-slate-700 hover:bg-slate-800 text-white text-xs px-3 py-1.5 rounded disabled:opacity-40">
          {{ testing ? '测速中...' : '测速(路由器执行)' }}
        </button>
      </div>
    </div>
    <div class="space-y-2">
      <div v-for="m in (results.length ? results.map(r => ({ ...MIRRORS.find(x => x.domain === r.domain)!, latency: r.latency_ms })) : MIRRORS.map(m => ({ ...m, latency: null as number | null })))"
        :key="m.domain"
        class="flex items-center justify-between text-sm border border-slate-100 rounded px-3 py-2">
        <div>
          <div class="font-medium">{{ m.name }}</div>
          <div class="text-xs text-slate-400">{{ m.domain }}</div>
        </div>
        <div class="flex items-center gap-3">
          <span v-if="m.latency !== null" class="text-teal-600 font-mono text-xs">{{ m.latency }} ms</span>
          <span v-else-if="results.length" class="text-red-500 font-mono text-xs">超时</span>
          <span v-else class="text-slate-300 font-mono text-xs">--</span>
          <button @click="doSwitch(m)" :disabled="!store.connected || !!switching"
            class="bg-slate-600 hover:bg-slate-700 text-white text-xs px-3 py-1 rounded disabled:opacity-40">
            {{ switching === m.domain ? '切换中...' : '使用此源' }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
