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
    pub custom_level_keywords: CustomLevelKeywords,
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

/// 自定义日志级别关键字配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomLevelKeywords {
    #[serde(default = "default_verbose_keywords")]
    pub verbose: Vec<String>,
    #[serde(default = "default_debug_keywords")]
    pub debug: Vec<String>,
    #[serde(default = "default_info_keywords")]
    pub info: Vec<String>,
    #[serde(default = "default_warn_keywords")]
    pub warn: Vec<String>,
    #[serde(default = "default_error_keywords")]
    pub error: Vec<String>,
    #[serde(default = "default_fatal_keywords")]
    pub fatal: Vec<String>,
}

impl Default for CustomLevelKeywords {
    fn default() -> Self {
        Self {
            verbose: default_verbose_keywords(),
            debug: default_debug_keywords(),
            info: default_info_keywords(),
            warn: default_warn_keywords(),
            error: default_error_keywords(),
            fatal: default_fatal_keywords(),
        }
    }
}

// 默认自定义关键字
fn default_verbose_keywords() -> Vec<String> {
    vec!["[v]".to_string(), "[verbose]".to_string()]
}

fn default_debug_keywords() -> Vec<String> {
    vec!["[d]".to_string(), "[debug]".to_string()]
}

fn default_info_keywords() -> Vec<String> {
    vec!["[i]".to_string(), "[info]".to_string()]
}

fn default_warn_keywords() -> Vec<String> {
    vec!["[w]".to_string(), "[warn]".to_string(), "[warning]".to_string()]
}

fn default_error_keywords() -> Vec<String> {
    vec!["[e]".to_string(), "[error]".to_string()]
}

fn default_fatal_keywords() -> Vec<String> {
    vec!["[f]".to_string(), "[fatal]".to_string()]
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
            custom_level_keywords: CustomLevelKeywords::default(),
            ssh_servers: Vec::new(),
            saved_filters: Vec::new(),
        }
    }
}

impl SshServerConfig {
    /// 从 ssh::config::SshConfig 转换
    pub fn from_ssh_config(config: crate::ssh::config::SshConfig) -> Self {
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
    
    /// 从 ssh::config::SshConfig 转换（用于对话框返回的配置）
    pub fn from(config: crate::ssh::config::SshConfig) -> Self {
        Self::from_ssh_config(config)
    }
}

impl Config {
    /// 获取配置文件路径
    pub fn config_path() -> Option<PathBuf> {
        ProjectDirs::from("com", "openclaw", "ilogcat")
            .map(|dirs| dirs.config_dir().join("config.toml"))
    }
    
    /// 添加 SSH 服务器配置
    pub fn add_ssh_server(&mut self, server: SshServerConfig) {
        // 检查是否已存在同名配置，如果存在则更新
        if let Some(existing) = self.ssh_servers.iter_mut().find(|s| s.name == server.name) {
            *existing = server;
        } else {
            self.ssh_servers.push(server);
        }
        // 自动保存配置
        let _ = self.save();
    }

    /// 更新自定义级别关键字
    pub fn update_custom_keywords(&mut self, level: &str, keywords: Vec<String>) {
        match level {
            "verbose" => self.custom_level_keywords.verbose = keywords,
            "debug" => self.custom_level_keywords.debug = keywords,
            "info" => self.custom_level_keywords.info = keywords,
            "warn" => self.custom_level_keywords.warn = keywords,
            "error" => self.custom_level_keywords.error = keywords,
            "fatal" => self.custom_level_keywords.fatal = keywords,
            _ => {}
        }
        let _ = self.save();
    }

    /// 获取所有自定义关键字（用于日志解析）
    pub fn get_all_custom_keywords(&self) -> std::collections::HashMap<String, String> {
        let mut map = std::collections::HashMap::new();
        
        for kw in &self.custom_level_keywords.verbose {
            map.insert(kw.to_lowercase(), "verbose".to_string());
        }
        for kw in &self.custom_level_keywords.debug {
            map.insert(kw.to_lowercase(), "debug".to_string());
        }
        for kw in &self.custom_level_keywords.info {
            map.insert(kw.to_lowercase(), "info".to_string());
        }
        for kw in &self.custom_level_keywords.warn {
            map.insert(kw.to_lowercase(), "warn".to_string());
        }
        for kw in &self.custom_level_keywords.error {
            map.insert(kw.to_lowercase(), "error".to_string());
        }
        for kw in &self.custom_level_keywords.fatal {
            map.insert(kw.to_lowercase(), "fatal".to_string());
        }
        
        map
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
    
    /// 导出配置到指定路径
    pub fn export_to(&self, path: &PathBuf) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
    
    /// 从指定路径导入配置
    pub fn import_from(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
    
    /// 合并导入的配置（保留现有 SSH 服务器和过滤器）
    pub fn merge(&mut self, other: Config) {
        // 合并 SSH 服务器（避免重复）
        for server in other.ssh_servers {
            if !self.ssh_servers.iter().any(|s| s.name == server.name) {
                self.ssh_servers.push(server);
            }
        }
        
        // 合并保存的过滤器（避免重复）
        for filter in other.saved_filters {
            if !self.saved_filters.iter().any(|f| f.name == filter.name) {
                self.saved_filters.push(filter);
            }
        }
        
        // 使用导入的自定义关键字（如果非默认）
        if other.custom_level_keywords != CustomLevelKeywords::default() {
            self.custom_level_keywords = other.custom_level_keywords;
        }
        
        // 使用导入的颜色配置（如果非默认）
        if other.colors != ColorConfig::default() {
            self.colors = other.colors;
        }
        
        // 使用导入的 UI 配置（如果非默认）
        if other.ui != UiConfig::default() {
            self.ui = other.ui;
        }
        
        // 使用导入的通用配置（如果非默认）
        if other.general != GeneralConfig::default() {
            self.general = other.general;
        }
    }
}
