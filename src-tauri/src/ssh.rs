use anyhow::{bail, Context, Result};
use ssh2::Session;
use std::io::Read;
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::{Duration, Instant};

/// 基于 ssh2 的同步 SSH 客户端(在独立线程中使用,不阻塞 UI)
pub struct SshClient {
    pub sess: Session,
}

impl SshClient {
    pub fn connect(host: &str, port: u16, user: &str, password: &str) -> Result<Self> {
        let addr_str = format!("{}:{}", host, port);
        let addr = addr_str
            .to_socket_addrs()
            .context("无法解析主机地址")?
            .next()
            .context("无法解析主机地址")?;
        let tcp = TcpStream::connect_timeout(&addr, Duration::from_secs(8))
            .context("SSH TCP 连接失败(超时或被拒绝)")?;
        // 握手与认证阶段设置读超时,防止对端非 SSH 服务导致永久阻塞
        tcp.set_read_timeout(Some(Duration::from_secs(15))).ok();
        let mut sess = Session::new()?;
        sess.set_tcp_stream(tcp.try_clone().context("复制 TCP 句柄失败")?);
        sess.handshake().context("SSH 握手失败")?;
        sess.userauth_password(user, password)
            .context("SSH 认证失败,请检查用户名或密码")?;
        if !sess.authenticated() {
            bail!("SSH 认证失败");
        }
        // 认证完成后解除读超时,避免长命令(如 apk update)被误杀
        tcp.set_read_timeout(None).ok();
        Ok(Self { sess })
    }

    /// 执行普通命令,合并返回 (stdout+stderr, exit_code)
    pub fn exec(&self, cmd: &str) -> Result<(String, i32)> {
        let mut ch = self.sess.channel_session()?;
        ch.exec(cmd)?;
        let mut out = String::new();
        ch.read_to_string(&mut out).ok();
        let mut err = String::new();
        ch.stderr().read_to_string(&mut err).ok();
        ch.wait_close()?;
        let code = ch.exit_status().unwrap_or(-1);
        Ok((format!("{}{}", out, err), code))
    }

    /// 流式执行: 非阻塞读循环,实时按行推送 stdout/stderr,支持中止信号与超时强杀
    pub fn exec_stream(
        &self,
        cmd: &str,
        timeout: Duration,
        abort: &AtomicBool,
        tx: &Sender<(String, String)>,
    ) -> Result<i32> {
        let mut ch = self.sess.channel_session()?;
        ch.exec(cmd)?;
        self.sess.set_blocking(false);
        let start = Instant::now();
        let mut stdout_buf: Vec<u8> = Vec::new();
        let mut stderr_buf: Vec<u8> = Vec::new();
        let mut buf = [0u8; 8192];
        let mut failed: Option<anyhow::Error> = None;

        loop {
            if abort.load(Ordering::Relaxed) {
                let _ = ch.close();
                failed = Some(anyhow::anyhow!("任务已被用户中止"));
                break;
            }
            if start.elapsed() > timeout {
                let _ = ch.close();
                failed = Some(anyhow::anyhow!(
                    "命令执行超时({}s),已强制终止",
                    timeout.as_secs()
                ));
                break;
            }
            let mut got_data = false;
            match ch.read(&mut buf) {
                Ok(0) => {}
                Ok(n) => {
                    got_data = true;
                    stdout_buf.extend_from_slice(&buf[..n]);
                    drain_lines(&mut stdout_buf, tx, "stdout");
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(e) => {
                    failed = Some(e.into());
                    break;
                }
            }
            match ch.stderr().read(&mut buf) {
                Ok(0) => {}
                Ok(n) => {
                    got_data = true;
                    stderr_buf.extend_from_slice(&buf[..n]);
                    drain_lines(&mut stderr_buf, tx, "stderr");
                }
                _ => {}
            }
            if ch.eof() {
                if !stdout_buf.is_empty() {
                    let s = String::from_utf8_lossy(&stdout_buf).to_string();
                    let _ = tx.send(("stdout".into(), s));
                    stdout_buf.clear();
                }
                if !stderr_buf.is_empty() {
                    let s = String::from_utf8_lossy(&stderr_buf).to_string();
                    let _ = tx.send(("stderr".into(), s));
                    stderr_buf.clear();
                }
                break;
            }
            if !got_data {
                thread::sleep(Duration::from_millis(40));
            }
        }
        self.sess.set_blocking(true);
        if let Some(e) = failed {
            return Err(e);
        }
        ch.wait_close()?;
        Ok(ch.exit_status().unwrap_or(-1))
    }
}

/// 从缓冲区按行切分并推送,保留未完整的尾部行
fn drain_lines(buf: &mut Vec<u8>, tx: &Sender<(String, String)>, stream: &str) {
    while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
        let line: Vec<u8> = buf.drain(..=pos).collect();
        let s = String::from_utf8_lossy(&line)
            .trim_end_matches(['\r', '\n'])
            .to_string();
        let _ = tx.send((stream.to_string(), s));
    }
}
