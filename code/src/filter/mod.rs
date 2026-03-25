pub mod keyword;
pub mod level;
pub mod regex_filter;
pub mod enhanced;

pub use keyword::KeywordFilter;
pub use level::LevelFilter;
pub use regex_filter::RegexFilter;
pub use enhanced::{EnhancedRegexFilter, FilterPattern};

use crate::log::LogEntry;
use std::collections::HashSet;

/// 组合过滤器
pub struct Filter {
    pub keywords: Vec<KeywordFilter>,
    pub levels: HashSet<String>,
    pub regex: Option<RegexFilter>,
}

impl Filter {
    pub fn new() -> Self {
        Self {
            keywords: Vec::new(),
            levels: HashSet::new(),
            regex: None,
        }
    }

    /// 设置最小日志级别（显示该级别及以上的日志）
    pub fn set_min_level(&mut self, min_level: crate::log::LogLevel) {
        use crate::log::LogLevel;
        self.levels.clear();

        let levels_to_include = match min_level {
            LogLevel::Trace => vec!["T", "V", "D", "I", "W", "E", "F", "C"],
            LogLevel::Verbose => vec!["V", "D", "I", "W", "E", "F", "C"],
            LogLevel::Debug => vec!["D", "I", "W", "E", "F", "C"],
            LogLevel::Info => vec!["I", "W", "E", "F", "C"],
            LogLevel::Warn => vec!["W", "E", "F", "C"],
            LogLevel::Error => vec!["E", "F", "C"],
            LogLevel::Fatal => vec!["F", "C"],
            LogLevel::Critical => vec!["C"],
        };

        for level in levels_to_include {
            self.levels.insert(level.to_string());
        }
    }

    /// 清除级别过滤
    pub fn clear_level_filter(&mut self) {
        self.levels.clear();
    }

    /// 检查日志条目是否通过过滤
    pub fn matches(&self, entry: &LogEntry) -> bool {
        // 级别过滤 - 使用单字符表示（V, D, I, W, E, F）
        if !self.levels.is_empty() {
            let level_str = entry.level.to_string(); // 返回单字符如 "V", "D", "I"
            if !self.levels.contains(&level_str) {
                return false;
            }
        }

        // 关键字过滤 - 搜索整个日志内容（时间戳 + 级别 + 标签 + 消息）
        if !self.keywords.is_empty() {
            let full_text = format!(
                "{} {} {}: {}",
                entry.timestamp.format("%H:%M:%S.%3f"),
                entry.level,
                entry.tag,
                entry.message
            );
            let matches = self.keywords.iter().any(|kw| kw.matches(&full_text));
            if !matches {
                return false;
            }
        }

        // 正则过滤 - 同样搜索整个日志内容
        if let Some(ref regex) = self.regex {
            let full_text = format!(
                "{} {} {}: {}",
                entry.timestamp.format("%H:%M:%S.%3f"),
                entry.level,
                entry.tag,
                entry.message
            );
            if !regex.matches(&full_text) {
                return false;
            }
        }

        true
    }
}