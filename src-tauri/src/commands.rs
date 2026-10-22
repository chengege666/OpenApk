use crate::ssh::SshClient;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};

const GATE_MSG: &str =
    "OpenApk 仅支持搭载原生 apk 体系的 OpenWrt 25.12.0 及以上版本,当前固件已被安全拦截";
const MIN_VERSION: &str = "25.12.0";
const CMD_TIMEOUT_SECS: u64 = 120;

#[derive(Clone)]
pub struct Cred {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
}

pub struct AppState {
    pub client: Arc<Mutex<Option<SshClient>>>,
    pub creds: Mutex<Option<Cred>>,
    pub abort: Arc<AtomicBool>,
}

#[derive(Serialize)]
pub struct DeviceInfo {
    pub release: String,
    pub arch: String,
    pub overlay_free_mb: i64,
    pub is_snapshot: bool,
}

#[derive(Deserialize)]
pub struct MirrorReq {
    pub name: String,
    pub domain: String,
}

#[derive(Serialize)]
pub struct MirrorLatency {
    pub name: String,
    pub domain: String,
    pub latency_ms: Option<f64>,
}

fn parse_release_field(content: &str, key: &str) -> Option<String> {
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with(key) {
            return line
                .split('=')
                .nth(1)
                .map(|v| v.trim().trim_matches('\'').trim_matches('"').to_string());
        }
    }
    None
}

/// 版本号比较: v < min 返回 true (仅取数字段逐段比较)
fn version_lt(v: &str, min: &str) -> bool {
    let parse = |s: &str| -> Vec<u64> {
        s.split('.')
            .map(|p| {
                p.chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect::<String>()
                    .parse()
                    .unwrap_or(0)
            })
            .collect()
    };
    let a = parse(v);
    let b = parse(min);
    for i in 0..a.len().max(b.len()) {
        let x = a.get(i).copied().unwrap_or(0);
        let y = b.get(i).copied().unwrap_or(0);
        if x != y {
            return x < y;
        }
    }
    false
}

/// SSH 连接 + 版本门禁校验 + 空间检测
#[tauri::command]
pub fn connect_device(
    state: State<AppState>,
    host: String,
    port: u16,
    user: String,
    password: String,
) -> Result<DeviceInfo, String> {
    (|| -> Result<DeviceInfo> {
        let client = SshClient::connect(&host, port, &user, &password)?;

        // 门禁 1: 必须存在原生 apk 命令
        let (apk_path, code) = client.exec("command -v apk")?;
        if code != 0 || apk_path.trim().is_empty() {
            bail!(GATE_MSG);
        }

        // 门禁 2: 版本断言 OpenWrt >= 25.12.0 (snapshot 放行)
        let (release_txt, code) = client.exec("cat /etc/openwrt_release")?;
        if code != 0 {
            bail!("无法读取 /etc/openwrt_release,{}", GATE_MSG);
        }
        let release =
            parse_release_field(&release_txt, "DISTRIB_RELEASE").unwrap_or_default();
        let arch = parse_release_field(&release_txt, "DISTRIB_ARCH").unwrap_or_default();
        let is_snapshot = release.to_uppercase().contains("SNAPSHOT");
        if !is_snapshot && (release.is_empty() || version_lt(&release, MIN_VERSION)) {
            bail!(GATE_MSG);
        }

        // 空间检测: /overlay 剩余 MB
        let (df, _) = client.exec("df -m /overlay | awk 'NR==2 {print $4}'")?;
        let free = df.trim().parse::<i64>().unwrap_or(-1);

        *state.creds.lock().unwrap() = Some(Cred {
            host,
            port,
            user,
            password,
        });
        *state.client.lock().unwrap() = Some(client);
        Ok(DeviceInfo {
            release,
            arch,
            overlay_free_mb: free,
            is_snapshot,
        })
    })()
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn disconnect_device(state: State<AppState>) -> Result<(), String> {
    state.abort.store(true, Ordering::Relaxed);
    *state.client.lock().unwrap() = None;
    *state.creds.lock().unwrap() = None;
    Ok(())
}

/// 路由器端多线程测速: 每个镜像独立 SSH 连接并行执行 ping(失败回退 wget spider)
#[tauri::command]
pub fn test_mirrors(
    state: State<AppState>,
    mirrors: Vec<MirrorReq>,
) -> Result<Vec<MirrorLatency>, String> {
    let cred = state
        .creds
        .lock()
        .unwrap()
        .clone()
        .context("尚未连接设备")
        .map_err(|e: anyhow::Error| e.to_string())?;

    let mut handles = Vec::new();
    for m in mirrors {
        let c = cred.clone();
        handles.push(thread::spawn(move || {
            let latency_ms = probe_from_router(&c, &m.domain).unwrap_or(None);
            MirrorLatency {
                name: m.name,
                domain: m.domain,
                latency_ms,
            }
        }));
    }
    let mut out: Vec<MirrorLatency> = handles.into_iter().filter_map(|h| h.join().ok()).collect();
    out.sort_by(|a, b| match (a.latency_ms, b.latency_ms) {
        (Some(x), Some(y)) => x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });
    Ok(out)
}

fn probe_from_router(cred: &Cred, domain: &str) -> Result<Option<f64>> {
    let client = SshClient::connect(&cred.host, cred.port, &cred.user, &cred.password)?;
    // 优先 ping 解析平均毫秒(sed 提取 "a/b/c" 中间值,兼容 BusyBox 与 iputils 两种输出格式);
    // 解析失败则回退 wget spider,用 date %s%N 计算毫秒差
    let script = format!(
        "avg=$(ping -c 2 -W 1 {d} 2>/dev/null | sed -n 's#.*= *[0-9.]*/\\([0-9.]*\\)/.*#\\1#p'); \
         if [ -n \"$avg\" ]; then echo \"PING:$avg\"; else \
         s=$(date +%s%N); wget -q --spider -T 2 https://{d} 2>/dev/null; rc=$?; e=$(date +%s%N); \
         if [ $rc -eq 0 ]; then echo \"WGET:$(( (e-s)/1000000 ))\"; else echo \"FAIL\"; fi; fi",
        d = domain
    );
    let (out, _) = client.exec(&script)?;
    for line in out.lines() {
        if let Some(v) = line.trim().strip_prefix("PING:") {
            return Ok(v.trim().parse::<f64>().ok());
        }
        if let Some(v) = line.trim().strip_prefix("WGET:") {
            return Ok(v.trim().parse::<f64>().ok());
        }
    }
    Ok(None)
}

/// 原子性换源: 备份 -> sed 替换域名 -> apk update 校验 -> 失败自动回滚
#[tauri::command]
pub fn switch_mirror(state: State<AppState>, mirror_url: String) -> Result<(), String> {
    (|| -> Result<()> {
        if !mirror_url.starts_with("https://") {
            bail!("非法镜像地址");
        }
        let guard = state.client.lock().unwrap();
        let client = guard.as_ref().context("尚未连接设备")?;
        let url = mirror_url.trim_end_matches('/');
        // 覆盖 /etc/apk/repositories 与 /etc/apk/repositories.d/*.list 全部源配置;
        // 仅当所有文件都不存在时, 才依据设备 release 信息按新镜像重建 repositories
        let cmd = format!(
            "found=0; \
             for f in /etc/apk/repositories /etc/apk/repositories.d/*.list; do \
               [ -f \"$f\" ] || continue; \
               found=1; \
               cp \"$f\" \"$f.bak\" || exit 1; \
               sed -i -e 's#https\\?://downloads\\.openwrt\\.org#{u}#g' \
                      -e 's#\\(https\\?://[^/[:space:]]*\\)/openwrt#{u}#g' \"$f\"; \
             done; \
             if [ \"$found\" = \"0\" ]; then \
               . /etc/openwrt_release; \
               rel=\"releases/$DISTRIB_RELEASE\"; \
               [ \"$DISTRIB_RELEASE\" = \"SNAPSHOT\" ] && rel=\"snapshots\"; \
               {{ echo \"{u}/$rel/targets/$DISTRIB_TARGET/packages/packages.adb\"; \
                 for p in base luci packages routing telephony video; do \
                   echo \"{u}/$rel/packages/$DISTRIB_ARCH/$p/packages.adb\"; \
                 done; }} > /etc/apk/repositories; \
             fi; \
             apk update --allow-untrusted",
            u = url
        );
        let (out, code) = client.exec(&cmd)?;
        if code != 0 {
            let _ = client.exec(
                "for f in /etc/apk/repositories /etc/apk/repositories.d/*.list; do \
                   [ -f \"$f.bak\" ] && mv -f \"$f.bak\" \"$f\"; \
                 done",
            );
            bail!("换源后 apk update 校验失败,已自动回滚至原软件源。{}", out);
        }
        Ok(())
    })()
    .map_err(|e| e.to_string())
}

fn run_one(
    client_arc: &Arc<Mutex<Option<SshClient>>>,
    cmd: &str,
    abort: &AtomicBool,
    tx: &mpsc::Sender<(String, String)>,
) -> Result<i32> {
    let guard = client_arc.lock().unwrap();
    let client = guard.as_ref().context("SSH 会话已断开")?;
    client.exec_stream(cmd, Duration::from_secs(CMD_TIMEOUT_SECS), abort, tx)
}

/// 串行批量执行: 可选先全局 apk update 一次,再逐条执行,实时流式推送日志
/// allow_untrusted: 全局 update 是否带 --allow-untrusted
/// skip_update: 跳过全局 update(命令自带更新场景)
/// stop_on_error: 单条失败是否中断后续
#[tauri::command]
pub fn install_plugins(
    app: AppHandle,
    state: State<AppState>,
    cmds: Vec<String>,
    allow_untrusted: bool,
    skip_update: bool,
    stop_on_error: bool,
) -> Result<(), String> {
    let abort = state.abort.clone();
    abort.store(false, Ordering::Relaxed);
    let client_arc = state.client.clone();

    thread::spawn(move || {
        let (tx, rx) = mpsc::channel::<(String, String)>();
        // 日志转发线程: mpsc -> Tauri 事件
        let app_fwd = app.clone();
        let fwd = thread::spawn(move || {
            while let Ok((stream, text)) = rx.recv() {
                let _ = app_fwd.emit(
                    "openapk://log",
                    serde_json::json!({ "stream": stream, "text": text }),
                );
            }
        });
        let sys = |t: &str| {
            let _ = tx.send(("sys".to_string(), t.to_string()));
        };

        let run = || -> Result<()> {
            if cmds.is_empty() {
                bail!("未选择任何插件");
            }
            if !skip_update {
                let update_cmd = if allow_untrusted {
                    "apk update --allow-untrusted"
                } else {
                    "apk update"
                };
                sys(&format!("==> 全局更新索引: {}", update_cmd));
                let code = run_one(&client_arc, update_cmd, &abort, &tx)?;
                if code != 0 {
                    bail!("全局 apk update 失败(退出码 {}),已终止批量任务", code);
                }
            }
            let mut failed = 0usize;
            for (i, c) in cmds.iter().enumerate() {
                if abort.load(Ordering::Relaxed) {
                    bail!("任务已被用户中止");
                }
                sys(&format!("==> [{}/{}] {}", i + 1, cmds.len(), c));
                let code = run_one(&client_arc, c, &abort, &tx)?;
                if code != 0 {
                    failed += 1;
                    let _ = tx.send((
                        "err".to_string(),
                        format!("==> 命令失败(退出码 {}): {}", code, c),
                    ));
                    if stop_on_error {
                        bail!("任务因失败中断: {}", c);
                    }
                }
            }
            if failed > 0 {
                sys(&format!(
                    "==> 完成, 成功 {} 项, 失败 {} 项",
                    cmds.len() - failed,
                    failed
                ));
            }
            Ok(())
        };

        match run() {
            Ok(()) => sys("==> 全部插件安装完成"),
            Err(e) => {
                let _ = tx.send(("err".to_string(), format!("==> 执行中断: {}", e)));
            }
        }
        drop(tx);
        let _ = fwd.join();
        let _ = app.emit(
            "openapk://done",
            serde_json::json!({ "finished": true }),
        );
    });
    Ok(())
}

/// 查询路由器已安装软件包列表(原始输出,前端解析)
#[tauri::command]
pub fn list_installed(state: State<AppState>) -> Result<String, String> {
    let guard = state.client.lock().unwrap();
    let client = guard
        .as_ref()
        .context("尚未连接设备")
        .map_err(|e: anyhow::Error| e.to_string())?;
    let (out, _) = client
        .exec("apk list --installed 2>/dev/null")
        .map_err(|e| e.to_string())?;
    Ok(out)
}

#[tauri::command]
pub fn abort_install(state: State<AppState>) -> Result<(), String> {
    state.abort.store(true, Ordering::Relaxed);
    Ok(())
}
