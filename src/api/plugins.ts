// OpenApk 插件菜单三级自动降级加载模块
// 第一优先级: GitHub 加速代理 -> raw.githubusercontent.com
// 第二优先级: 码云 Gitee 备用源
// 第三优先级: 前端打包内置的本地离线 plugins.json
import type { PluginItem } from '../store'

// GitHub / Gitee 用户名
const GITHUB_USER = 'chengege666'
const GITEE_USER = 'chengege666'

export const DEFAULT_PROXY = 'https://cgjs.1231818.xyz/'
const TIMEOUT_MS = 2500

const RAW_URL = `https://raw.githubusercontent.com/${GITHUB_USER}/OpenApk/main/plugins.json`
const GITEE_URL = `https://gitee.com/${GITEE_USER}/OpenApk/raw/master/plugins.json`

export function getProxyPrefix(): string {
  return localStorage.getItem('openapk_proxy') || DEFAULT_PROXY
}

export function setProxyPrefix(v: string): void {
  localStorage.setItem('openapk_proxy', v)
}

export function proxyEnabled(): boolean {
  return localStorage.getItem('openapk_proxy_on') !== '0'
}

export function setProxyEnabled(on: boolean): void {
  localStorage.setItem('openapk_proxy_on', on ? '1' : '0')
}

/** 对 GitHub 下载地址拼接加速代理前缀 */
export function withProxy(url: string): string {
  if (proxyEnabled() && url.includes('https://github.com/')) {
    return `${getProxyPrefix()}${url}`
  }
  return url
}

async function fetchJson(url: string, timeout = TIMEOUT_MS): Promise<PluginItem[]> {
  const ctrl = new AbortController()
  const timer = setTimeout(() => ctrl.abort(), timeout)
  try {
    const resp = await fetch(url, { signal: ctrl.signal, cache: 'no-store' })
    if (!resp.ok) throw new Error(`HTTP ${resp.status}`)
    const data = await resp.json()
    if (!data || !Array.isArray(data.plugins)) throw new Error('插件清单格式非法')
    return data.plugins as PluginItem[]
  } finally {
    clearTimeout(timer)
  }
}

export interface PluginLoadResult {
  list: PluginItem[]
  source: string
}

/** 三级平滑降级拉取插件菜单,任何情况下都不白屏 */
export async function loadPlugins(): Promise<PluginLoadResult> {
  // 第一优先级: GitHub (经加速代理或直连)
  const ghUrl = proxyEnabled() ? `${getProxyPrefix()}${RAW_URL}` : RAW_URL
  try {
    const list = await fetchJson(ghUrl)
    return { list, source: proxyEnabled() ? 'GitHub 加速代理' : 'GitHub 直连' }
  } catch {
    /* 静默降级 */
  }
  // 第二优先级: 码云 Gitee 备用源
  try {
    const list = await fetchJson(GITEE_URL)
    return { list, source: '码云 Gitee 备用源' }
  } catch {
    /* 静默降级 */
  }
  // 第三优先级: 本地内置离线兜底
  const local = (await import('../assets/plugins.json')).default
  return { list: (local as { plugins: PluginItem[] }).plugins, source: '本地内置离线菜单' }
}

/** 场景A: 源内插件 / 场景B: 第三方 GitHub 直链 APK(支持 cmd 自定义模板) */
export function buildInstallCommand(p: PluginItem): string {
  if (p.type === 'external' && p.url) {
    const url = withProxy(p.url)
    if (p.cmd) {
      return p.cmd.split('{URL}').join(url)
    }
    const file = `/tmp/openapk_${(p.pkg || p.name).replace(/[^a-zA-Z0-9_-]/g, '_')}.apk`
    return `wget -q -O ${file} '${url}' && apk add --allow-untrusted ${file} && rm -f ${file}`
  }
  return `apk add --allow-untrusted ${p.pkg || p.name}`
}

/** 更新插件: 源内走 apk upgrade; 第三方 APK 重新下载安装即完成升级 */
export function buildUpgradeCommand(p: PluginItem): string {
  if (p.type === 'external' && p.url) {
    return buildInstallCommand(p)
  }
  return `apk upgrade --allow-untrusted ${p.pkg || p.name}`
}

/** 架构兼容过滤 */
export function isArchCompatible(p: PluginItem, arch: string): boolean {
  if (!p.arch || p.arch.length === 0 || !arch) return true
  return p.arch.includes(arch)
}
