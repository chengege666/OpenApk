# OpenApk

跨平台桌面 GUI 工具，专为搭载**原生 apk 体系**的现代 OpenWrt 路由器设计，提供软件源管理与插件批量一键安装能力。

> ⚠️ 仅支持 **OpenWrt 25.12.0 及以上版本**（或 SNAPSHOT）。连接时会进行严格版本门禁校验，低于该版本的固件将被安全拦截。

## 功能特性

- **SSH 连接与环境探测**：IP 历史记录、版本/架构门禁校验、`/overlay` 剩余空间检测（低于 20MB 显眼警告）
- **镜像源管理与路由器端测速**：内置官方源 / 清华 TUNA / 中科大 USTC / 阿里云四个节点，由路由器实际执行测速并排序；换源自动备份 + `apk update` 校验，失败自动回滚
- **插件批量安装**：分类浏览、关键字搜索、多选/全选、一键串行安装；批处理前统一执行一次全局 `apk update`；单条命令 120 秒超时，支持随时中止
- **双插件场景**：源内插件直接 `apk add`；第三方 GitHub 直链 APK 自动下载到 `/tmp` → 本地安装 → 立即清理
- **GitHub 下载加速**：可开关、可自定义代理前缀（默认 `https://cgjs.1231818.xyz/`），持久化保存
- **插件菜单三级降级**：GitHub 加速代理 → 码云 Gitee 备用源 → 本地内置离线菜单，任何网络环境下界面不白屏
- **架构兼容过滤**：插件声明的 `arch` 与路由器 `DISTRIB_ARCH` 不匹配时自动置灰并标注原因
- **实时流式终端**：黑色终端面板实时回显 SSH 命令 stdout/stderr，支持 ANSI 颜色高亮与自动滚动

## 技术栈

- 后端：Tauri v2 (Rust) + ssh2（多线程 SSH 会话、非阻塞流式读取）
- 前端：Vue 3 + Vite + Tailwind CSS
- 发布：GitHub Actions 矩阵构建，自动产出 Windows (`.exe`/`.msi`) 与 macOS (`.dmg`) 安装包并挂载到 Release

## 下载安装

前往 [Releases](https://github.com/chengege666/OpenApk/releases) 页面下载对应平台的安装包：

| 平台 | 文件 |
|---|---|
| Windows | `OpenApk_x.x.x_x64-setup.exe` / `.msi` |

## 本地开发

```bash
npm install
npm run dev        # 仅前端界面调试（无需 Rust 环境）
npm run tauri dev  # 完整桌面调试（需安装 Rust 工具链）
```

## 发布流程

```bash
git tag v1.0.0
git push origin v1.0.0
```

推送 tag 后 GitHub Actions 自动完成多平台编译并创建 Release。



## 许可证

[MIT](LICENSE) © 2026 chengege666
