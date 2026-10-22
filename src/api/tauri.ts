import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { DeviceInfo, LogLine, MirrorLatency } from '../store'

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
  install: (cmds: string[]) => invoke<void>('install_plugins', { cmds }),
  abort: () => invoke<void>('abort_install')
}

/** 订阅后端实时流式日志 */
export function onLog(cb: (line: LogLine) => void): Promise<() => void> {
  return listen<LogLine>('openapk://log', (e) => cb(e.payload))
}
