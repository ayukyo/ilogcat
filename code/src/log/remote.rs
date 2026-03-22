use ssh2::{Session, Channel};
use std::io::{BufRead, BufReader, Read};
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

use crate::log::{LogSource, LogEntry, LogSourceInfo};
use crate::log::parser::parse_log_line;
use crate::config::{SshServerConfig, SshAuthConfig};

/// SSH 远程日志源
pub struct SshSource {
    config: SshServerConfig,
    command: String,
    running: Arc<AtomicBool>,
    entries: crossbeam_channel::Receiver<LogEntry>,
    sender: crossbeam_channel::Sender<LogEntry>,
    session: Option<Session>,
}

impl SshSource {
    pub fn new(config: SshServerConfig, command: String) -> Self {
        let (sender, entries) = crossbeam_channel::unbounded();
        Self {
            config,
            command,
            running: Arc::new(AtomicBool::new(false)),
            entries,
            sender,
            session: None,
        }
    }

    /// 获取服务器名称
    pub fn server_name(&self) -> &str {
        &self.config.name
    }

    /// 获取命令
    pub fn command(&self) -> &str {
        &self.command
    }
}

impl LogSource for SshSource {
    fn start(&mut self) -> anyhow::Result<()> {
        let running = self.running.clone();
        running.store(true, Ordering::SeqCst);

        // 建立 TCP 连接
        let addr = format!("{}:{}", self.config.host, self.config.port);
        let tcp = TcpStream::connect(&addr)?;

        // 创建 SSH 会话
        let mut session = Session::new()?;
        session.set_tcp_stream(tcp);
        session.handshake()?;

        // 认证
        match &self.config.auth {
            SshAuthConfig::Password { password } => {
                session.userauth_password(&self.config.username, password)?;
            }
            SshAuthConfig::KeyFile { key_file } => {
                session.userauth_pubkey_file(
                    &self.config.username,
                    None,
                    key_file,
                    None::<&str>,
                )?;
            }
        }

        if !session.authenticated() {
            return Err(anyhow::anyhow!("SSH authentication failed"));
        }

        // 执行命令
        let mut channel = session.channel_session()?;
        channel.exec(&self.command)?;

        let sender = self.sender.clone();
        let source_info = LogSourceInfo::Remote(
            self.config.name.clone(),
            self.command.clone(),
        );

        // 启动读取线程
        let running_clone = running.clone();
        thread::spawn(move || {
            let stdout = channel.stream(0);
            let reader = BufReader::new(stdout);

            for line in reader.lines() {
                if !running_clone.load(Ordering::SeqCst) {
                    break;
                }

                if let Ok(line) = line {
                    if let Some(entry) = parse_log_line(&line, source_info.clone()) {
                        let _ = sender.send(entry);
                    }
                }
            }

            let _ = channel.wait_close();
        });

        self.session = Some(session);
        Ok(())
    }

    fn stop(&mut self) -> anyhow::Result<()> {
        self.running.store(false, Ordering::SeqCst);
        self.session = None;
        Ok(())
    }

    fn try_recv(&mut self) -> Option<LogEntry> {
        self.entries.try_recv().ok()
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

/// SSH 文件跟踪日志源
pub struct SshFileWatchSource {
    config: SshServerConfig,
    remote_path: String,
    running: Arc<AtomicBool>,
    entries: crossbeam_channel::Receiver<LogEntry>,
    sender: crossbeam_channel::Sender<LogEntry>,
}

impl SshFileWatchSource {
    pub fn new(config: SshServerConfig, remote_path: String) -> Self {
        let (sender, entries) = crossbeam_channel::unbounded();
        Self {
            config,
            remote_path,
            running: Arc::new(AtomicBool::new(false)),
            entries,
            sender,
        }
    }
}

impl LogSource for SshFileWatchSource {
    fn start(&mut self) -> anyhow::Result<()> {
        let running = self.running.clone();
        running.store(true, Ordering::SeqCst);

        // 使用 tail -f 跟踪远程文件
        let command = format!("tail -f {}", self.remote_path);
        let addr = format!("{}:{}", self.config.host, self.config.port);

        let config = self.config.clone();
        let sender = self.sender.clone();
        let source_info = LogSourceInfo::Remote(
            self.config.name.clone(),
            command.clone(),
        );

        thread::spawn(move || {
            let tcp = match TcpStream::connect(&addr) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("Failed to connect to {}: {}", addr, e);
                    return;
                }
            };

            let mut session = match Session::new() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Failed to create SSH session: {}", e);
                    return;
                }
            };

            session.set_tcp_stream(tcp);
            if let Err(e) = session.handshake() {
                eprintln!("SSH handshake failed: {}", e);
                return;
            }

            // 认证
            let auth_result = match &config.auth {
                SshAuthConfig::Password { password } => {
                    session.userauth_password(&config.username, password)
                }
                SshAuthConfig::KeyFile { key_file } => {
                    session.userauth_pubkey_file(&config.username, None, key_file, None::<&str>)
                }
            };

            if let Err(e) = auth_result {
                eprintln!("SSH authentication failed: {}", e);
                return;
            }

            let mut channel = match session.channel_session() {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Failed to create channel: {}", e);
                    return;
                }
            };

            if let Err(e) = channel.exec(&command) {
                eprintln!("Failed to execute command: {}", e);
                return;
            }

            let stdout = channel.stream(0);
            let reader = BufReader::new(stdout);

            for line in reader.lines() {
                if !running.load(Ordering::SeqCst) {
                    break;
                }

                if let Ok(line) = line {
                    if let Some(entry) = parse_log_line(&line, source_info.clone()) {
                        let _ = sender.send(entry);
                    }
                }
            }

            let _ = channel.wait_close();
        });

        Ok(())
    }

    fn stop(&mut self) -> anyhow::Result<()> {
        self.running.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn try_recv(&mut self) -> Option<LogEntry> {
        self.entries.try_recv().ok()
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}
