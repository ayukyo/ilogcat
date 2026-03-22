use chrono::{DateTime, Local};
use std::fmt;

/// 日志级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogLevel {
    Verbose,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::Verbose => write!(f, "V"),
            LogLevel::Debug => write!(f, "D"),
            LogLevel::Info => write!(f, "I"),
            LogLevel::Warn => write!(f, "W"),
            LogLevel::Error => write!(f, "E"),
            LogLevel::Fatal => write!(f, "F"),
        }
    }
}

impl LogLevel {
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            'V' | 'v' => Some(LogLevel::Verbose),
            'D' | 'd' => Some(LogLevel::Debug),
            'I' | 'i' => Some(LogLevel::Info),
            'W' | 'w' => Some(LogLevel::Warn),
            'E' | 'e' => Some(LogLevel::Error),
            'F' | 'f' => Some(LogLevel::Fatal),
            _ => None,
        }
    }
    
    pub fn to_string_full(&self) -> String {
        match self {
            LogLevel::Verbose => "VERBOSE".to_string(),
            LogLevel::Debug => "DEBUG".to_string(),
            LogLevel::Info => "INFO".to_string(),
            LogLevel::Warn => "WARN".to_string(),
            LogLevel::Error => "ERROR".to_string(),
            LogLevel::Fatal => "FATAL".to_string(),
        }
    }
}

/// 日志来源信息
#[derive(Debug, Clone)]
pub enum LogSourceInfo {
    Local(String),           // 本地命令
    File(String),            // 文件路径
    Remote(String, String),  // (服务器名, 命令)
}

/// 日志条目
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: DateTime<Local>,
    pub level: LogLevel,
    pub tag: String,
    pub pid: Option<u32>,
    pub message: String,
    pub source: LogSourceInfo,
    pub raw_line: String,
}

/// 日志源 trait
pub trait LogSource: Send {
    /// 启动日志源
    fn start(&mut self) -> anyhow::Result<()>;
    
    /// 停止日志源
    fn stop(&mut self) -> anyhow::Result<()>;
    
    /// 尝试读取一条日志（非阻塞）
    fn try_recv(&mut self) -> Option<LogEntry>;
    
    /// 是否正在运行
    fn is_running(&self) -> bool;
}

/// 为 Box<dyn LogSource + Send> 实现 LogSource trait，使其可以作为 trait 对象使用
impl LogSource for Box<dyn LogSource + Send> {
    fn start(&mut self) -> anyhow::Result<()> {
        (**self).start()
    }
    
    fn stop(&mut self) -> anyhow::Result<()> {
        (**self).stop()
    }
    
    fn try_recv(&mut self) -> Option<LogEntry> {
        (**self).try_recv()
    }
    
    fn is_running(&self) -> bool {
        (**self).is_running()
    }
}