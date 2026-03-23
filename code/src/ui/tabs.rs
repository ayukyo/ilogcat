use gtk4::prelude::*;
use gtk4::{Notebook, Label, Orientation, Button, ScrolledWindow, TextView, TextBuffer};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::collections::HashMap;

use crate::log::{LogSource, LogEntry, LogLevel};
use crate::filter::Filter;

/// 单个日志窗口标签页的状态
pub struct LogTab {
    pub id: usize,
    pub name: String,
    pub source_name: String,  // 日志来源名称（用于标签页标题）
    pub source_type: SourceType,  // 日志源类型
    pub log_entries: Vec<LogEntry>,
    pub filtered_entries: Vec<LogEntry>,
    pub filter: Filter,
    pub current_source: Option<std::boxed::Box<dyn LogSource>>,
    pub is_paused: Arc<AtomicBool>,
    pub log_count: usize,
    pub filtered_count: usize,
    pub text_buffer: TextBuffer,
    pub text_view: TextView,
    pub tab_label_widget: Option<gtk4::Label>,  // 保存标签页标题文本 widget 引用
    // 统计信息
    level_counts: HashMap<LogLevel, usize>,
    start_time: std::time::Instant,
}

/// 日志源类型
#[derive(Clone, Debug)]
pub enum SourceType {
    Dmesg,
    Journalctl,
    File(String),  // 文件路径
    Ssh(String, String),  // (主机名, 命令)
    SshCommand(String, String),  // (主机名, 自定义命令)
    Unknown,
}

impl LogTab {
    pub fn new(id: usize, name: String) -> Self {
        let text_view = TextView::builder()
            .editable(false)
            .monospace(true)
            .wrap_mode(gtk4::WrapMode::WordChar)
            .build();

        let text_buffer = text_view.buffer();
        Self::setup_log_tags(&text_buffer);

        Self {
            id,
            name: name.clone(),
            source_name: name,
            source_type: SourceType::Unknown,
            log_entries: Vec::new(),
            filtered_entries: Vec::new(),
            filter: Filter::new(),
            current_source: None,
            is_paused: Arc::new(AtomicBool::new(false)),
            log_count: 0,
            filtered_count: 0,
            text_buffer,
            text_view,
            tab_label_widget: None,
            level_counts: HashMap::new(),
            start_time: std::time::Instant::now(),
        }
    }

    /// 获取统计信息
    pub fn get_statistics(&self) -> crate::stats::LogStatistics {
        let total = self.log_entries.len();
        let filtered = self.filtered_entries.len();
        let elapsed = self.start_time.elapsed().as_secs_f64();
        let rate = if elapsed > 0.0 { total as f64 / elapsed } else { 0.0 };
        
        crate::stats::LogStatistics {
            level_counts: self.level_counts.clone(),
            total_count: total,
            filtered_count: filtered,
            logs_per_second: rate,
            start_time: self.start_time,
            last_update: std::time::Instant::now(),
        }
    }

    /// 设置日志源信息并更新标签页标题
    pub fn set_source_info(&mut self, source_type: SourceType) {
        self.source_name = match &source_type {
            SourceType::Dmesg => "dmesg".to_string(),
            SourceType::Journalctl => "journalctl".to_string(),
            SourceType::File(path) => {
                std::path::Path::new(path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(path)
                    .to_string()
            }
            SourceType::Ssh(host, _) => format!("📡 {}", host),
            SourceType::SshCommand(host, cmd) => {
                let short_cmd = if cmd.len() > 20 {
                    format!("{}...", &cmd[..20])
                } else {
                    cmd.clone()
                };
                format!("📡 {}: {}", host, short_cmd)
            }
            SourceType::Unknown => self.name.clone(),
        };
        self.source_type = source_type;
    }

    /// 获取连接状态图标
    pub fn get_connection_status_icon(&self) -> &'static str {
        match &self.source_type {
            SourceType::Ssh(_, _) | SourceType::SshCommand(_, _) => {
                if self.is_source_running() {
                    "🟢"  // 连接中
                } else {
                    "🔴"  // 断开
                }
            }
            _ => "",
        }
    }

    /// 获取完整的标签页标题（包含状态图标）
    pub fn get_tab_title(&self) -> String {
        let status_icon = self.get_connection_status_icon();
        if status_icon.is_empty() {
            self.source_name.clone()
        } else {
            format!("{} {}", status_icon, self.source_name)
        }
    }

    /// 检查日志源是否正在运行
    fn is_source_running(&self) -> bool {
        self.current_source.as_ref()
            .map(|s| s.is_running())
            .unwrap_or(false)
    }

    /// 设置日志标签
    fn setup_log_tags(buffer: &TextBuffer) {
        let tag_table = buffer.tag_table();

        let tag_verbose = gtk4::TextTag::builder()
            .name("verbose")
            .foreground("#808080")
            .build();
        tag_table.add(&tag_verbose);

        let tag_debug = gtk4::TextTag::builder()
            .name("debug")
            .foreground("#0066CC")
            .build();
        tag_table.add(&tag_debug);

        let tag_info = gtk4::TextTag::builder()
            .name("info")
            .foreground("#008800")
            .build();
        tag_table.add(&tag_info);

        let tag_warn = gtk4::TextTag::builder()
            .name("warn")
            .foreground("#FF8800")
            .build();
        tag_table.add(&tag_warn);

        let tag_error = gtk4::TextTag::builder()
            .name("error")
            .foreground("#CC0000")
            .build();
        tag_table.add(&tag_error);

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
        
        // 更新级别统计
        *self.level_counts.entry(entry.level).or_insert(0) += 1;

        if self.filter.matches(entry) {
            self.filtered_entries.push(entry.clone());
            self.filtered_count = self.filtered_entries.len();
            self.append_to_buffer(entry);
        }

        // 限制日志数量
        if self.log_entries.len() > 100000 {
            self.log_entries.drain(0..10000);
        }
    }

    /// 添加日志到缓冲区
    fn append_to_buffer(&self, entry: &LogEntry) {
        let tag_name = match entry.level {
            LogLevel::Verbose => "verbose",
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
            LogLevel::Fatal => "fatal",
        };

        let line = format!(
            "{} {} {}: {}\n",
            entry.timestamp.format("%H:%M:%S.%3f"),
            entry.level,
            entry.tag,
            entry.message
        );

        let mut end_iter = self.text_buffer.end_iter();
        self.text_buffer.insert(&mut end_iter, &line);

        let line_start = self.text_buffer.char_count() - line.len() as i32;
        if line_start >= 0 {
            let start_iter = self.text_buffer.iter_at_offset(line_start);
            let end_iter = self.text_buffer.end_iter();
            self.text_buffer.apply_tag_by_name(tag_name, &start_iter, &end_iter);
        }

        // 自动滚动到底部
        if !self.is_paused.load(Ordering::SeqCst) {
            self.scroll_to_end();
        }
    }

    /// 滚动到末尾
    fn scroll_to_end(&self) {
        let mut end_iter = self.text_buffer.end_iter();
        self.text_view.scroll_to_iter(&mut end_iter, 0.0, false, 0.0, 0.0);
    }

    /// 清空日志
    pub fn clear_logs(&mut self) {
        self.text_buffer.set_text("");
        self.log_entries.clear();
        self.filtered_entries.clear();
        self.log_count = 0;
        self.filtered_count = 0;
        self.level_counts.clear();
        self.start_time = std::time::Instant::now();
    }

    /// 暂停/恢复
    pub fn toggle_pause(&mut self) {
        let paused = self.is_paused.load(Ordering::SeqCst);
        self.is_paused.store(!paused, Ordering::SeqCst);
    }

    /// 是否暂停
    pub fn is_paused(&self) -> bool {
        self.is_paused.load(Ordering::SeqCst)
    }

    /// 停止当前日志源
    pub fn stop_source(&mut self) {
        if let Some(ref mut source) = self.current_source {
            let _ = source.stop();
        }
        self.current_source = None;
    }

    /// 设置日志源
    pub fn set_source(&mut self, source: std::boxed::Box<dyn LogSource>) {
        self.stop_source();
        self.current_source = Some(source);
    }

    /// 刷新过滤显示
    pub fn refresh_filtered_display(&mut self) {
        self.text_buffer.set_text("");
        
        for entry in &self.filtered_entries {
            self.append_to_buffer(entry);
        }
    }

    /// 设置最小日志级别过滤
    pub fn set_min_level(&mut self, min_level: LogLevel) {
        self.filter.set_min_level(min_level);
        
        // 重新过滤
        self.filtered_entries.clear();
        for entry in &self.log_entries {
            if self.filter.matches(entry) {
                self.filtered_entries.push(entry.clone());
            }
        }
        self.filtered_count = self.filtered_entries.len();
        
        // 刷新显示
        self.refresh_filtered_display();
    }
}

/// 标签页管理器
pub struct TabManager {
    notebook: Notebook,
    tabs: Vec<Rc<RefCell<LogTab>>>,
    next_id: usize,
}

impl TabManager {
    pub fn new(notebook: Notebook) -> Self {
        Self {
            notebook,
            tabs: Vec::new(),
            next_id: 0,
        }
    }

    /// 创建新标签页
    pub fn create_tab(&mut self, name: &str) -> Rc<RefCell<LogTab>> {
        let id = self.next_id;
        self.next_id += 1;

        let tab = Rc::new(RefCell::new(LogTab::new(id, name.to_string())));
        
        // 创建标签页UI
        let scrolled = ScrolledWindow::builder()
            .hexpand(true)
            .vexpand(true)
            .build();
        
        let text_view = tab.borrow().text_view.clone();
        scrolled.set_child(Some(&text_view));

        // 创建标签标题（包含关闭按钮）
        let (tab_label, label_widget) = self.create_tab_label(name, id);
        
        // 保存标签页标题 widget 引用
        tab.borrow_mut().tab_label_widget = Some(label_widget);
        
        // 添加到notebook
        let page_num = self.notebook.append_page(&scrolled, Some(&tab_label));
        self.notebook.set_tab_reorderable(&scrolled, true);
        
        // 切换到新标签页
        self.notebook.set_current_page(Some(page_num as u32));

        self.tabs.push(tab.clone());
        tab
    }

    /// 创建标签标题，返回 (容器, 标签文本widget)
    fn create_tab_label(&self, name: &str, tab_id: usize) -> (gtk4::Box, gtk4::Label) {
        let hbox = gtk4::Box::new(Orientation::Horizontal, 6);
        
        let label = Label::new(Some(name));
        hbox.append(&label);

        let close_btn = Button::builder()
            .icon_name("window-close-symbolic")
            .has_frame(false)
            .build();
        
        hbox.append(&close_btn);

        (hbox, label)
    }

    /// 更新指定标签页的标题
    pub fn update_tab_title(&self, tab_id: usize) {
        if let Some(pos) = self.tabs.iter().position(|t| t.borrow().id == tab_id) {
            let tab = self.tabs[pos].borrow();
            if let Some(ref label_widget) = tab.tab_label_widget {
                let new_title = tab.get_tab_title();
                label_widget.set_text(&new_title);
            }
        }
    }

    /// 更新所有标签页的标题（用于刷新连接状态）
    pub fn refresh_all_tab_titles(&self) {
        for tab in &self.tabs {
            let tab_ref = tab.borrow();
            if let Some(ref label_widget) = tab_ref.tab_label_widget {
                let new_title = tab_ref.get_tab_title();
                label_widget.set_text(&new_title);
            }
        }
    }

    /// 获取当前标签页
    pub fn current_tab(&self) -> Option<Rc<RefCell<LogTab>>> {
        let current_page = self.notebook.current_page()?;
        self.tabs.get(current_page as usize).cloned()
    }

    /// 切换到下一个标签页
    pub fn next_tab(&self) {
        let current = self.notebook.current_page().unwrap_or(0);
        let total = self.notebook.n_pages();
        if total > 0 {
            let next = (current + 1) % total;
            self.notebook.set_current_page(Some(next));
        }
    }

    /// 切换到上一个标签页
    pub fn prev_tab(&self) {
        let current = self.notebook.current_page().unwrap_or(0);
        let total = self.notebook.n_pages();
        if total > 0 {
            let prev = if current == 0 { total - 1 } else { current - 1 };
            self.notebook.set_current_page(Some(prev));
        }
    }

    /// 关闭标签页
    pub fn close_tab(&mut self, tab_id: usize) {
        if let Some(pos) = self.tabs.iter().position(|t| t.borrow().id == tab_id) {
            // 停止日志源
            self.tabs[pos].borrow_mut().stop_source();
            
            // 从notebook移除
            if let Some(page) = self.notebook.nth_page(Some(pos as u32)) {
                self.notebook.remove_page(Some(pos as u32));
            }
            
            // 从列表移除
            self.tabs.remove(pos);
        }
    }

    /// 关闭当前标签页
    pub fn close_current_tab(&mut self) {
        if let Some(current_page) = self.notebook.current_page() {
            if let Some(tab) = self.tabs.get(current_page as usize) {
                let tab_id = tab.borrow().id;
                self.close_tab(tab_id);
            }
        }
    }

    /// 获取标签页数量
    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    /// 获取notebook
    pub fn notebook(&self) -> &Notebook {
        &self.notebook
    }
}
