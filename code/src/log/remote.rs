use ssh2::{Session, KeyboardInteractivePrompt, Prompt};
use std::borrow::Cow;
use std::io::{BufRead, BufReader, Read};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use chrono::{DateTime, Local, TimeZone};

use crate::log::{LogSource, LogEntry, LogSourceInfo};
use crate::log::parser::LogParser;
use crate::ssh::config::{SshConfig, AuthMethod};

/// 键盘交互认证提示处理器
pub struct PasswordPromptHandler {
    pub password: String,
}

impl KeyboardInteractivePrompt for PasswordPromptHandler {
    fn prompt<'a>(&mut self, _username: &str, _instructions: &str, prompts: &[Prompt<'a>]) -> Vec<String> {
        // 对所有提示都返回密码
        prompts.iter().map(|_| self.password.clone()).collect()
    }
}

/// 尝试多种SSH认证方法
fn authenticate_session(session: &mut Session, username: &str, password: &str) -> Result<(), ssh2::Error> {
    // 首先尝试空密码认证（某些服务器允许无密码访问）
    // 这会触发 SSH "none" 认证方法
    if session.userauth_password(username, "").is_ok() && session.authenticated() {
        return Ok(());
    }

    // 如果已经认证成功（某些服务器允许 none 认证），直接返回
    if session.authenticated() {
        return Ok(());
    }

    // 尝试用户提供的密码认证
    if session.userauth_password(username, password).is_ok() && session.authenticated() {
        return Ok(());
    }

    // 如果密码认证失败，尝试键盘交互认证
    if session.authenticated() {
        return Ok(());
    }

    // 使用键盘交互认证
    let mut handler = PasswordPromptHandler { password: password.to_string() };
    session.userauth_keyboard_interactive(username, &mut handler)
}

/// 获取服务器时间偏移量（服务器时间 - 本地时间，单位：秒）
fn get_server_time_offset(session: &Session) -> Option<i64> {
    let mut channel = session.channel_session().ok()?;
    channel.exec("date +%s").ok()?;

    let mut output = String::new();
    channel.read_to_string(&mut output).ok()?;
    let _ = channel.wait_close();

    let server_timestamp: i64 = output.trim().parse().ok()?;
    let local_timestamp = Local::now().timestamp();

    Some(server_timestamp - local_timestamp)
}

/// SSH 远程日志源
pub struct SshSource {
    config: SshConfig,
    command: String,
    running: Arc<AtomicBool>,
    entries: crossbeam_channel::Receiver<LogEntry>,
    sender: crossbeam_channel::Sender<LogEntry>,
    session: Option<Session>,
    error: Arc<Mutex<Option<String>>>,
    /// 服务器时间偏移量（服务器时间 - 本地时间，单位：秒）
    time_offset: Arc<Mutex<Option<i64>>>,
}

impl SshSource {
    pub fn new(config: SshConfig, command: String) -> Self {
        let (sender, entries) = crossbeam_channel::unbounded();
        Self {
            config,
            command,
            running: Arc::new(AtomicBool::new(false)),
            entries,
            sender,
            session: None,
            error: Arc::new(Mutex::new(None)),
            time_offset: Arc::new(Mutex::new(None)),
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

    /// 获取错误信息
    pub fn get_error(&self) -> Option<String> {
        self.error.lock().ok()?.clone()
    }

    /// 带超时的接收
    pub fn recv_timeout(&mut self, timeout: std::time::Duration) -> Option<LogEntry> {
        self.entries.recv_timeout(timeout).ok()
    }
}

impl LogSource for SshSource {
    fn start(&mut self) -> anyhow::Result<()> {
        let running = self.running.clone();
        running.store(true, Ordering::SeqCst);

        // 清除之前的错误
        if let Ok(mut err) = self.error.lock() {
            *err = None;
        }

        let addr = format!("{}:{}", self.config.host, self.config.port);

        // 建立 TCP 连接（带超时）
        let tcp = TcpStream::connect_timeout(
            &addr.parse().map_err(|e| anyhow::anyhow!("Invalid address {}: {}", addr, e))?,
            Duration::from_secs(self.config.timeout_secs),
        ).map_err(|e| {
            let err_msg = format!("连接 {} 失败: {} (请检查主机地址和端口是否正确)", addr, e);
            if let Ok(mut err) = self.error.lock() {
                *err = Some(err_msg.clone());
            }
            anyhow::anyhow!("{}", err_msg)
        })?;

        // 创建 SSH 会话
        let mut session = Session::new()
            .map_err(|e| anyhow::anyhow!("创建 SSH 会话失败: {}", e))?;
        session.set_tcp_stream(tcp);
        session.handshake()
            .map_err(|e| {
                let err_msg = format!("SSH 握手失败: {} (请确认目标主机运行SSH服务)", e);
                if let Ok(mut err) = self.error.lock() {
                    *err = Some(err_msg.clone());
                }
                anyhow::anyhow!("{}", err_msg)
            })?;

        // 认证 - 尝试多种认证方法
        match &self.config.auth {
            AuthMethod::Password(password) => {
                authenticate_session(&mut session, &self.config.username, password)
                    .map_err(|e| {
                        let err_msg = format!("SSH 认证失败: {} (请检查用户名和密码)", e);
                        if let Ok(mut err) = self.error.lock() {
                            *err = Some(err_msg.clone());
                        }
                        anyhow::anyhow!("{}", err_msg)
                    })?;
            }
            AuthMethod::KeyFile(key_file) => {
                session.userauth_pubkey_file(
                    &self.config.username,
                    None,
                    key_file,
                    self.config.key_passphrase.as_deref(),
                ).map_err(|e| {
                    let err_msg = format!("SSH 密钥认证失败: {}", e);
                    if let Ok(mut err) = self.error.lock() {
                        *err = Some(err_msg.clone());
                    }
                    anyhow::anyhow!("{}", err_msg)
                })?;
            }
        }

        if !session.authenticated() {
            let err_msg = "SSH 认证失败 - 请检查用户名和密码是否正确".to_string();
            if let Ok(mut err) = self.error.lock() {
                *err = Some(err_msg.clone());
            }
            return Err(anyhow::anyhow!("{}", err_msg));
        }

        // 获取服务器时间偏移量
        let time_offset = get_server_time_offset(&session);
        if let Ok(mut offset) = self.time_offset.lock() {
            *offset = time_offset;
        }
        if let Some(offset) = time_offset {
            eprintln!("Server time offset: {} seconds", offset);
        }

        // 执行命令
        let mut channel = session.channel_session()
            .map_err(|e| anyhow::anyhow!("创建 SSH 通道失败: {}", e))?;
        channel.exec(&self.command)
            .map_err(|e| anyhow::anyhow!("执行命令 '{}' 失败: {}", self.command, e))?;

        let sender = self.sender.clone();
        let source_info = LogSourceInfo::Remote(
            self.config.name.clone(),
            self.command.clone(),
        );
        let time_offset_for_thread = time_offset;

        // 启动读取线程 - 同时读取 stdout 和 stderr
        let running_clone = running.clone();
        thread::spawn(move || {
            let parser = LogParser::new();

            // 读取 stdout (stream 0)
            let stdout = channel.stream(0);
            let stdout_reader = BufReader::new(stdout);
            let sender_clone = sender.clone();
            let source_info_clone = source_info.clone();
            let running_for_stdout = running_clone.clone();

            for line in stdout_reader.lines() {
                if !running_for_stdout.load(Ordering::SeqCst) {
                    break;
                }
                if let Ok(line) = line {
                    if let Some(mut entry) = parser.parse_line(&line, source_info_clone.clone()) {
                        // 如果时间戳是当前时间（说明日志中没有时间戳），使用服务器时间偏移量
                        let now = Local::now();
                        let entry_time = entry.timestamp;
                        // 如果时间戳在当前时间的±1秒内，说明是使用 Local::now() 生成的
                        if (entry_time.timestamp() - now.timestamp()).abs() <= 1 {
                            if let Some(offset) = time_offset_for_thread {
                                // 应用服务器时间偏移量
                                let server_time = now.timestamp() + offset;
                                if let Some(dt) = Local.timestamp_opt(server_time, 0).single() {
                                    entry.timestamp = dt;
                                }
                            }
                        }
                        let _ = sender_clone.send(entry);
                    }
                }
            }

            // 读取 stderr (stream 1) - 扩展流
            // 注意: ssh2 的 channel.stream(1) 用于读取扩展数据（stderr）
            let stderr = channel.stream(1);
            let stderr_reader = BufReader::new(stderr);
            let running_for_stderr = running_clone.clone();

            for line in stderr_reader.lines() {
                if !running_for_stderr.load(Ordering::SeqCst) {
                    break;
                }
                if let Ok(line) = line {
                    // stderr 也作为日志处理
                    if let Some(mut entry) = parser.parse_line(&line, source_info.clone()) {
                        // 如果时间戳是当前时间，使用服务器时间偏移量
                        let now = Local::now();
                        let entry_time = entry.timestamp;
                        if (entry_time.timestamp() - now.timestamp()).abs() <= 1 {
                            if let Some(offset) = time_offset_for_thread {
                                let server_time = now.timestamp() + offset;
                                if let Some(dt) = Local.timestamp_opt(server_time, 0).single() {
                                    entry.timestamp = dt;
                                }
                            }
                        }
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
    config: SshConfig,
    remote_path: String,
    running: Arc<AtomicBool>,
    entries: crossbeam_channel::Receiver<LogEntry>,
    sender: crossbeam_channel::Sender<LogEntry>,
    session: Arc<Mutex<Option<Session>>>,
    /// 服务器时间偏移量（服务器时间 - 本地时间，单位：秒）
    time_offset: Arc<Mutex<Option<i64>>>,
}

impl SshFileWatchSource {
    pub fn new(config: SshConfig, remote_path: String) -> Self {
        let (sender, entries) = crossbeam_channel::unbounded();
        Self {
            config,
            remote_path,
            running: Arc::new(AtomicBool::new(false)),
            entries,
            sender,
            session: Arc::new(Mutex::new(None)),
            time_offset: Arc::new(Mutex::new(None)),
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
        let time_offset_arc = self.time_offset.clone();

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

            // 认证 - 尝试多种认证方法
            let auth_result = match &config.auth {
                AuthMethod::Password(password) => {
                    authenticate_session(&mut session, &config.username, password)
                }
                AuthMethod::KeyFile(key_file) => {
                    session.userauth_pubkey_file(&config.username, None, key_file, config.key_passphrase.as_deref())
                }
            };

            if let Err(e) = auth_result {
                eprintln!("SSH authentication failed: {}", e);
                return;
            }

            // 获取服务器时间偏移量
            let time_offset = get_server_time_offset(&session);
            if let Ok(mut offset) = time_offset_arc.lock() {
                *offset = time_offset;
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
            *session_arc.lock().unwrap() = Some(session);

            let stdout = channel.stream(0);
            let reader = BufReader::new(stdout);
            let parser = LogParser::new();

            for line in reader.lines() {
                if !running.load(Ordering::SeqCst) {
                    break;
                }

                if let Ok(line) = line {
                    if let Some(mut entry) = parser.parse_line(&line, source_info.clone()) {
                        // 如果时间戳是当前时间，使用服务器时间偏移量
                        let now = Local::now();
                        let entry_time = entry.timestamp;
                        if (entry_time.timestamp() - now.timestamp()).abs() <= 1 {
                            if let Some(offset) = time_offset {
                                let server_time = now.timestamp() + offset;
                                if let Some(dt) = Local.timestamp_opt(server_time, 0).single() {
                                    entry.timestamp = dt;
                                }
                            }
                        }
                        let _ = sender.send(entry);
                    }
                }
            }

            let _ = channel.wait_close();

            // 清理 session
            if let Ok(mut session) = session_arc.lock() {
                *session = None;
            }
        });

        Ok(())
    }

    fn stop(&mut self) -> anyhow::Result<()> {
        self.running.store(false, Ordering::SeqCst);
        // 关闭 SSH session 来强制终止连接
        if let Ok(mut session) = self.session.lock() {
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
