pub mod keyword;
pub mod level;
pub mod regex_filter;

pub use keyword::KeywordFilter;
pub use level::LevelFilter;
pub use regex_filter::RegexFilter;

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
            LogLevel::Verbose => vec!["V", "D", "I", "W", "E", "F"],
            LogLevel::Debug => vec!["D", "I", "W", "E", "F"],
            LogLevel::Info => vec!["I", "W", "E", "F"],
            LogLevel::Warn => vec!["W", "E", "F"],
            LogLevel::Error => vec!["E", "F"],
            LogLevel::Fatal => vec!["F"],
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
        // 级别过滤
        if !self.levels.is_empty() && !self.levels.contains(&entry.level.to_string()) {
            return false;
        }

        // 关键字过滤
        if !self.keywords.is_empty() {
            let matches = self.keywords.iter().any(|kw| kw.matches(&entry.message));
            if !matches {
                return false;
            }
        }

        // 正则过滤
        if let Some(ref regex) = self.regex {
            if !regex.matches(&entry.message) {
                return false;
            }
        }

        true
    }
}