use ssh2::{Session, Channel};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use anyhow::Result;

use crate::ssh::config::SshConfig;

/// SSH 客户端
pub struct SshClient {
    config: SshConfig,
    session: Option<Session>,
}

impl SshClient {
    /// 创建新的 SSH 客户端
    pub fn new(config: SshConfig) -> Self {
        Self {
            config,
            session: None,
        }
    }

    /// 连接到 SSH 服务器
    pub fn connect(&mut self) -> Result<()> {
        let addr = format!("{}:{}", self.config.host, self.config.port);
        let tcp = TcpStream::connect(&addr)?;

        let mut session = Session::new()?;
        session.set_tcp_stream(tcp);
        session.handshake()?;

        // 认证
        match &self.config.auth {
            crate::ssh::config::AuthMethod::Password(password) => {
                session.userauth_password(&self.config.username, password)?;
            }
            crate::ssh::config::AuthMethod::KeyFile(key_file) => {
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

        self.session = Some(session);
        Ok(())
    }

    /// 断开连接
    pub fn disconnect(&mut self) {
        self.session = None;
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
        self.session.is_some()
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
