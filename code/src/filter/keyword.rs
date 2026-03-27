use regex::Regex;

/// 关键字过滤器
#[derive(Debug, Clone)]
pub struct KeywordFilter {
    pub text: String,
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub is_regex: bool,
    regex: Option<Regex>,
}

impl KeywordFilter {
    pub fn new(text: String, case_sensitive: bool, whole_word: bool) -> Self {
        Self::with_regex(text, case_sensitive, whole_word, false)
    }

    /// 创建支持正则表达式的过滤器
    pub fn with_regex(text: String, case_sensitive: bool, whole_word: bool, is_regex: bool) -> Self {
        let regex = if is_regex {
            // 正则表达式模式：自动去除 | 两边的空格
            // 例如 "errorCode | Early" -> "errorCode|Early"
            let cleaned_text = text.replace(" | ", "|").replace(" |", "|").replace("| ", "|");
            if case_sensitive {
                Regex::new(&cleaned_text).ok()
            } else {
                Regex::new(&format!("(?i){}", cleaned_text)).ok()
            }
        } else if whole_word {
            let pattern = format!(r"\b{}\b", regex::escape(&text));
            Regex::new(&pattern).ok()
        } else if case_sensitive {
            Regex::new(&regex::escape(&text)).ok()
        } else {
            Regex::new(&format!("(?i){}", regex::escape(&text))).ok()
        };

        Self {
            text,
            case_sensitive,
            whole_word,
            is_regex,
            regex,
        }
    }

    pub fn matches(&self, text: &str) -> bool {
        if let Some(ref regex) = self.regex {
            regex.is_match(text)
        } else if self.case_sensitive {
            text.contains(&self.text)
        } else {
            text.to_lowercase().contains(&self.text.to_lowercase())
        }
    }

    /// 测试用：获取编译的正则表达式字符串
    #[cfg(test)]
    fn get_regex_pattern(&self) -> Option<String> {
        self.regex.as_ref().map(|r| r.to_string())
    }

    /// 获取匹配的所有位置（用于高亮）
    pub fn find_matches<'a>(&self, text: &'a str) -> Vec<(usize, usize)> {
        if let Some(ref regex) = self.regex {
            regex.find_iter(text).map(|m| (m.start(), m.end())).collect()
        } else {
            // 非正则模式，查找所有匹配位置
            let mut matches = Vec::new();
            let search_text = if self.case_sensitive {
                text.to_string()
            } else {
                text.to_lowercase()
            };
            let pattern = self.text.to_lowercase();

            let mut start = 0;
            while let Some(pos) = search_text[start..].find(&pattern) {
                let abs_pos = start + pos;
                matches.push((abs_pos, abs_pos + self.text.len()));
                start = abs_pos + 1;
            }
            matches
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regex_with_pipe_spaces() {
        // 测试 "socProtocol | battery" 的处理
        let filter = KeywordFilter::with_regex(
            "socProtocol | battery".to_string(),
            false, // case_sensitive
            false, // whole_word
            true,  // is_regex
        );

        // 验证正则表达式能匹配
        assert!(filter.matches("socProtocol test"), "Should match socProtocol");
        assert!(filter.matches("battery level"), "Should match battery");
        assert!(filter.matches("SOCPROTOCOL"), "Should match SOCPROTOCOL (case insensitive)");
        assert!(filter.matches("BATTERY"), "Should match BATTERY (case insensitive)");
        assert!(!filter.matches("other text"), "Should not match other text");
    }

    #[test]
    fn test_regex_pipe_cleaning() {
        let input = "errorCode | Early";
        let cleaned = input.replace(" | ", "|").replace(" |", "|").replace("| ", "|");
        assert_eq!(cleaned, "errorCode|Early");

        let input2 = "a | b | c";
        let cleaned2 = input2.replace(" | ", "|").replace(" |", "|").replace("| ", "|");
        assert_eq!(cleaned2, "a|b|c");
    }
}