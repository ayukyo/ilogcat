use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use anyhow::Result;
use directories::ProjectDirs;

// 重导出 SSH 配置，方便其他模块使用
pub use crate::ssh::config::{SshConfig, AuthMethod};

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
    pub ssh_servers: Vec<SshConfig>,
    #[serde(default)]
    pub saved_filters: Vec<SavedFilter>,
    #[serde(default)]
    pub command_history: Vec<String>,
    #[serde(default)]
    pub last_ssh_input: Option<LastSshInput>,
}

/// 上次SSH输入记录
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LastSshInput {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GeneralConfig {
    #[serde(default = "default_log_level")]
    pub default_log_level: String,
    #[serde(default = "default_max_lines")]
    pub max_log_lines: usize,
    #[serde(default = "default_auto_scroll")]
    pub auto_scroll: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiConfig {
    #[serde(default = "default_font")]
    pub font: String,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_language")]
    pub language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CustomLevelKeywords {
    #[serde(default = "default_trace_keywords")]
    pub trace: Vec<String>,
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
    #[serde(default = "default_critical_keywords")]
    pub critical: Vec<String>,
}

impl Default for CustomLevelKeywords {
    fn default() -> Self {
        Self {
            trace: default_trace_keywords(),
            verbose: default_verbose_keywords(),
            debug: default_debug_keywords(),
            info: default_info_keywords(),
            warn: default_warn_keywords(),
            error: default_error_keywords(),
            fatal: default_fatal_keywords(),
            critical: default_critical_keywords(),
        }
    }
}

// 默认自定义关键字
fn default_trace_keywords() -> Vec<String> {
    vec!["[trace]".to_string()]
}

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

fn default_critical_keywords() -> Vec<String> {
    vec!["[critical]".to_string()]
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
fn default_theme() -> String { "light".to_string() }
fn default_language() -> String { "zh".to_string() }
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
            language: default_language(),
        }
    }
}

impl UiConfig {
    /// 设置语言
    pub fn set_language(&mut self, lang: &str) {
        self.language = lang.to_string();
    }
    
    /// 获取当前语言
    pub fn current_language(&self) -> &str {
        &self.language
    }
}

impl UiConfig {
    /// 设置主题
    pub fn set_theme(&mut self, theme: &str) {
        self.theme = theme.to_string();
    }
    
    /// 检查是否为暗色主题
    pub fn is_dark_theme(&self) -> bool {
        self.theme == "dark"
    }
    
    /// 获取当前主题
    pub fn current_theme(&self) -> &str {
        &self.theme
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
            command_history: Vec::new(),
            last_ssh_input: None,
        }
    }
}

impl Config {
    /// 获取配置文件路径
    pub fn config_path() -> Option<PathBuf> {
        ProjectDirs::from("com", "openclaw", "ilogcat")
            .map(|dirs| dirs.config_dir().join("config.toml"))
    }

    /// 添加 SSH 服务器配置
    pub fn add_ssh_server(&mut self, server: SshConfig) {
        // 检查是否已存在同名配置，如果存在则更新
        if let Some(existing) = self.ssh_servers.iter_mut().find(|s| s.name == server.name) {
            *existing = server;
        } else {
            self.ssh_servers.push(server);
        }
        // 自动保存配置
        let _ = self.save();
    }

    /// 添加命令到历史记录
    pub fn add_command_history(&mut self, command: String) {
        // 移除已存在的相同命令
        self.command_history.retain(|c| c != &command);
        // 添加到开头
        self.command_history.insert(0, command);
        // 限制历史记录数量
        if self.command_history.len() > 50 {
            self.command_history.truncate(50);
        }
        // 自动保存
        let _ = self.save();
    }

    /// 获取命令历史
    pub fn get_command_history(&self) -> &[String] {
        &self.command_history
    }

    /// 保存上次SSH输入
    pub fn save_last_ssh_input(&mut self, name: &str, host: &str, port: u16, username: &str) {
        self.last_ssh_input = Some(LastSshInput {
            name: name.to_string(),
            host: host.to_string(),
            port,
            username: username.to_string(),
        });
        let _ = self.save();
    }

    /// 获取上次SSH输入
    pub fn get_last_ssh_input(&self) -> Option<&LastSshInput> {
        self.last_ssh_input.as_ref()
    }

    /// 更新自定义级别关键字
    pub fn update_custom_keywords(&mut self, level: &str, keywords: Vec<String>) {
        match level {
            "trace" => self.custom_level_keywords.trace = keywords,
            "verbose" => self.custom_level_keywords.verbose = keywords,
            "debug" => self.custom_level_keywords.debug = keywords,
            "info" => self.custom_level_keywords.info = keywords,
            "warn" => self.custom_level_keywords.warn = keywords,
            "error" => self.custom_level_keywords.error = keywords,
            "fatal" => self.custom_level_keywords.fatal = keywords,
            "critical" => self.custom_level_keywords.critical = keywords,
            _ => {}
        }
        let _ = self.save();
    }

    /// 获取所有自定义关键字（用于日志解析）
    pub fn get_all_custom_keywords(&self) -> std::collections::HashMap<String, String> {
        let mut map = std::collections::HashMap::new();

        for kw in &self.custom_level_keywords.trace {
            map.insert(kw.to_lowercase(), "trace".to_string());
        }
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
        for kw in &self.custom_level_keywords.critical {
            map.insert(kw.to_lowercase(), "critical".to_string());
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
