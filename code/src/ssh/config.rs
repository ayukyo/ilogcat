use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// SSH 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshConfig {
    pub name: String,
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    pub username: String,
    pub auth: AuthMethod,
    /// 密钥密码短语（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_passphrase: Option<String>,
    /// 连接超时（秒）
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AuthMethod {
    #[serde(rename = "password")]
    Password(String),
    #[serde(rename = "key")]
    KeyFile(PathBuf),
}

fn default_port() -> u16 {
    22
}

fn default_timeout() -> u64 {
    10
}

impl SshConfig {
    /// 创建新的 SSH 配置
    pub fn new(name: String, host: String, username: String) -> Self {
        Self {
            name,
            host,
            port: 22,
            username,
            auth: AuthMethod::Password(String::new()),
            key_passphrase: None,
            timeout_secs: 10,
        }
    }

    /// 设置端口
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// 设置密码认证
    pub fn with_password(mut self, password: String) -> Self {
        self.auth = AuthMethod::Password(password);
        self
    }

    /// 设置密钥认证
    pub fn with_key_file(mut self, key_file: PathBuf) -> Self {
        self.auth = AuthMethod::KeyFile(key_file);
        self
    }

    /// 设置密钥密码短语
    pub fn with_key_passphrase(mut self, passphrase: String) -> Self {
        self.key_passphrase = Some(passphrase);
        self
    }

    /// 设置连接超时
    pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
        self.timeout_secs = timeout_secs;
        self
    }

    /// 获取显示名称
    pub fn display_name(&self) -> String {
        format!("{}@{}:{}", self.username, self.host, self.port)
    }

    /// 获取连接超时
    pub fn timeout(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.timeout_secs)
    }
}

/// SSH 配置列表
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshConfigList {
    pub servers: Vec<SshConfig>,
}

impl SshConfigList {
    /// 创建新的配置列表
    pub fn new() -> Self {
        Self {
            servers: Vec::new(),
        }
    }

    /// 添加服务器配置
    pub fn add(&mut self, config: SshConfig) {
        self.servers.push(config);
    }

    /// 获取服务器配置
    pub fn get(&self, name: &str) -> Option<&SshConfig> {
        self.servers.iter().find(|s| s.name == name)
    }

    /// 删除服务器配置
    pub fn remove(&mut self, name: &str) -> Option<SshConfig> {
        let index = self.servers.iter().position(|s| s.name == name)?;
        Some(self.servers.remove(index))
    }

    /// 获取服务器名称列表
    pub fn names(&self) -> Vec<String> {
        self.servers.iter().map(|s| s.name.clone()).collect()
    }
}

impl Default for SshConfigList {
    fn default() -> Self {
        Self::new()
    }
}
