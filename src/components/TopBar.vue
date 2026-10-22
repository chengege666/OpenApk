<script setup lang="ts">
import { ref } from 'vue'
import { api } from '../api/tauri'
import { store, pushLog } from '../store'

const HISTORY_KEY = 'openapk_ip_history'

const host = ref(localStorage.getItem('openapk_last_host') || '')
const port = ref(22)
const user = ref('root')
const password = ref('')
const histories = ref<string[]>(JSON.parse(localStorage.getItem(HISTORY_KEY) || '[]'))

function saveHistory(ip: string) {
  const list = [ip, ...histories.value.filter((h) => h !== ip)].slice(0, 10)
  histories.value = list
  localStorage.setItem(HISTORY_KEY, JSON.stringify(list))
  localStorage.setItem('openapk_last_host', ip)
}

async function doConnect() {
  if (!host.value || store.connecting) return
  store.connecting = true
  store.gateError = ''
  pushLog({ stream: 'sys', text: `==> 正在连接 ${host.value}:${port.value} ...` })
  try {
    const info = await api.connect(host.value.trim(), Number(port.value) || 22, user.value.trim(), password.value)
    store.device = info
    store.connected = true
    saveHistory(host.value.trim())
    pushLog({ stream: 'sys', text: `==> 连接成功: OpenWrt ${info.release} (${info.arch})${info.is_snapshot ? ' [SNAPSHOT]' : ''}` })
  } catch (e) {
    store.gateError = String(e)
    pushLog({ stream: 'err', text: `==> 连接失败: ${String(e)}` })
  } finally {
    store.connecting = false
  }
}

async function doDisconnect() {
  await api.abort().catch(() => {})
  await api.disconnect().catch(() => {})
  store.connected = false
  store.device = null
  pushLog({ stream: 'sys', text: '==> 已断开连接' })
}
</script>

<template>
  <div class="bg-white shadow-sm border-b border-slate-200 px-4 py-3">
    <div class="flex items-center gap-2 flex-wrap">
      <div class="text-lg font-bold text-teal-700 mr-2">OpenApk</div>
      <input v-model="host" list="ip-history" placeholder="路由器 IP" :disabled="store.connected"
        class="border rounded px-2 py-1.5 w-40 text-sm disabled:bg-slate-100" />
      <datalist id="ip-history">
        <option v-for="h in histories" :key="h" :value="h" />
      </datalist>
      <input v-model.number="port" type="number" placeholder="端口" :disabled="store.connected"
        class="border rounded px-2 py-1.5 w-20 text-sm disabled:bg-slate-100" />
      <input v-model="user" placeholder="用户名" :disabled="store.connected"
        class="border rounded px-2 py-1.5 w-24 text-sm disabled:bg-slate-100" />
      <input v-model="password" type="password" placeholder="密码" :disabled="store.connected"
        class="border rounded px-2 py-1.5 w-36 text-sm disabled:bg-slate-100"
        @keyup.enter="doConnect" />
      <button v-if="!store.connected" @click="doConnect" :disabled="store.connecting"
        class="bg-teal-600 hover:bg-teal-700 text-white text-sm px-4 py-1.5 rounded disabled:opacity-50">
        {{ store.connecting ? '连接中...' : '连接' }}
      </button>
      <button v-else @click="doDisconnect"
        class="bg-slate-500 hover:bg-slate-600 text-white text-sm px-4 py-1.5 rounded">
        断开
      </button>
      <div v-if="store.device" class="flex items-center gap-2 text-xs">
        <span class="bg-teal-50 text-teal-700 border border-teal-200 rounded px-2 py-1">
          OpenWrt {{ store.device.release }}{{ store.device.is_snapshot ? ' (snapshot)' : '' }}
        </span>
        <span class="bg-slate-100 text-slate-600 border border-slate-200 rounded px-2 py-1">
          {{ store.device.arch }}
        </span>
      </div>
    </div>
    <!-- 剩余闪存空间显眼警告 -->
    <div v-if="store.device && store.device.overlay_free_mb >= 0 && store.device.overlay_free_mb < 20"
      class="mt-2 bg-red-50 border border-red-300 text-red-700 text-sm rounded px-3 py-2 font-medium">
      ⚠ 闪存剩余空间仅 {{ store.device.overlay_free_mb }} MB，低于 20MB 安全线，继续安装可能导致写满 /overlay！
    </div>
    <div v-else-if="store.device && store.device.overlay_free_mb >= 0"
      class="mt-2 text-xs text-slate-500">
      /overlay 剩余空间: {{ store.device.overlay_free_mb }} MB
    </div>
  </div>

  <!-- 版本拦截弹窗 (Gatekeeper) -->
  <div v-if="store.gateError" class="fixed inset-0 bg-black/40 flex items-center justify-center z-50">
    <div class="bg-white rounded-lg shadow-xl w-[420px] p-6">
      <div class="text-red-600 font-bold text-lg mb-3">连接被安全拦截</div>
      <div class="text-sm text-slate-700 whitespace-pre-wrap break-all">{{ store.gateError }}</div>
      <div class="mt-5 text-right">
        <button @click="store.gateError = ''"
          class="bg-slate-600 hover:bg-slate-700 text-white text-sm px-4 py-1.5 rounded">关闭</button>
      </div>
    </div>
  </div>
</template>
