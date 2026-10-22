import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { DeviceInfo, LogLine, MirrorLatency } from '../store'
import { pushLog } from '../store'
import { allowUntrusted } from './plugins'

export interface MirrorReq {
  name: string
  domain: string
}

export const api = {
  connect: (host: string, port: number, user: string, password: string) =>
    invoke<DeviceInfo>('connect_device', { host, port, user, password }),
  disconnect: () => invoke<void>('disconnect_device'),
  testMirrors: (mirrors: MirrorReq[]) => invoke<MirrorLatency[]>('test_mirrors', { mirrors }),
  switchMirror: (mirrorUrl: string) => invoke<string>('switch_mirror', { mirrorUrl }),
  install: (cmds: string[], allowUntrusted: boolean, skipUpdate: boolean, stopOnError: boolean) =>
    invoke<void>('install_plugins', { cmds, allowUntrusted, skipUpdate, stopOnError }),
  listInstalled: () => invoke<string>('list_installed'),
  abort: () => invoke<void>('abort_install')
}

/** 订阅后端实时流式日志 */
export function onLog(cb: (line: LogLine) => void): Promise<() => void> {
  return listen<LogLine>('openapk://log', (e) => cb(e.payload))
}

/** 执行批量命令并等待 openapk://done 完成事件(修复安装锁定态立即失效的问题) */
export async function runBatch(cmds: string[], skipUpdate: boolean, stopOnError: boolean): Promise<void> {
  return new Promise((resolve) => {
    let un: (() => void) | null = null
    listen('openapk://done', () => {
      if (un) un()
      resolve()
    }).then((u) => (un = u))
    api.install(cmds, allowUntrusted(), skipUpdate, stopOnError).catch((e) => {
      pushLog({ stream: 'err', text: `==> 批量任务异常: ${String(e)}` })
      resolve()
    })
  })
}
