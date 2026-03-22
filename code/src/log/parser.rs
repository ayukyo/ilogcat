use regex::Regex;
use chrono::{DateTime, Local, NaiveTime, Timelike, Datelike};
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
}

impl LogParser {
    pub fn new() -> Self {
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
        }
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
        
        // 无法解析，作为原始行处理
        Some(LogEntry {
            timestamp: Local::now(),
            level: LogLevel::Info,
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
        Some(
            now.date_naive()
                .and_hms_opt(naive.hour(), naive.minute(), naive.second())?
                .and_nano(naive.nanosecond())?
                .and_local_timezone(Local)
                .single()?
        )
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
        match level_str.to_uppercase().as_str() {
            "V" | "VERBOSE" => LogLevel::Verbose,
            "D" | "DEBUG" => LogLevel::Debug,
            "I" | "INFO" => LogLevel::Info,
            "W" | "WARN" | "WARNING" => LogLevel::Warn,
            "E" | "ERROR" => LogLevel::Error,
            "F" | "FATAL" => LogLevel::Fatal,
            _ => LogLevel::Info,
        }
    }
}

/// 便捷函数
pub fn parse_log_line(line: &str, source: LogSourceInfo) -> Option<LogEntry> {
    LogParser::new().parse_line(line, source)
}