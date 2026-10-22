import { reactive } from 'vue'

export interface DeviceInfo {
  release: string
  arch: string
  overlay_free_mb: number
  is_snapshot: boolean
}

export interface PluginItem {
  name: string
  desc: string
  category: string
  type: 'repo' | 'external'
  pkg?: string
  url?: string
  /** 可选: 自定义安装命令模板, 其中的 {URL} 会被替换为代理处理后的 url */
  cmd?: string
  arch?: string[]
}

export interface MirrorLatency {
  name: string
  domain: string
  latency_ms: number | null
}

export interface LogLine {
  stream: string
  text: string
}

export const store = reactive({
  connected: false,
  connecting: false,
  device: null as DeviceInfo | null,
  gateError: '',
  plugins: [] as PluginItem[],
  pluginsSource: '',
  installed: [] as string[],
  installing: false,
  logs: [] as LogLine[]
})

export function pushLog(line: LogLine): void {
  store.logs.push(line)
  if (store.logs.length > 2000) store.logs.splice(0, store.logs.length - 2000)
}
