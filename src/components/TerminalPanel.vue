<script setup lang="ts">
import { nextTick, ref, watch } from 'vue'
import { store } from '../store'

const panel = ref<HTMLElement | null>(null)

const COLORS: Record<number, string> = {
  30: '#6b7280', 31: '#f87171', 32: '#4ade80', 33: '#facc15',
  34: '#60a5fa', 35: '#c084fc', 36: '#22d3ee', 37: '#e5e7eb',
  90: '#9ca3af', 91: '#fca5a5', 92: '#86efac', 93: '#fde047',
  94: '#93c5fd', 95: '#d8b4fe', 96: '#67e8f9', 97: '#ffffff'
}

function escapeHtml(s: string): string {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
}

/** 轻量 ANSI 颜色渲染 (xterm 风格) */
function ansiToHtml(s: string): string {
  let html = escapeHtml(s)
  let open = false
  html = html.replace(/\x1b\[([0-9;]*)m/g, (_m, codes: string) => {
    const nums = codes === '' ? [0] : codes.split(';').map(Number)
    let out = ''
    for (const n of nums) {
      if (n === 0) {
        if (open) { out += '</span>'; open = false }
      } else if (COLORS[n]) {
        if (open) out += '</span>'
        out += `<span style="color:${COLORS[n]}">`
        open = true
      }
    }
    return out
  })
  // 剥离其余控制序列
  html = html.replace(/\x1b\[[0-9;]*[A-Za-z]/g, '')
  return html + (open ? '</span>' : '')
}

function lineClass(stream: string): string {
  if (stream === 'stderr' || stream === 'err') return 'text-red-400'
  if (stream === 'sys') return 'text-cyan-300'
  return 'text-slate-200'
}

function render(line: { stream: string; text: string }): string {
  return ansiToHtml(line.text)
}

watch(
  () => store.logs.length,
  async () => {
    await nextTick()
    if (panel.value) panel.value.scrollTop = panel.value.scrollHeight
  }
)

function clearLogs() {
  store.logs = []
}
</script>

<template>
  <div class="bg-black rounded-lg shadow-sm border border-slate-700 flex flex-col h-full">
    <div class="flex items-center justify-between px-3 py-2 border-b border-slate-700">
      <div class="text-slate-300 text-xs font-mono">openapk@router:~# 实时日志</div>
      <button @click="clearLogs" class="text-slate-400 hover:text-white text-xs">清屏</button>
    </div>
    <div ref="panel" class="term-scroll flex-1 overflow-y-auto px-3 py-2 font-mono text-xs leading-5">
      <div v-for="(l, i) in store.logs" :key="i" :class="lineClass(l.stream)">
        <span v-html="render(l)"></span>
      </div>
      <div v-if="!store.logs.length" class="text-slate-600">等待操作...</div>
    </div>
  </div>
</template>
