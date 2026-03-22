use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use anyhow::Result;
use directories::ProjectDirs;

/// 应用配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default)]
    pub colors: ColorConfig,
    #[serde(default)]
    pub ssh_servers: Vec<SshServerConfig>,
    #[serde(default)]
    pub saved_filters: Vec<SavedFilter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    #[serde(default = "default_log_level")]
    pub default_log_level: String,
    #[serde(default = "default_max_lines")]
    pub max_log_lines: usize,
    #[serde(default = "default_auto_scroll")]
    pub auto_scroll: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    #[serde(default = "default_font")]
    pub font: String,
    #[serde(default = "default_theme")]
    pub theme: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorConfig {
    #[serde(default = "default_color_verbose")]
    pub verbose: String,
    #[serde(default = "default_color_debug")]
    pub debug: String,
    #[serde(default = "default_color_info")]
    pub info: String,
    #[serde(default = "default_color_warn")]
    pub warn: String,
    #[serde(default = "default_color_error")]
    pub error: String,
    #[serde(default = "default_color_fatal")]
    pub fatal: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshServerConfig {
    pub name: String,
    pub host: String,
    #[serde(default = "default_ssh_port")]
    pub port: u16,
    pub username: String,
    pub auth: SshAuthConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SshAuthConfig {
    #[serde(rename = "password")]
    Password { password: String },
    #[serde(rename = "key")]
    KeyFile { key_file: PathBuf },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedFilter {
    pub name: String,
    pub keywords: Vec<String>,
    #[serde(default)]
    pub logic: FilterLogic,
    pub levels: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum FilterLogic {
    And,
    Or,
}

impl Default for FilterLogic {
    fn default() -> Self {
        FilterLogic::Or
    }
}

// 默认值函数
fn default_log_level() -> String { "Info".to_string() }
fn default_max_lines() -> usize { 100000 }
fn default_auto_scroll() -> bool { true }
fn default_font() -> String { "Monospace 12".to_string() }
fn default_theme() -> String { "dark".to_string() }
fn default_ssh_port() -> u16 { 22 }
fn default_color_verbose() -> String { "#808080".to_string() }
fn default_color_debug() -> String { "#0066CC".to_string() }
fn default_color_info() -> String { "#008800".to_string() }
fn default_color_warn() -> String { "#FF8800".to_string() }
fn default_color_error() -> String { "#CC0000".to_string() }
fn default_color_fatal() -> String { "#CC0000".to_string() }

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            default_log_level: default_log_level(),
            max_log_lines: default_max_lines(),
            auto_scroll: default_auto_scroll(),
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            font: default_font(),
            theme: default_theme(),
        }
    }
}

impl Default for ColorConfig {
    fn default() -> Self {
        Self {
            verbose: default_color_verbose(),
            debug: default_color_debug(),
            info: default_color_info(),
            warn: default_color_warn(),
            error: default_color_error(),
            fatal: default_color_fatal(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            ui: UiConfig::default(),
            colors: ColorConfig::default(),
            ssh_servers: Vec::new(),
            saved_filters: Vec::new(),
        }
    }
}

impl SshServerConfig {
    /// 从 ssh::config::SshConfig 转换
    pub fn from(config: crate::ssh::config::SshConfig) -> Self {
        let auth = match config.auth {
            crate::ssh::config::AuthMethod::Password(pwd) => SshAuthConfig::Password { password: pwd },
            crate::ssh::config::AuthMethod::KeyFile(path) => SshAuthConfig::KeyFile { key_file: path },
        };
        
        Self {
            name: config.name,
            host: config.host,
            port: config.port,
            username: config.username,
            auth,
        }
    }
}

impl Config {
    /// 获取配置文件路径
    pub fn config_path() -> Option<PathBuf> {
        ProjectDirs::from("com", "openclaw", "ilogcat")
            .map(|dirs| dirs.config_dir().join("config.toml"))
    }

    /// 加载配置
    pub fn load() -> Result<Self> {
        if let Some(path) = Self::config_path() {
            if path.exists() {
                let content = std::fs::read_to_string(&path)?;
                let config: Config = toml::from_str(&content)?;
                return Ok(config);
            }
        }
        Ok(Self::default())
    }

    /// 保存配置
    pub fn save(&self) -> Result<()> {
        if let Some(path) = Self::config_path() {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let content = toml::to_string_pretty(self)?;
            std::fs::write(&path, content)?;
        }
        Ok(())
    }
}
