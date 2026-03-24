use ssh2::{Session, Channel};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::PathBuf;
use std::time::Duration;
use anyhow::{Result, Context};

use crate::ssh::config::SshConfig;

/// 连接状态
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Authenticating,
    Connected,
    Failed,
}

impl std::fmt::Display for ConnectionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionState::Disconnected => write!(f, "Disconnected"),
            ConnectionState::Connecting => write!(f, "Connecting..."),
            ConnectionState::Authenticating => write!(f, "Authenticating..."),
            ConnectionState::Connected => write!(f, "Connected"),
            ConnectionState::Failed => write!(f, "Failed"),
        }
    }
}

/// SSH 客户端
pub struct SshClient {
    config: SshConfig,
    session: Option<Session>,
    state: ConnectionState,
    last_error: Option<String>,
    connect_timeout: Duration,
    retry_count: u32,
}

impl SshClient {
    /// 创建新的 SSH 客户端
    pub fn new(config: SshConfig) -> Self {
        Self {
            config,
            session: None,
            state: ConnectionState::Disconnected,
            last_error: None,
            connect_timeout: Duration::from_secs(10),
            retry_count: 0,
        }
    }

    /// 设置连接超时
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.connect_timeout = timeout;
    }

    /// 获取当前连接状态
    pub fn state(&self) -> ConnectionState {
        self.state
    }

    /// 获取最后一次错误信息
    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    /// 获取重试次数
    pub fn retry_count(&self) -> u32 {
        self.retry_count
    }

    /// 连接到 SSH 服务器（带重试机制）
    pub fn connect(&mut self) -> Result<()> {
        self.connect_with_retry(3)
    }

    /// 连接到 SSH 服务器（带重试）
    fn connect_with_retry(&mut self, max_retries: u32) -> Result<()> {
        let mut last_error = None;

        for attempt in 0..max_retries {
            self.retry_count = attempt;
            
            match self.try_connect() {
                Ok(()) => {
                    self.retry_count = 0;
                    return Ok(());
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt < max_retries - 1 {
                        // 等待后重试
                        std::thread::sleep(Duration::from_millis(500 * (attempt + 1) as u64));
                    }
                }
            }
        }

        self.state = ConnectionState::Failed;
        let err = last_error.unwrap_or_else(|| anyhow::anyhow!("Unknown connection error"));
        self.last_error = Some(err.to_string());
        Err(err)
    }

    /// 尝试单次连接
    fn try_connect(&mut self) -> Result<()> {
        self.state = ConnectionState::Connecting;
        self.last_error = None;

        let addr = format!("{}:{}", self.config.host, self.config.port);
        
        // 使用超时连接
        let tcp = TcpStream::connect_timeout(
            &addr.to_socket_addrs()?
                .next()
                .context("Failed to resolve address")?,
            self.connect_timeout
        ).with_context(|| format!("Failed to connect to {} within {:?}", addr, self.connect_timeout))?;

        tcp.set_read_timeout(Some(Duration::from_secs(30)))?;
        tcp.set_write_timeout(Some(Duration::from_secs(30)))?;

        let mut session = Session::new()?;
        session.set_tcp_stream(tcp);
        session.handshake().context("SSH handshake failed")?;

        self.state = ConnectionState::Authenticating;

        // 认证
        match &self.config.auth {
            crate::ssh::config::AuthMethod::Password(password) => {
                session.userauth_password(&self.config.username, password)
                    .context("Password authentication failed")?;
            }
            crate::ssh::config::AuthMethod::KeyFile(key_file) => {
                session.userauth_pubkey_file(
                    &self.config.username,
                    None,
                    key_file,
                    self.config.key_passphrase.as_deref(),
                ).with_context(|| format!("Key authentication failed with key file: {:?}", key_file))?;
            }
        }

        if !session.authenticated() {
            return Err(anyhow::anyhow!("SSH authentication failed - please check your credentials"));
        }

        self.session = Some(session);
        self.state = ConnectionState::Connected;
        Ok(())
    }

    /// 断开连接
    pub fn disconnect(&mut self) {
        if let Some(session) = self.session.take() {
            // 尝试优雅地关闭会话
            let _ = session.disconnect(None, "Disconnecting", None);
        }
        self.state = ConnectionState::Disconnected;
        self.last_error = None;
    }

    /// 测试连接是否仍然有效
    pub fn is_alive(&mut self) -> bool {
        if let Some(session) = &self.session {
            // 尝试执行一个简单的命令来测试连接
            match session.channel_session() {
                Ok(mut channel) => {
                    if channel.exec("echo ping").is_ok() {
                        let _ = channel.wait_close();
                        return true;
                    }
                }
                Err(_) => {}
            }
        }
        self.state = ConnectionState::Failed;
        false
    }

    /// 执行命令并返回输出
    pub fn exec(&mut self, command: &str) -> Result<String> {
        let session = self.session.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;

        let mut channel = session.channel_session()?;
        channel.exec(command)?;

        let mut output = String::new();
        channel.read_to_string(&mut output)?;

        channel.wait_close()?;
        Ok(output)
    }

    /// 执行命令并返回逐行输出
    pub fn exec_stream(&mut self, command: &str) -> Result<impl Iterator<Item = String>> {
        let session = self.session.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;

        let mut channel = session.channel_session()?;
        channel.exec(command)?;

        let reader = BufReader::new(channel.stream(0));
        Ok(reader.lines().filter_map(|line| line.ok()))
    }

    /// 检查是否已连接
    pub fn is_connected(&self) -> bool {
        self.session.is_some() && self.state == ConnectionState::Connected
    }

    /// 检查连接状态（包括连接中和认证中）
    pub fn is_connecting(&self) -> bool {
        matches!(self.state, ConnectionState::Connecting | ConnectionState::Authenticating)
    }

    /// 获取配置
    pub fn config(&self) -> &SshConfig {
        &self.config
    }
}

/// SSH 连接管理器
pub struct SshConnectionManager {
    connections: Vec<SshClient>,
}

impl SshConnectionManager {
    /// 创建新的连接管理器
    pub fn new() -> Self {
        Self {
            connections: Vec::new(),
        }
    }

    /// 添加连接
    pub fn add_connection(&mut self, client: SshClient) {
        self.connections.push(client);
    }

    /// 获取连接
    pub fn get_connection(&mut self, name: &str) -> Option<&mut SshClient> {
        self.connections.iter_mut()
            .find(|c| c.config().name == name)
    }

    /// 断开所有连接
    pub fn disconnect_all(&mut self) {
        for client in &mut self.connections {
            client.disconnect();
        }
        self.connections.clear();
    }

    /// 获取连接列表
    pub fn list_connections(&self) -> Vec<&SshConfig> {
        self.connections.iter()
            .map(|c| c.config())
            .collect()
    }
}

impl Default for SshConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}
