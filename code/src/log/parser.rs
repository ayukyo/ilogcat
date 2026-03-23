use regex::Regex;
use chrono::{DateTime, Local, NaiveTime, Timelike, Datelike};
use std::collections::HashMap;
use crate::log::{LogEntry, LogLevel, LogSourceInfo};

/// 日志行解析器
pub struct LogParser {
    /// Android logcat 格式
    /// 例如: 10-22 14:30:45.123 D/MyApp( 1234): This is a debug message
    logcat_pattern: Regex,
    
    /// 简单格式
    /// 例如: [2024-10-22 14:30:45] [ERROR] message
    bracket_pattern: Regex,
    
    /// syslog 格式
    /// 例如: Oct 22 14:30:45 hostname app[1234]: message
    syslog_pattern: Regex,
    
    /// 自定义级别关键字映射
    custom_keywords: HashMap<String, String>,
}

impl LogParser {
    pub fn new() -> Self {
        Self::with_keywords(HashMap::new())
    }
    
    /// 使用自定义关键字创建解析器
    pub fn with_keywords(custom_keywords: HashMap<String, String>) -> Self {
        Self {
            logcat_pattern: Regex::new(
                r"(?m)^(\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2}\.\d{3})\s+([VDIWEF])/([^\(]+?)\(\s*(\d+)\):\s*(.*)$"
            ).unwrap(),
            
            bracket_pattern: Regex::new(
                r"(?m)^\[([^\]]+)\]\s*\[([^\]]+)\]\s*(.*)$"
            ).unwrap(),
            
            syslog_pattern: Regex::new(
                r"(?m)^(\w{3}\s+\d{1,2}\s+\d{2}:\d{2}:\d{2})\s+(\S+)\s+(\S+?)(?:\[(\d+)\])?:\s*(.*)$"
            ).unwrap(),
            
            custom_keywords,
        }
    }
    
    /// 更新自定义关键字
    pub fn set_custom_keywords(&mut self, keywords: HashMap<String, String>) {
        self.custom_keywords = keywords;
    }
    
    /// 解析单行日志
    pub fn parse_line(&self, line: &str, source: LogSourceInfo) -> Option<LogEntry> {
        if line.trim().is_empty() {
            return None;
        }
        
        // 尝试 logcat 格式
        if let Some(caps) = self.logcat_pattern.captures(line) {
            let timestamp = self.parse_logcat_time(&caps[1])?;
            let level = LogLevel::from_char(caps[2].chars().next()?)?;
            let tag = caps[3].trim().to_string();
            let pid = caps[4].parse().ok();
            let message = caps[5].to_string();
            
            return Some(LogEntry {
                timestamp,
                level,
                tag,
                pid,
                message,
                source,
                raw_line: line.to_string(),
            });
        }
        
        // 尝试括号格式
        if let Some(caps) = self.bracket_pattern.captures(line) {
            let timestamp = self.parse_bracket_time(&caps[1]).unwrap_or_else(|_| Local::now());
            let level = self.parse_level_str(&caps[2]);
            let message = caps[3].to_string();
            
            return Some(LogEntry {
                timestamp,
                level,
                tag: String::new(),
                pid: None,
                message,
                source,
                raw_line: line.to_string(),
            });
        }
        
        // 尝试 syslog 格式
        if let Some(caps) = self.syslog_pattern.captures(line) {
            let timestamp = self.parse_syslog_time(&caps[1]).unwrap_or_else(|_| Local::now());
            let tag = caps[3].to_string();
            let pid = caps.get(4).and_then(|m| m.as_str().parse().ok());
            let message = caps[5].to_string();
            
            return Some(LogEntry {
                timestamp,
                level: LogLevel::Info, // syslog 没有级别，默认 Info
                tag,
                pid,
                message,
                source,
                raw_line: line.to_string(),
            });
        }
        
        // 无法解析，检查是否包含自定义级别关键字
        let level = self.detect_custom_level(line).unwrap_or(LogLevel::Info);
        
        Some(LogEntry {
            timestamp: Local::now(),
            level,
            tag: String::new(),
            pid: None,
            message: line.to_string(),
            source,
            raw_line: line.to_string(),
        })
    }
    
    fn parse_logcat_time(&self, time_str: &str) -> Option<DateTime<Local>> {
        // 格式: 10-22 14:30:45.123
        let naive = NaiveTime::parse_from_str(time_str, "%m-%d %H:%M:%S%.3f").ok()?;
        let now = Local::now();
        let date = now.date_naive();
        let time = date.and_hms_nano_opt(
            naive.hour(),
            naive.minute(),
            naive.second(),
            naive.nanosecond()
        )?;
        Some(time.and_local_timezone(Local).single()?)
    }
    
    fn parse_bracket_time(&self, time_str: &str) -> anyhow::Result<DateTime<Local>> {
        // 尝试多种格式
        if let Ok(dt) = chrono::DateTime::parse_from_str(time_str, "%Y-%m-%d %H:%M:%S") {
            return Ok(dt.with_timezone(&Local));
        }
        if let Ok(dt) = chrono::DateTime::parse_from_str(time_str, "%Y-%m-%d %H:%M:%S%.3f") {
            return Ok(dt.with_timezone(&Local));
        }
        Ok(Local::now())
    }
    
    fn parse_syslog_time(&self, time_str: &str) -> anyhow::Result<DateTime<Local>> {
        // 格式: Oct 22 14:30:45
        let now = Local::now();
        let fmt = format!("{} {}", now.year(), time_str);
        let naive = chrono::NaiveDateTime::parse_from_str(&fmt, "%Y %b %d %H:%M:%S")?;
        Ok(naive.and_local_timezone(Local).single().unwrap_or_else(|| Local::now()))
    }
    
    fn parse_level_str(&self, level_str: &str) -> LogLevel {
        let upper = level_str.to_uppercase();
        
        // 首先检查自定义关键字
        let lower = level_str.to_lowercase();
        if let Some(level) = self.custom_keywords.get(&lower) {
            return match level.as_str() {
                "verbose" => LogLevel::Verbose,
                "debug" => LogLevel::Debug,
                "info" => LogLevel::Info,
                "warn" => LogLevel::Warn,
                "error" => LogLevel::Error,
                "fatal" => LogLevel::Fatal,
                _ => LogLevel::Info,
            };
        }
        
        // 标准级别解析
        match upper.as_str() {
            "V" | "VERBOSE" => LogLevel::Verbose,
            "D" | "DEBUG" => LogLevel::Debug,
            "I" | "INFO" => LogLevel::Info,
            "W" | "WARN" | "WARNING" => LogLevel::Warn,
            "E" | "ERROR" => LogLevel::Error,
            "F" | "FATAL" => LogLevel::Fatal,
            _ => LogLevel::Info,
        }
    }
    
    /// 检测行中是否包含自定义级别关键字
    fn detect_custom_level(&self, line: &str) -> Option<LogLevel> {
        let lower_line = line.to_lowercase();
        
        for (keyword, level) in &self.custom_keywords {
            if lower_line.contains(keyword) {
                return match level.as_str() {
                    "verbose" => Some(LogLevel::Verbose),
                    "debug" => Some(LogLevel::Debug),
                    "info" => Some(LogLevel::Info),
                    "warn" => Some(LogLevel::Warn),
                    "error" => Some(LogLevel::Error),
                    "fatal" => Some(LogLevel::Fatal),
                    _ => None,
                };
            }
        }
        None
    }
}

/// 便捷函数 - 使用默认解析器
pub fn parse_log_line(line: &str, source: LogSourceInfo) -> Option<LogEntry> {
    LogParser::new().parse_line(line, source)
}

/// 便捷函数 - 使用自定义关键字
pub fn parse_log_line_with_keywords(line: &str, source: LogSourceInfo, keywords: std::collections::HashMap<String, String>) -> Option<LogEntry> {
    LogParser::with_keywords(keywords).parse_line(line, source)
}