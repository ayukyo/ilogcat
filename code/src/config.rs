// Config module - placeholder
// Will be implemented in future phases

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub recent_sources: Vec<String>,
    pub filter_history: Vec<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            recent_sources: Vec::new(),
            filter_history: Vec::new(),
        }
    }
}