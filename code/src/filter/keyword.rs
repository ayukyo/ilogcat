use regex::Regex;

/// 关键字过滤器
#[derive(Debug, Clone)]
pub struct KeywordFilter {
    pub text: String,
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub regex: Option<Regex>,
}

impl KeywordFilter {
    pub fn new(text: String, case_sensitive: bool, whole_word: bool) -> Self {
        let regex = if whole_word {
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
}