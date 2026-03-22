use regex::Regex;

/// 正则表达式过滤器
pub struct RegexFilter {
    pub pattern: String,
    pub case_sensitive: bool,
    regex: Regex,
}

impl RegexFilter {
    pub fn new(pattern: String, case_sensitive: bool) -> anyhow::Result<Self> {
        let regex = if case_sensitive {
            Regex::new(&pattern)?
        } else {
            Regex::new(&format!("(?i){}", pattern))?
        };
        
        Ok(Self {
            pattern,
            case_sensitive,
            regex,
        })
    }
    
    pub fn matches(&self, text: &str) -> bool {
        self.regex.is_match(text)
    }
}