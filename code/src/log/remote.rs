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
        let tcp = TcpStream::connect(&addr)
            .map_err(|e| anyhow::anyhow!("Failed to connect to {}: {}", addr, e))?;

        // 创建 SSH 会话
        let mut session = Session::new()
            .map_err(|e| anyhow::anyhow!("Failed to create SSH session: {}", e))?;
        session.set_tcp_stream(tcp);
        session.handshake()
            .map_err(|e| anyhow::anyhow!("SSH handshake failed: {}", e))?;

        // 认证
        match &self.config.auth {
            SshAuthConfig::Password { password } => {
                session.userauth_password(&self.config.username, password)
                    .map_err(|e| anyhow::anyhow!("SSH password authentication failed: {}", e))?;
            }
            SshAuthConfig::KeyFile { key_file } => {
                session.userauth_pubkey_file(
                    &self.config.username,
                    None,
                    key_file,
                    None::<&str>,
                ).map_err(|e| anyhow::anyhow!("SSH key authentication failed: {}", e))?;
            }
        }

        if !session.authenticated() {
            return Err(anyhow::anyhow!("SSH authentication failed - please check your credentials"));
        }

        // 执行命令
        let mut channel = session.channel_session()
            .map_err(|e| anyhow::anyhow!("Failed to create SSH channel: {}", e))?;
        channel.exec(&self.command)
            .map_err(|e| anyhow::anyhow!("Failed to execute command '{}': {}", self.command, e))?;

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

use std::cell::RefCell;

/// SSH 文件跟踪日志源
pub struct SshFileWatchSource {
    config: SshServerConfig,
    remote_path: String,
    running: Arc<AtomicBool>,
    entries: crossbeam_channel::Receiver<LogEntry>,
    sender: crossbeam_channel::Sender<LogEntry>,
    session: Arc<RefCell<Option<Session>>>,
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
            session: Arc::new(RefCell::new(None)),
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
        let session_arc = self.session.clone();

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

            // 保存 session 以便后续关闭
            *session_arc.borrow_mut() = Some(session);

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
            
            // 清理 session
            *session_arc.borrow_mut() = None;
        });

        Ok(())
    }

    fn stop(&mut self) -> anyhow::Result<()> {
        self.running.store(false, Ordering::SeqCst);
        // 关闭 SSH session 来强制终止连接
        if let Ok(mut session) = self.session.try_borrow_mut() {
            *session = None;
        }
        Ok(())
    }

    fn try_recv(&mut self) -> Option<LogEntry> {
        self.entries.try_recv().ok()
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}
