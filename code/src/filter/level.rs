use std::collections::HashSet;

/// 日志级别过滤器
pub struct LevelFilter {
    pub enabled_levels: HashSet<String>,
}

impl LevelFilter {
    pub fn new() -> Self {
        // 默认启用所有级别
        let enabled_levels = vec![
            "VERBOSE".to_string(),
            "DEBUG".to_string(),
            "INFO".to_string(),
            "WARN".to_string(),
            "ERROR".to_string(),
            "FATAL".to_string(),
        ].into_iter().collect();
        
        Self { enabled_levels }
    }
    
    pub fn enable(&mut self, level: &str) {
        self.enabled_levels.insert(level.to_uppercase());
    }
    
    pub fn disable(&mut self, level: &str) {
        self.enabled_levels.remove(&level.to_uppercase());
    }
    
    pub fn toggle(&mut self, level: &str) {
        let level = level.to_uppercase();
        if self.enabled_levels.contains(&level) {
            self.enabled_levels.remove(&level);
        } else {
            self.enabled_levels.insert(level);
        }
    }
    
    pub fn is_enabled(&self, level: &str) -> bool {
        self.enabled_levels.contains(&level.to_uppercase())
    }
}