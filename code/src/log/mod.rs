pub mod source;
pub mod local;
pub mod parser;

pub use source::{LogSource, LogEntry, LogLevel, LogSourceInfo};
pub use parser::parse_log_line;