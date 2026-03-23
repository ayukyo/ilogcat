use gtk4::prelude::*;
use gtk4::{Notebook, Label, Orientation, Button, ScrolledWindow, TextView, TextBuffer};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::log::{LogSource, LogEntry, LogLevel};
use crate::filter::Filter;

/// 单个日志窗口标签页的状态
pub struct LogTab {
    pub id: usize,
    pub name: String,
    pub log_entries: Vec<LogEntry>,
    pub filtered_entries: Vec<LogEntry>,
    pub filter: Filter,
    pub current_source: Option<std::boxed::Box<dyn LogSource>>,
    pub is_paused: Arc<AtomicBool>,
    pub log_count: usize,
    pub filtered_count: usize,
    pub text_buffer: TextBuffer,
    pub text_view: TextView,
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
            name,
            log_entries: Vec::new(),
            filtered_entries: Vec::new(),
            filter: Filter::new(),
            current_source: None,
            is_paused: Arc::new(AtomicBool::new(false)),
            log_count: 0,
            filtered_count: 0,
            text_buffer,
            text_view,
        }
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
        let tab_label = self.create_tab_label(name, id);
        
        // 添加到notebook
        self.notebook.append_page(&scrolled, Some(&tab_label));
        self.notebook.set_tab_reorderable(&scrolled, true);
        
        // 切换到新标签页
        let page_num = self.notebook.n_pages() - 1;
        self.notebook.set_current_page(Some(page_num));

        self.tabs.push(tab.clone());
        tab
    }

    /// 创建标签标题
    fn create_tab_label(&self, name: &str, tab_id: usize) -> gtk4::Box {
        let hbox = gtk4::Box::new(Orientation::Horizontal, 6);
        
        let label = Label::new(Some(name));
        hbox.append(&label);

        let close_btn = Button::builder()
            .icon_name("window-close-symbolic")
            .has_frame(false)
            .build();
        
        hbox.append(&close_btn);

        hbox
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
