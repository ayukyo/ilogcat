use regex::Regex;
use chrono::{DateTime, Local, Timelike, Datelike};
use std::collections::HashMap;
use crate::log::{LogEntry, LogLevel, LogSourceInfo};

/// 日志行解析器
pub struct LogParser {
    /// Android logcat 格式
    /// 例如: 10-22 14:30:45.123 D/MyApp( 1234): This is a debug message
    logcat_pattern: Regex,

    /// 括号提取模式 - 提取所有 [field] 格式
    bracket_extract_pattern: Regex,

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

            // 匹配所有 [field] 格式
            bracket_extract_pattern: Regex::new(
                r"\[([^\]]+)\]"
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

        // 尝试灵活括号格式解析
        if line.contains('[') {
            return self.parse_bracket_format(line, source);
        }

        // 尝试 syslog 格式
        if let Some(caps) = self.syslog_pattern.captures(line) {
            let timestamp = self.parse_syslog_time(&caps[1]).unwrap_or_else(|| Local::now());
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

    /// 灵活解析括号格式
    /// 支持 [field1] [field2] [field3] ... message 格式
    /// 也支持 timestamp: [field1] [field2] ... 格式
    fn parse_bracket_format(&self, line: &str, source: LogSourceInfo) -> Option<LogEntry> {
        // 检查括号前是否有时间戳（格式：2024-10-22 14:30:45.123: [...] )
        let bracket_start = line.find('[')?;
        let prefix = &line[..bracket_start];
        let prefix_timestamp = if !prefix.trim().is_empty() {
            // 移除尾部冒号和空格
            let prefix_clean = prefix.trim().trim_end_matches(':').trim();
            self.parse_bracket_time(prefix_clean)
        } else {
            None
        };

        // 提取所有括号字段
        let fields: Vec<String> = self.bracket_extract_pattern
            .captures_iter(line)
            .map(|caps| caps[1].to_string())
            .collect();

        if fields.is_empty() {
            return None;
        }

        // 找到消息部分（最后一个 ] 之后的内容）
        let last_bracket_end = line.rfind(']')? + 1;
        let trailing_message = line[last_bracket_end..].trim().to_string();

        // 分析每个字段
        let mut timestamp: Option<DateTime<Local>> = prefix_timestamp;
        let mut level: Option<LogLevel> = None;
        let mut tags: Vec<String> = Vec::new();
        let mut skip_indices: Vec<usize> = Vec::new(); // 需要跳过的字段索引

        for (i, field) in fields.iter().enumerate() {
            // 检查是否是时间字段
            let is_time_field = self.parse_bracket_time(field).is_some();

            if is_time_field {
                // 如果还没有时间戳，使用这个时间
                if timestamp.is_none() {
                    timestamp = self.parse_bracket_time(field);
                }
                // 无论是否使用，时间字段都要跳过（不在消息中显示）
                skip_indices.push(i);
                continue;
            }

            // 尝试解析为级别
            if level.is_none() {
                let parsed_level = self.parse_level_str(field);
                // 检查是否真正是级别（不是默认的 Info）
                let is_level = !matches!(parsed_level, LogLevel::Info) ||
                    field.to_lowercase() == "info" ||
                    field.to_lowercase() == "warning" ||
                    field.to_lowercase() == "warn" ||
                    field.to_lowercase() == "error" ||
                    field.to_lowercase() == "debug" ||
                    field.to_lowercase() == "trace" ||
                    field.to_lowercase() == "verbose" ||
                    field.to_lowercase() == "fatal" ||
                    field.to_lowercase() == "critical";

                if is_level && !matches!(parsed_level, LogLevel::Info) || field.to_lowercase() == "info" {
                    level = Some(parsed_level);
                    continue;
                }
            }

            // 否则作为标签
            tags.push(field.clone());
        }

        // 如果消息中还有级别关键字，也要检查
        let final_level = level.or_else(|| self.detect_custom_level(&trailing_message))
            .or_else(|| self.detect_custom_level(line))
            .unwrap_or(LogLevel::Info);

        // 构建显示的消息：跳过时间字段，保留其他括号字段 + 消息尾部
        let bracket_part: String = fields.iter()
            .enumerate()
            .filter(|(i, _)| !skip_indices.contains(i))
            .map(|(_, f)| format!("[{}]", f))
            .collect::<Vec<_>>()
            .join(" ");

        let display_message = if bracket_part.is_empty() {
            trailing_message.clone()
        } else if trailing_message.is_empty() {
            bracket_part
        } else {
            format!("{} {}", bracket_part, trailing_message)
        };

        // 标签只取第一个非时间字段（通常是模块名或标签）
        let display_tag = tags.first().cloned().unwrap_or_default();

        Some(LogEntry {
            timestamp: timestamp.unwrap_or_else(|| Local::now()),
            level: final_level,
            tag: display_tag,
            pid: None,
            message: display_message,
            source,
            raw_line: line.to_string(),
        })
    }

    fn parse_logcat_time(&self, time_str: &str) -> Option<DateTime<Local>> {
        // 格式: 10-22 14:30:45.123 (月-日 时:分:秒.毫秒)
        // 解析完整的日期时间
        let naive = chrono::NaiveDateTime::parse_from_str(time_str, "%m-%d %H:%M:%S%.3f").ok()?;

        // 获取当前年份
        let year = Local::now().year();
        let date_with_year = naive.date().with_year(year)?;
        let time_with_year = date_with_year.and_time(naive.time());

        // 直接将时间作为本地时间处理，显示日志中的原始时间
        time_with_year.and_local_timezone(Local).single()
    }

    fn parse_bracket_time(&self, time_str: &str) -> Option<DateTime<Local>> {
        // 尝试多种格式
        // 格式1: 2024-10-22 14:30:45
        if let Ok(dt) = chrono::DateTime::parse_from_str(time_str, "%Y-%m-%d %H:%M:%S") {
            // 直接转换为本地时间，保持原始时间值
            return Some(dt.with_timezone(&Local));
        }
        // 格式2: 2024-10-22 14:30:45.123
        if let Ok(dt) = chrono::DateTime::parse_from_str(time_str, "%Y-%m-%d %H:%M:%S%.3f") {
            return Some(dt.with_timezone(&Local));
        }
        None
    }

    fn parse_syslog_time(&self, time_str: &str) -> Option<DateTime<Local>> {
        // 格式: Oct 22 14:30:45
        let now = Local::now();
        let year = now.year();
        let fmt = format!("{} {}", year, time_str);
        let naive = chrono::NaiveDateTime::parse_from_str(&fmt, "%Y %b %d %H:%M:%S").ok()?;
        // 直接使用原始时间值作为本地时间
        naive.and_local_timezone(Local).single()
    }
    
    fn parse_level_str(&self, level_str: &str) -> LogLevel {
        let upper = level_str.to_uppercase();

        // 首先检查自定义关键字
        let lower = level_str.to_lowercase();
        if let Some(level) = self.custom_keywords.get(&lower) {
            return match level.as_str() {
                "trace" => LogLevel::Trace,
                "verbose" => LogLevel::Verbose,
                "debug" => LogLevel::Debug,
                "info" => LogLevel::Info,
                "warn" => LogLevel::Warn,
                "error" => LogLevel::Error,
                "fatal" => LogLevel::Fatal,
                "critical" => LogLevel::Critical,
                _ => LogLevel::Info,
            };
        }

        // 标准级别解析
        match upper.as_str() {
            "T" | "TRACE" => LogLevel::Trace,
            "V" | "VERBOSE" => LogLevel::Verbose,
            "D" | "DEBUG" => LogLevel::Debug,
            "I" | "INFO" => LogLevel::Info,
            "W" | "WARN" | "WARNING" => LogLevel::Warn,
            "E" | "ERROR" => LogLevel::Error,
            "F" | "FATAL" => LogLevel::Fatal,
            "C" | "CRITICAL" => LogLevel::Critical,
            _ => LogLevel::Info,
        }
    }

    /// 检测行中是否包含自定义级别关键字
    fn detect_custom_level(&self, line: &str) -> Option<LogLevel> {
        let lower_line = line.to_lowercase();

        // 首先检查自定义关键字
        for (keyword, level) in &self.custom_keywords {
            if lower_line.contains(keyword) {
                return match level.as_str() {
                    "trace" => Some(LogLevel::Trace),
                    "verbose" => Some(LogLevel::Verbose),
                    "debug" => Some(LogLevel::Debug),
                    "info" => Some(LogLevel::Info),
                    "warn" => Some(LogLevel::Warn),
                    "error" => Some(LogLevel::Error),
                    "fatal" => Some(LogLevel::Fatal),
                    "critical" => Some(LogLevel::Critical),
                    _ => None,
                };
            }
        }

        // 默认关键字检测（支持 [warning], [error], [info] 等格式）
        if lower_line.contains("[trace]") || lower_line.contains("[t]") {
            return Some(LogLevel::Trace);
        }
        if lower_line.contains("[verbose]") || lower_line.contains("[v]") {
            return Some(LogLevel::Verbose);
        }
        if lower_line.contains("[debug]") || lower_line.contains("[d]") {
            return Some(LogLevel::Debug);
        }
        if lower_line.contains("[info]") || lower_line.contains("[i]") {
            return Some(LogLevel::Info);
        }
        if lower_line.contains("[warn]") || lower_line.contains("[w]") || lower_line.contains("[warning]") {
            return Some(LogLevel::Warn);
        }
        if lower_line.contains("[error]") || lower_line.contains("[e]") {
            return Some(LogLevel::Error);
        }
        if lower_line.contains("[fatal]") || lower_line.contains("[f]") {
            return Some(LogLevel::Fatal);
        }
        if lower_line.contains("[critical]") || lower_line.contains("[c]") {
            return Some(LogLevel::Critical);
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