// 将 tag 版本号同步到 package.json / tauri.conf.json / Cargo.toml(仅 CI 工作区,不提交)
import { readFileSync, writeFileSync } from 'node:fs'

const tag = process.argv[2] || ''
const version = tag.replace(/^v/, '')
if (!/^\d+\.\d+\.\d+/.test(version)) {
  console.error(`无效的版本号: ${tag}`)
  process.exit(1)
}

// package.json
const pkgPath = 'package.json'
const pkg = JSON.parse(readFileSync(pkgPath, 'utf8'))
pkg.version = version
writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + '\n')

// tauri.conf.json
const confPath = 'src-tauri/tauri.conf.json'
const conf = JSON.parse(readFileSync(confPath, 'utf8'))
conf.version = version
writeFileSync(confPath, JSON.stringify(conf, null, 2) + '\n')

// Cargo.toml(首个 version 字段)
const cargoPath = 'src-tauri/Cargo.toml'
const cargo = readFileSync(cargoPath, 'utf8')
const updated = cargo.replace(/^(version\s*=\s*")[^"]*(")/m, `$1${version}$2`)
if (updated === cargo) {
  console.error('Cargo.toml 中未找到 version 字段')
  process.exit(1)
}
writeFileSync(cargoPath, updated)

console.log(`版本号已同步为: ${version}`)
