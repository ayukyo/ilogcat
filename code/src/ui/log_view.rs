use gtk4::prelude::*;
use gtk4::{TextView, TextBuffer, ScrolledWindow, TextTagTable, TextIter};
use std::collections::HashMap;

use crate::log::{LogEntry, LogLevel};

/// 日志显示选项
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LogDisplayOptions {
    pub show_line_number: bool,
    pub show_timestamp: bool,
    pub show_level: bool,
    pub show_tag: bool,
    pub show_pid: bool,
    pub compact_mode: bool,
}

impl Default for LogDisplayOptions {
    fn default() -> Self {
        Self {
            show_line_number: true,
            show_timestamp: true,
            show_level: true,
            show_tag: true,
            show_pid: false,
            compact_mode: false,
        }
    }
}

/// 日志视图组件
pub struct LogView {
    pub text_view: TextView,
    pub buffer: TextBuffer,
    pub tags: HashMap<LogLevel, String>,
    max_lines: usize,
    line_count: usize,
    display_options: LogDisplayOptions,
}

impl LogView {
    /// 创建新的日志视图
    pub fn new() -> Self {
        let buffer = TextBuffer::new(None);
        let text_view = TextView::builder()
            .buffer(&buffer)
            .editable(false)
            .monospace(true)
            .wrap_mode(gtk4::WrapMode::WordChar)
            .build();

        let mut view = Self {
            text_view,
            buffer,
            tags: HashMap::new(),
            max_lines: 100000,
            line_count: 0,
            display_options: LogDisplayOptions::default(),
        };

        view.setup_tags();
        view
    }

    /// 设置显示选项
    pub fn set_display_options(&mut self, options: LogDisplayOptions) {
        self.display_options = options;
    }

    /// 获取当前显示选项
    pub fn display_options(&self) -> LogDisplayOptions {
        self.display_options
    }

    /// 切换行号显示
    pub fn toggle_line_numbers(&mut self) {
        self.display_options.show_line_number = !self.display_options.show_line_number;
    }

    /// 切换紧凑模式
    pub fn toggle_compact_mode(&mut self) {
        self.display_options.compact_mode = !self.display_options.compact_mode;
    }

    /// 设置文本标签
    fn setup_tags(&mut self) {
        let tag_table = self.buffer.tag_table();

        // Verbose - Gray
        let tag = gtk4::TextTag::builder()
            .name("verbose")
            .foreground("#808080")
            .build();
        tag_table.add(&tag);
        self.tags.insert(LogLevel::Verbose, "verbose".to_string());

        // Debug - Blue
        let tag = gtk4::TextTag::builder()
            .name("debug")
            .foreground("#0066CC")
            .build();
        tag_table.add(&tag);
        self.tags.insert(LogLevel::Debug, "debug".to_string());

        // Info - Green
        let tag = gtk4::TextTag::builder()
            .name("info")
            .foreground("#008800")
            .build();
        tag_table.add(&tag);
        self.tags.insert(LogLevel::Info, "info".to_string());

        // Warn - Orange
        let tag = gtk4::TextTag::builder()
            .name("warn")
            .foreground("#FF8800")
            .build();
        tag_table.add(&tag);
        self.tags.insert(LogLevel::Warn, "warn".to_string());

        // Error - Red
        let tag = gtk4::TextTag::builder()
            .name("error")
            .foreground("#CC0000")
            .build();
        tag_table.add(&tag);
        self.tags.insert(LogLevel::Error, "error".to_string());

        // Fatal - Red Bold
        let tag = gtk4::TextTag::builder()
            .name("fatal")
            .foreground("#CC0000")
            .weight(700)
            .build();
        tag_table.add(&tag);
        self.tags.insert(LogLevel::Fatal, "fatal".to_string());

        // Line number - Gray muted
        let tag = gtk4::TextTag::builder()
            .name("linenumber")
            .foreground("#666666")
            .build();
        tag_table.add(&tag);
    }

    /// 添加日志条目
    pub fn append_entry(&mut self, entry: &LogEntry) {
        let tag_name = self.tags.get(&entry.level)
            .map(|s| s.as_str())
            .unwrap_or("info");

        self.line_count += 1;
        let line = self.format_log_entry(entry);

        let mut end_iter = self.buffer.end_iter();
        let start_offset = self.buffer.char_count();
        self.buffer.insert(&mut end_iter, &line);
        self.buffer.insert(&mut end_iter, "\n");

        // 应用标签到整行
        let start_iter = self.buffer.iter_at_offset(start_offset);
        let end_iter = self.buffer.end_iter();
        self.buffer.apply_tag_by_name(tag_name, &start_iter, &end_iter);

        // 限制行数
        self.trim_if_needed();
    }

    /// 格式化日志条目
    fn format_log_entry(&self, entry: &LogEntry) -> String {
        let opts = &self.display_options;
        
        if opts.compact_mode {
            // 紧凑模式：只显示关键信息
            let pid_str = entry.pid.map(|p| format!("[{}]", p)).unwrap_or_default();
            format!("{} {} {}{}", 
                entry.timestamp.format("%H:%M:%S"),
                entry.level,
                pid_str,
                entry.message
            )
        } else {
            // 完整模式
            let mut parts = Vec::new();
            
            // 行号
            if opts.show_line_number {
                parts.push(format!("{:>6}", self.line_count));
            }
            
            // 时间戳
            if opts.show_timestamp {
                parts.push(entry.timestamp.format("%H:%M:%S.%3f").to_string());
            }
            
            // 日志级别
            if opts.show_level {
                parts.push(format!("{}", entry.level));
            }
            
            // 标签
            if opts.show_tag && !entry.tag.is_empty() {
                parts.push(entry.tag.clone());
            }
            
            // PID
            if opts.show_pid && entry.pid.is_some() {
                parts.push(format!("{}", entry.pid.unwrap()));
            }
            
            // 消息
            parts.push(entry.message.clone());
            
            parts.join(" ")
        }
    }

    /// 批量添加日志条目
    pub fn append_entries(&mut self, entries: &[LogEntry]) {
        for entry in entries {
            self.append_entry(entry);
        }
    }

    /// 清空日志
    pub fn clear(&mut self) {
        self.buffer.set_text("");
        self.line_count = 0;
    }

    /// 滚动到底部
    pub fn scroll_to_end(&self) {
        let mut end_iter = self.buffer.end_iter();
        self.text_view.scroll_to_iter(&mut end_iter, 0.0, false, 0.0, 0.0);
    }

    /// 设置最大行数
    pub fn set_max_lines(&mut self, max_lines: usize) {
        self.max_lines = max_lines;
    }

    /// 修剪旧日志
    fn trim_if_needed(&mut self) {
        let line_count = self.buffer.line_count();
        if line_count > self.max_lines as i32 {
            let lines_to_remove = line_count - self.max_lines as i32;
            let mut start_iter = self.buffer.start_iter();
            let mut end_iter = self.buffer.iter_at_line(lines_to_remove).unwrap_or_else(|| self.buffer.end_iter());
            self.buffer.delete(&mut start_iter, &mut end_iter);
        }
    }

    /// 获取文本视图
    pub fn widget(&self) -> &TextView {
        &self.text_view
    }

    /// 获取文本缓冲区
    pub fn buffer(&self) -> &TextBuffer {
        &self.buffer
    }
}

impl Default for LogView {
    fn default() -> Self {
        Self::new()
    }
}
