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