use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Box, ScrolledWindow, TextView, Statusbar, Label};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::log::{LogSource, LogEntry, LogLevel};
use crate::filter::Filter;

/// 主窗口状态
pub struct MainWindow {
    pub window: ApplicationWindow,
    pub log_view: TextView,
    pub log_buffer: gtk4::TextBuffer,
    pub status_bar: Statusbar,
    pub filter: Filter,
    pub log_entries: Vec<LogEntry>,
    pub filtered_entries: Vec<LogEntry>,
    pub current_source: Option<std::boxed::Box<dyn LogSource>>,
    pub is_paused: Arc<AtomicBool>,
    pub log_count: usize,
    pub filtered_count: usize,
}

impl MainWindow {
    /// 从组件创建主窗口
    pub fn from_widgets(
        window: ApplicationWindow,
        log_view: TextView,
        log_buffer: gtk4::TextBuffer,
        status_bar: Statusbar,
    ) -> Self {
        Self {
            window,
            log_view,
            log_buffer,
            status_bar,
            filter: Filter::new(),
            log_entries: Vec::new(),
            filtered_entries: Vec::new(),
            current_source: None,
            is_paused: Arc::new(AtomicBool::new(false)),
            log_count: 0,
            filtered_count: 0,
        }
    }

    /// 创建新的主窗口
    pub fn new(app: &Application) -> Self {
        let window = ApplicationWindow::builder()
            .application(app)
            .title("iLogCat")
            .default_width(1200)
            .default_height(800)
            .build();

        // 创建主布局
        let vbox = Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(6)
            .margin_top(6)
            .margin_bottom(6)
            .margin_start(6)
            .margin_end(6)
            .build();

        // 创建日志显示区域
        let scrolled = ScrolledWindow::builder()
            .hexpand(true)
            .vexpand(true)
            .build();

        let log_view = TextView::builder()
            .editable(false)
            .monospace(true)
            .wrap_mode(gtk4::WrapMode::WordChar)
            .build();

        let log_buffer = log_view.buffer();
        Self::setup_log_tags(&log_buffer);

        scrolled.set_child(Some(&log_view));
        vbox.append(&scrolled);

        // 创建状态栏
        let status_bar = Statusbar::new();
        status_bar.push(
            status_bar.context_id("main"),
            "Ready - No log source connected",
        );
        vbox.append(&status_bar);

        window.set_child(Some(&vbox));

        Self {
            window,
            log_view,
            log_buffer,
            status_bar,
            filter: Filter::new(),
            log_entries: Vec::new(),
            filtered_entries: Vec::new(),
            current_source: None,
            is_paused: Arc::new(AtomicBool::new(false)),
            log_count: 0,
            filtered_count: 0,
        }
    }

    /// 设置日志标签
    fn setup_log_tags(buffer: &gtk4::TextBuffer) {
        let tag_table = buffer.tag_table();

        // Verbose - Gray
        let tag_verbose = gtk4::TextTag::builder()
            .name("verbose")
            .foreground("#808080")
            .build();
        tag_table.add(&tag_verbose);

        // Debug - Blue
        let tag_debug = gtk4::TextTag::builder()
            .name("debug")
            .foreground("#0066CC")
            .build();
        tag_table.add(&tag_debug);

        // Info - Green
        let tag_info = gtk4::TextTag::builder()
            .name("info")
            .foreground("#008800")
            .build();
        tag_table.add(&tag_info);

        // Warn - Orange
        let tag_warn = gtk4::TextTag::builder()
            .name("warn")
            .foreground("#FF8800")
            .build();
        tag_table.add(&tag_warn);

        // Error - Red
        let tag_error = gtk4::TextTag::builder()
            .name("error")
            .foreground("#CC0000")
            .build();
        tag_table.add(&tag_error);

        // Fatal - Red Bold
        let tag_fatal = gtk4::TextTag::builder()
            .name("fatal")
            .foreground("#CC0000")
            .weight(700)
            .build();
        tag_table.add(&tag_fatal);
    }

    /// 添加日志条目
    pub fn append_log_entry(&mut self, entry: &LogEntry) {
        self.log_entries.push(entry.clone());
        self.log_count = self.log_entries.len();

        // 应用过滤
        if self.filter.matches(entry) {
            self.filtered_entries.push(entry.clone());
            self.filtered_count = self.filtered_entries.len();

            // 获取标签名称
            let tag_name = match entry.level {
                LogLevel::Verbose => "verbose",
                LogLevel::Debug => "debug",
                LogLevel::Info => "info",
                LogLevel::Warn => "warn",
                LogLevel::Error => "error",
                LogLevel::Fatal => "fatal",
            };

            // 格式化日志行
            let line = format!(
                "{} {} {}: {}\n",
                entry.timestamp.format("%H:%M:%S.%3f"),
                entry.level,
                entry.tag,
                entry.message
            );

            // 插入到文本缓冲区
            let mut end_iter = self.log_buffer.end_iter();
            self.log_buffer.insert(&mut end_iter, &line);

            // 应用颜色标签到最后一行
            let start_iter = self.log_buffer.iter_at_offset(
                self.log_buffer.char_count() - line.len() as i32
            );
            let end_iter = self.log_buffer.end_iter();
            self.log_buffer.apply_tag_by_name(tag_name, &start_iter, &end_iter);

            // 自动滚动到底部
            if !self.is_paused.load(Ordering::SeqCst) {
                self.scroll_to_end();
            }
        }

        // 限制日志数量
        if self.log_entries.len() > 100000 {
            self.log_entries.drain(0..10000);
        }

        self.update_status();
    }

    /// 清空日志
    pub fn clear_logs(&mut self) {
        self.log_buffer.set_text("");
        self.log_entries.clear();
        self.filtered_entries.clear();
        self.log_count = 0;
        self.filtered_count = 0;
        self.update_status();
    }

    /// 刷新过滤后的日志显示
    pub fn refresh_filtered_logs(&mut self, entries: &[LogEntry]) {
        // 清空当前显示
        self.log_buffer.set_text("");
        
        // 重新插入过滤后的日志
        for entry in entries {
            self.append_log_entry_internal(entry);
        }
        
        self.filtered_count = entries.len();
        self.update_status();
    }

    /// 内部方法：添加日志条目到缓冲区（不更新统计）
    fn append_log_entry_internal(&self, entry: &LogEntry) {
        // 获取标签名称
        let tag_name = match entry.level {
            LogLevel::Verbose => "verbose",
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
            LogLevel::Fatal => "fatal",
        };

        // 格式化日志行
        let line = format!(
            "{} {} {}: {}\n",
            entry.timestamp.format("%H:%M:%S.%3f"),
            entry.level,
            entry.tag,
            entry.message
        );

        // 插入到文本缓冲区
        let mut end_iter = self.log_buffer.end_iter();
        self.log_buffer.insert(&mut end_iter, &line);

        // 应用颜色标签到最后一行
        let line_start = self.log_buffer.char_count() - line.len() as i32;
        if line_start >= 0 {
            let start_iter = self.log_buffer.iter_at_offset(line_start);
            let end_iter = self.log_buffer.end_iter();
            self.log_buffer.apply_tag_by_name(tag_name, &start_iter, &end_iter);
        }
    }

    /// 滚动到末尾
    pub fn scroll_to_end(&self) {
        let mut end_iter = self.log_buffer.end_iter();
        self.log_view.scroll_to_iter(&mut end_iter, 0.0, false, 0.0, 0.0);
    }

    /// 更新状态栏
    pub fn update_status(&self) {
        let status = format!(
            "Total: {} | Filtered: {} | {}",
            self.log_count,
            self.filtered_count,
            if self.is_paused.load(Ordering::SeqCst) {
                "PAUSED"
            } else {
                "Running"
            }
        );
        self.status_bar.push(self.status_bar.context_id("main"), &status);
    }

    /// 暂停/恢复
    pub fn toggle_pause(&mut self) {
        let paused = self.is_paused.load(Ordering::SeqCst);
        self.is_paused.store(!paused, Ordering::SeqCst);
        self.update_status();
    }

    /// 显示窗口
    pub fn present(&self) {
        self.window.present();
    }
}
