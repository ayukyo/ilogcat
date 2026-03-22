use gtk4::prelude::*;
use gtk4::{TextView, TextBuffer, ScrolledWindow, TextTagTable, TextIter};
use std::collections::HashMap;

use crate::log::{LogEntry, LogLevel};

/// 日志视图组件
pub struct LogView {
    pub text_view: TextView,
    pub buffer: TextBuffer,
    pub tags: HashMap<LogLevel, String>,
    max_lines: usize,
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
        };

        view.setup_tags();
        view
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
    }

    /// 添加日志条目
    pub fn append_entry(&mut self, entry: &LogEntry) {
        let tag_name = self.tags.get(&entry.level)
            .map(|s| s.as_str())
            .unwrap_or("info");

        let line = format!(
            "{} {} {}: {}\n",
            entry.timestamp.format("%H:%M:%S.%3f"),
            entry.level,
            entry.tag,
            entry.message
        );

        let end_iter = self.buffer.end_iter();
        let start_offset = self.buffer.char_count();
        self.buffer.insert(&end_iter, &line);

        // 应用标签
        let start_iter = self.buffer.iter_at_offset(start_offset);
        let end_iter = self.buffer.end_iter();
        self.buffer.apply_tag_by_name(tag_name, &start_iter, &end_iter);

        // 限制行数
        self.trim_if_needed();
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
    }

    /// 滚动到底部
    pub fn scroll_to_end(&self) {
        let end_iter = self.buffer.end_iter();
        self.text_view.scroll_to_iter(&end_iter, 0.0, false, 0.0, 0.0);
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
