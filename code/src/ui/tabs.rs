use gtk4::prelude::*;
use gtk4::{Notebook, Label, Orientation, Button, ScrolledWindow, TextView, TextBuffer, Entry, Box};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::collections::HashMap;

use crate::log::{LogSource, LogEntry, LogLevel};
use crate::filter::Filter;
use crate::i18n::{t, I18nKey};
use crate::ssh::config::SshConfig;

/// 单个日志窗口标签页的状态
pub struct LogTab {
    pub id: usize,
    pub name: String,
    pub source_name: String,  // 日志来源名称（用于标签页标题）
    pub source_type: SourceType,  // 日志源类型
    pub log_entries: Vec<LogEntry>,
    pub filtered_entries: Vec<LogEntry>,
    pub filter: Filter,
    pub filter_text: String,  // 当前过滤文本（用于切换标签页时同步）
    pub current_source: Option<std::boxed::Box<dyn LogSource>>,
    pub is_paused: Arc<AtomicBool>,
    pub log_count: usize,
    pub filtered_count: usize,
    pub text_buffer: TextBuffer,
    pub text_view: TextView,
    pub tab_label_widget: Option<gtk4::Label>,  // 保存标签页标题文本 widget 引用
    pub command_entry: Entry,  // 命令输入框
    pub ssh_config: Option<SshConfig>,  // SSH配置（如果已连接）
    pub ssh_connected: Arc<AtomicBool>,  // SSH连接状态
    pub ssh_reconnecting: Arc<AtomicBool>,  // 是否正在重连中
    pub current_path: Option<String>,  // 当前工作目录
    pub pending_cd: bool,  // 是否在等待cd命令的路径更新
    pub terminal_history: Vec<String>,  // 终端历史记录
    // 统计信息
    level_counts: HashMap<LogLevel, usize>,
    start_time: std::time::Instant,
    // 智能自动滚动
    pub auto_scroll_enabled: Arc<AtomicBool>,  // 是否启用自动滚动
    pub user_scrolled_up: Arc<AtomicBool>,     // 用户是否向上滚动查看历史
}

/// 日志源类型
#[derive(Clone, Debug)]
pub enum SourceType {
    Dmesg,
    Journalctl,
    File(String),  // 文件路径
    Ssh(String, String),  // (主机名, 命令)
    SshCommand(String, String),  // (主机名, 自定义命令)
    Command(String),  // 自定义命令
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

        let command_entry = Entry::builder()
            .placeholder_text(&t(I18nKey::PlaceholderCommand))
            .hexpand(true)
            .build();

        let mut tab = Self {
            id,
            name: name.clone(),
            source_name: name,
            source_type: SourceType::Unknown,
            log_entries: Vec::new(),
            filtered_entries: Vec::new(),
            filter: Filter::new(),
            filter_text: String::new(),
            current_source: None,
            is_paused: Arc::new(AtomicBool::new(false)),
            log_count: 0,
            filtered_count: 0,
            text_buffer,
            text_view,
            tab_label_widget: None,
            command_entry,
            ssh_config: None,
            ssh_connected: Arc::new(AtomicBool::new(false)),
            ssh_reconnecting: Arc::new(AtomicBool::new(false)),
            current_path: None,
            pending_cd: false,
            terminal_history: Vec::new(),
            level_counts: HashMap::new(),
            start_time: std::time::Instant::now(),
            auto_scroll_enabled: Arc::new(AtomicBool::new(true)),
            user_scrolled_up: Arc::new(AtomicBool::new(false)),
        };

        // 设置智能滚动事件监听
        tab.setup_smart_scroll();

        tab
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
            SourceType::Command(cmd) => {
                let short_cmd = if cmd.len() > 30 {
                    format!("{}...", &cmd[..30])
                } else {
                    cmd.clone()
                };
                format!("💻 {}", short_cmd)
            }
            SourceType::Unknown => self.name.clone(),
        };
        self.source_type = source_type;
    }

    /// 获取连接状态图标
    pub fn get_connection_status_icon(&self) -> &'static str {
        match &self.source_type {
            SourceType::Ssh(_, _) | SourceType::SshCommand(_, _) => {
                if self.ssh_connected.load(Ordering::SeqCst) {
                    "🟢"  // 连接中
                } else {
                    "🔴"  // 断开
                }
            }
            _ => "",
        }
    }

    /// 设置SSH连接状态
    pub fn set_ssh_connected(&mut self, connected: bool) {
        self.ssh_connected.store(connected, Ordering::SeqCst);
    }

    /// 检查SSH是否已连接
    pub fn is_ssh_connected(&self) -> bool {
        self.ssh_connected.load(Ordering::SeqCst)
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

        // 时间戳标签 - 浅灰色背景，粗体
        let tag_timestamp = gtk4::TextTag::builder()
            .name("timestamp")
            .foreground("#666666")
            .background("#E0E0E0")
            .weight(600)
            .build();
        tag_table.add(&tag_timestamp);

        let tag_trace = gtk4::TextTag::builder()
            .name("trace")
            .foreground("#909090")
            .build();
        tag_table.add(&tag_trace);

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

        let tag_critical = gtk4::TextTag::builder()
            .name("critical")
            .foreground("#FF0000")
            .weight(800)
            .background("#330000")
            .build();
        tag_table.add(&tag_critical);

        // 终端提示符标签（绿色）
        let tag_prompt = gtk4::TextTag::builder()
            .name("terminal_prompt")
            .foreground("#00FF00")
            .weight(500)
            .build();
        tag_table.add(&tag_prompt);

        // 关键字高亮标签（黄色背景，黑色粗体）
        let tag_highlight = gtk4::TextTag::builder()
            .name("highlight")
            .background("#FFFF00")
            .foreground("#000000")
            .weight(800)
            .build();
        tag_table.add(&tag_highlight);
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
    fn append_to_buffer(&mut self, entry: &LogEntry) {
        let level_tag_name = match entry.level {
            LogLevel::Trace => "trace",
            LogLevel::Verbose => "verbose",
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
            LogLevel::Fatal => "fatal",
            LogLevel::Critical => "critical",
        };

        // 直接显示原始日志行，保留原始格式
        let line = format!("{}\n", entry.raw_line);

        let mut end_iter = self.text_buffer.end_iter();
        let line_start = self.text_buffer.char_count();
        self.text_buffer.insert(&mut end_iter, &line);

        // 应用级别样式到整行
        let line_end = self.text_buffer.char_count();
        if line_start >= 0 && line_end > line_start {
            let start_iter = self.text_buffer.iter_at_offset(line_start);
            let end_iter = self.text_buffer.iter_at_offset(line_end);
            self.text_buffer.apply_tag_by_name(level_tag_name, &start_iter, &end_iter);
        }

        // 高亮过滤关键字
        if !self.filter.keywords.is_empty() {
            // 使用原始日志行（不含换行符）进行匹配
            let line_text = &entry.raw_line;
            for kw in &self.filter.keywords {
                // 使用 find_matches 获取所有匹配位置（支持正则表达式）
                let matches = kw.find_matches(line_text);
                for (start, end) in matches {
                    let char_offset_start = line_text[..start].chars().count() as i32;
                    let char_offset_end = line_text[..end].chars().count() as i32;

                    let highlight_start = line_start + char_offset_start;
                    let highlight_end = line_start + char_offset_end;

                    if highlight_end <= line_end {
                        let start_iter = self.text_buffer.iter_at_offset(highlight_start);
                        let end_iter = self.text_buffer.iter_at_offset(highlight_end);
                        self.text_buffer.apply_tag_by_name("highlight", &start_iter, &end_iter);
                    }
                }
            }
        }

        // 自动滚动到底部
        if !self.is_paused.load(Ordering::SeqCst) {
            self.scroll_to_end();
        }
    }

    /// 滚动到末尾
    fn scroll_to_end(&self) {
        // 只有启用了自动滚动且用户没有向上滚动时才滚动
        if !self.auto_scroll_enabled.load(Ordering::SeqCst) {
            return;
        }
        if self.user_scrolled_up.load(Ordering::SeqCst) {
            return;
        }
        
        let mut end_iter = self.text_buffer.end_iter();
        self.text_view.scroll_to_iter(&mut end_iter, 0.0, false, 0.0, 0.0);
    }
    
    /// 设置智能滚动事件监听
    fn setup_smart_scroll(&self) {
        let user_scrolled_up = self.user_scrolled_up.clone();
        let auto_scroll_enabled = self.auto_scroll_enabled.clone();
        let text_view = self.text_view.clone();
        
        // 创建滚动控制器
        let scroll_controller = gtk4::EventControllerScroll::new(
            gtk4::EventControllerScrollFlags::VERTICAL
        );
        
        scroll_controller.connect_scroll(move |_, _dx, dy| {
            // dy > 0 表示向下滚动，dy < 0 表示向上滚动
            if dy < 0.0 {
                // 用户向上滚动，暂停自动滚动
                user_scrolled_up.store(true, Ordering::SeqCst);
            } else {
                // 用户向下滚动，检查是否到达底部
                if let Some(vadj) = text_view.vadjustment() {
                    let current = vadj.value();
                    let upper = vadj.upper();
                    let page_size = vadj.page_size();
                    
                    // 如果接近底部（在50像素内），恢复自动滚动
                    if current + page_size >= upper - 50.0 {
                        user_scrolled_up.store(false, Ordering::SeqCst);
                    }
                }
            }
            glib::Propagation::Proceed
        });
        
        self.text_view.add_controller(scroll_controller);
        
        // 监听垂直调整值变化，检测是否滚动到底部
        if let Some(vadj) = self.text_view.vadjustment() {
            let user_scrolled_up_clone = self.user_scrolled_up.clone();
            
            vadj.connect_value_changed(move |adj: &gtk4::Adjustment| {
                let current = adj.value();
                let upper = adj.upper();
                let page_size = adj.page_size();
                
                // 如果滚动到底部附近，重置用户滚动标志
                if current + page_size >= upper - 10.0 {
                    user_scrolled_up_clone.store(false, Ordering::SeqCst);
                }
            });
        }
    }
    
    /// 切换自动滚动状态
    pub fn toggle_auto_scroll(&mut self) {
        let current = self.auto_scroll_enabled.load(Ordering::SeqCst);
        self.auto_scroll_enabled.store(!current, Ordering::SeqCst);
        
        // 如果重新启用自动滚动，重置用户滚动标志
        if !current {
            self.user_scrolled_up.store(false, Ordering::SeqCst);
        }
    }
    
    /// 获取自动滚动状态
    pub fn is_auto_scroll_enabled(&self) -> bool {
        self.auto_scroll_enabled.load(Ordering::SeqCst)
    }
    
    /// 手动跳转到最新日志（恢复自动滚动）
    pub fn jump_to_latest(&self) {
        self.user_scrolled_up.store(false, Ordering::SeqCst);
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

    /// 获取过滤文本
    pub fn get_filter_text(&self) -> &str {
        &self.filter_text
    }

    /// 设置过滤文本
    pub fn set_filter_text(&mut self, text: String) {
        self.filter_text = text;
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

    /// 设置SSH配置
    pub fn set_ssh_config(&mut self, config: SshConfig) {
        self.ssh_config = Some(config);
    }

    /// 获取SSH配置
    pub fn get_ssh_config(&self) -> Option<&SshConfig> {
        self.ssh_config.as_ref()
    }

    /// 清除SSH配置
    pub fn clear_ssh_config(&mut self) {
        self.ssh_config = None;
        self.ssh_connected.store(false, Ordering::SeqCst);
    }

    /// 检查SSH连接是否断开（需要重连）
    pub fn should_reconnect_ssh(&self) -> bool {
        // 如果没有SSH配置，不需要重连
        if self.ssh_config.is_none() {
            return false;
        }
        // 如果正在重连中，不触发新的重连
        if self.ssh_reconnecting.load(Ordering::SeqCst) {
            return false;
        }
        // 如果已经标记为断开，不触发重连（用户手动断开）
        if !self.ssh_connected.load(Ordering::SeqCst) {
            return false;
        }
        // 如果有日志源且不在运行，说明断开了
        if self.current_source.is_some() && !self.is_source_running() {
            return true;
        }
        false
    }

    /// 开始SSH重连
    pub fn start_reconnect(&self) {
        self.ssh_reconnecting.store(true, Ordering::SeqCst);
    }

    /// 完成SSH重连
    pub fn finish_reconnect(&self, success: bool) {
        self.ssh_reconnecting.store(false, Ordering::SeqCst);
        if success {
            self.ssh_connected.store(true, Ordering::SeqCst);
        }
    }

    /// 显示终端风格的输出
    pub fn append_terminal_output(&mut self, text: &str) {
        // 添加时间戳前缀
        let timestamp = chrono::Local::now().format("%H:%M:%S");
        let line = format!("[{}] {}", timestamp, text);

        // 保存到终端历史
        self.terminal_history.push(line.clone());

        // 添加到缓冲区
        let mut end_iter = self.text_buffer.end_iter();
        self.text_buffer.insert(&mut end_iter, &format!("{}\n", line));

        // 滚动到底部
        self.scroll_to_end();
    }

    /// 显示命令提示符和执行的命令
    pub fn append_command(&mut self, prompt: &str, command: &str) {
        let line = format!("{}{}", prompt, command);
        self.terminal_history.push(line.clone());

        // 添加提示符（绿色）
        let mut end_iter = self.text_buffer.end_iter();
        let prompt_start = self.text_buffer.char_count();
        self.text_buffer.insert(&mut end_iter, prompt);

        // 应用绿色标签到提示符
        let prompt_end = self.text_buffer.char_count();
        if prompt_start < prompt_end {
            let start_iter = self.text_buffer.iter_at_offset(prompt_start);
            let end_iter = self.text_buffer.iter_at_offset(prompt_end);
            if let Some(tag) = self.text_buffer.tag_table().lookup("terminal_prompt") {
                self.text_buffer.apply_tag(&tag, &start_iter, &end_iter);
            }
        }

        // 添加命令（普通文本）
        end_iter = self.text_buffer.end_iter();
        self.text_buffer.insert(&mut end_iter, &format!("{}\n", command));

        self.scroll_to_end();
    }

    /// 生成命令提示符
    pub fn get_prompt(&self) -> String {
        if let Some(ref ssh) = self.ssh_config {
            let path = self.current_path.as_deref().unwrap_or("~");
            if path == "/" {
                format!("{}@{}:/ ", ssh.username, ssh.host)
            } else if path.starts_with("/home/") {
                // 简化 home 目录显示
                let home_path = format!("/home/{}/", ssh.username);
                if path.starts_with(&home_path) {
                    let rest = &path[home_path.len()..];
                    format!("{}@{}:~/{}/ ", ssh.username, ssh.host, rest)
                } else if path == &format!("/home/{}", ssh.username) || path == &format!("/home/{}/", ssh.username) {
                    format!("{}@{}:~ ", ssh.username, ssh.host)
                } else {
                    format!("{}@{}:{} ", ssh.username, ssh.host, path)
                }
            } else {
                format!("{}@{}:{} ", ssh.username, ssh.host, path)
            }
        } else {
            "$ ".to_string()
        }
    }

    /// 设置当前路径
    pub fn set_current_path(&mut self, path: String) {
        self.current_path = Some(path);
        self.update_command_prompt();
    }

    /// 更新命令输入框提示
    pub fn update_command_prompt(&self) {
        let prompt = self.get_prompt();
        self.command_entry.set_placeholder_text(Some(&prompt));
    }

    /// 刷新过滤显示
    pub fn refresh_filtered_display(&mut self) {
        self.text_buffer.set_text("");

        // 先克隆 entries 避免借用冲突
        let entries: Vec<LogEntry> = self.filtered_entries.clone();
        for entry in &entries {
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
    tabs: Rc<RefCell<Vec<Rc<RefCell<LogTab>>>>>,
    next_id: usize,
    command_history: Rc<RefCell<Vec<String>>>,
}

impl TabManager {
    pub fn new(notebook: Notebook) -> Self {
        Self {
            notebook,
            tabs: Rc::new(RefCell::new(Vec::new())),
            next_id: 0,
            command_history: Rc::new(RefCell::new(Vec::new())),
        }
    }

    /// 设置命令历史
    pub fn set_command_history(&mut self, history: Vec<String>) {
        *self.command_history.borrow_mut() = history;
    }

    /// 获取命令历史
    pub fn get_command_history(&self) -> Vec<String> {
        self.command_history.borrow().clone()
    }

    /// 创建新标签页
    pub fn create_tab(&mut self, name: &str) -> Rc<RefCell<LogTab>> {
        let id = self.next_id;
        self.next_id += 1;

        let tab = Rc::new(RefCell::new(LogTab::new(id, name.to_string())));

        // 创建主容器（垂直布局）
        let main_box = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(4)
            .build();

        // 创建命令输入区域
        let cmd_box = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(6)
            .margin_start(4)
            .margin_end(4)
            .margin_top(4)
            .margin_bottom(4)
            .build();

        let cmd_label = Label::new(Some(&t(I18nKey::LabelCommand)));
        cmd_box.append(&cmd_label);

        let command_entry = tab.borrow().command_entry.clone();
        cmd_box.append(&command_entry);

        let run_btn = Button::builder()
            .label(&t(I18nKey::ButtonRun))
            .tooltip_text(&t(I18nKey::TooltipRunCommand))
            .build();
        cmd_box.append(&run_btn);

        // 历史记录按钮
        let history_btn = Button::builder()
            .label("📜")
            .tooltip_text("Command history")
            .build();
        cmd_box.append(&history_btn);

        main_box.append(&cmd_box);

        // 创建日志输出区域
        let scrolled = ScrolledWindow::builder()
            .hexpand(true)
            .vexpand(true)
            .build();

        let text_view = tab.borrow().text_view.clone();
        scrolled.set_child(Some(&text_view));

        main_box.append(&scrolled);

        // 创建标签标题（包含关闭按钮）
        let tabs_clone = self.tabs.clone();
        let notebook_clone = self.notebook.clone();
        let (tab_label, label_widget) = self.create_tab_label(name, id, tabs_clone, notebook_clone);

        // 保存标签页标题 widget 引用
        tab.borrow_mut().tab_label_widget = Some(label_widget);

        // 添加到notebook
        let page_num = self.notebook.append_page(&main_box, Some(&tab_label));
        self.notebook.set_tab_reorderable(&main_box, true);

        // 命令历史引用
        let history_ref = self.command_history.clone();

        // 执行命令的闭包
        let execute_command = {
            let tab_clone = tab.clone();
            let history_ref = history_ref.clone();
            move |input: String| {
                // 清理命令：
                // 1. 替换字面的 \r 文本为换行符（处理复制粘贴的情况）
                // 2. 将真正的 \r 控制字符转换为换行符
                // 3. 移除其他控制字符（除了空格和换行）
                let cleaned = input
                    .replace("\\r", "\n")  // 替换字面的 \r 文本
                    .replace("\r", "\n");   // 替换真正的回车符

                let cleaned: String = cleaned.chars()
                    .filter(|c| !c.is_control() || *c == ' ' || *c == '\n')
                    .collect();

                // 按行分割，过滤空行，移除每行的前后空白
                let commands: Vec<String> = cleaned.lines()
                    .map(|line| line.trim().to_string())
                    .filter(|line| !line.is_empty())
                    .collect();

                // 检查是否已连接SSH
                let ssh_config = tab_clone.borrow().ssh_config.clone();
                let is_ssh = ssh_config.is_some();

                if is_ssh {
                    // SSH: 将多行命令合并成一个用 ; 连接的命令
                    let combined_cmd = commands.join(" ; ");

                    // 保存到历史（保存合并后的命令）
                    {
                        let mut history = history_ref.borrow_mut();
                        history.retain(|c| c != &combined_cmd);
                        history.insert(0, combined_cmd.clone());
                        if history.len() > 10 {
                            history.truncate(10);
                        }
                    }

                    // 显示命令提示符和命令
                    let prompt = tab_clone.borrow().get_prompt();
                    tab_clone.borrow_mut().append_command(&prompt, &combined_cmd);

                    // 停止当前源
                    tab_clone.borrow_mut().stop_source();

                    let host = ssh_config.as_ref().map(|c| c.host.clone()).unwrap_or_default();

                    if let Some(ssh_cfg) = ssh_config {
                        let current_path = tab_clone.borrow().current_path.clone();
                        let cmd_trimmed = combined_cmd.trim();

                        // 检查命令是否以cd开头
                        let starts_with_cd = cmd_trimmed.starts_with("cd ") || cmd_trimmed.starts_with("cd\t");

                        // 构建实际命令：如果命令不以cd开头，先cd到当前目录
                        let actual_cmd = if starts_with_cd {
                            // 命令本身包含cd，直接执行
                            cmd_trimmed.to_string()
                        } else {
                            match &current_path {
                                Some(path) if path != "~" && !path.is_empty() => {
                                    format!("cd {} && {}", path, cmd_trimmed)
                                }
                                _ => cmd_trimmed.to_string()
                            }
                        };

                        // 如果命令以cd开头，尝试提取新路径
                        let new_path = if starts_with_cd {
                            // 提取cd目标目录
                            cmd_trimmed.lines().next().and_then(|first_line| {
                                first_line.strip_prefix("cd ").map(|s| s.trim().to_string())
                            })
                        } else {
                            None
                        };

                        let mut source = crate::log::SshSource::new(ssh_cfg.clone(), actual_cmd.clone());
                        tab_clone.borrow_mut().set_source_info(SourceType::SshCommand(host, combined_cmd));

                        if let Err(e) = source.start() {
                            tab_clone.borrow_mut().append_terminal_output(&format!("错误: {}", e));
                            tab_clone.borrow_mut().set_ssh_connected(false);
                        } else {
                            tab_clone.borrow_mut().set_source(std::boxed::Box::new(source));

                            // 更新当前目录
                            if let Some(new_path) = new_path {
                                tab_clone.borrow_mut().set_current_path(new_path);
                            }
                        }
                    }
                } else {
                    // 本地执行：逐个执行命令
                    for cmd in commands {
                        // 保存到历史
                        {
                            let mut history = history_ref.borrow_mut();
                            history.retain(|c| c != &cmd);
                            history.insert(0, cmd.clone());
                            if history.len() > 10 {
                                history.truncate(10);
                            }
                        }

                        // 显示命令提示符和命令
                        let prompt = tab_clone.borrow().get_prompt();
                        tab_clone.borrow_mut().append_command(&prompt, &cmd);

                        // 停止当前源
                        tab_clone.borrow_mut().stop_source();

                        // 本地执行命令
                        let parts: Vec<String> = match shlex::split(&cmd) {
                            Some(parts) => parts,
                            None => {
                                tab_clone.borrow_mut().append_terminal_output("错误: 无法解析命令");
                                continue;
                            }
                        };

                        if parts.is_empty() {
                            continue;
                        }

                        let command = parts[0].clone();
                        let args: Vec<String> = parts[1..].to_vec();

                        let mut source = crate::log::CommandSource::with_args(
                            command.clone(),
                            args.clone()
                        );

                        tab_clone.borrow_mut().set_source_info(SourceType::Command(cmd.clone()));

                        if let Err(e) = source.start() {
                            tab_clone.borrow_mut().append_terminal_output(&format!("错误: {}", e));
                        } else {
                            tab_clone.borrow_mut().set_source(std::boxed::Box::new(source));
                        }
                    }
                }
            }
        };

        // 设置运行按钮点击事件
        let command_entry_clone = command_entry.clone();
        let execute_cmd = execute_command.clone();
        run_btn.connect_clicked(move |_| {
            let cmd = command_entry_clone.text().to_string();
            execute_cmd(cmd);
        });

        // 设置回车键执行命令
        let command_entry_clone = command_entry.clone();
        let execute_cmd = execute_command;
        let command_entry_for_clear = command_entry.clone();
        command_entry.connect_activate(move |_entry| {
            let cmd = command_entry_clone.text().to_string();
            execute_cmd(cmd);
            command_entry_for_clear.set_text("");
        });

        // 设置上下箭头键导航历史
        let history_for_nav = history_ref.clone();
        let history_index: Rc<RefCell<usize>> = Rc::new(RefCell::new(0));
        let command_entry_for_nav = command_entry.clone();
        let tab_for_tab = tab.clone();

        let key_controller = gtk4::EventControllerKey::new();
        key_controller.connect_key_pressed(move |_, key, _, _| {
            match key {
                gtk4::gdk::Key::Tab => {
                    // TAB 补全：向服务器请求补全建议
                    let current_text = command_entry_for_nav.text().to_string();
                    if !current_text.is_empty() {
                        if let Some(ref ssh_cfg) = tab_for_tab.borrow().ssh_config {
                            let ssh_cfg_clone = ssh_cfg.clone();
                            let current_path = tab_for_tab.borrow().current_path.clone();
                            let entry_for_idle = command_entry_for_nav.clone();

                            // 使用 channel 在后台获取补全
                            let (sender, receiver) = std::sync::mpsc::channel::<Option<Vec<String>>>();
                            let text_for_thread = current_text.clone();

                            std::thread::spawn(move || {
                                let result = get_tab_completions(&ssh_cfg_clone, &text_for_thread, current_path.as_deref());
                                let _ = sender.send(result);
                            });

                            // 使用 idle_add 检查结果
                            let original_text = current_text.clone();
                            glib::idle_add_local(move || {
                                match receiver.try_recv() {
                                    Ok(Some(completions)) => {
                                        if completions.len() == 1 {
                                            entry_for_idle.set_text(&completions[0]);
                                            entry_for_idle.set_position(-1);
                                        } else if completions.len() > 1 {
                                            let common_prefix = find_common_prefix(&completions);
                                            if !common_prefix.is_empty() && common_prefix != original_text {
                                                entry_for_idle.set_text(&common_prefix);
                                                entry_for_idle.set_position(-1);
                                            }
                                        }
                                        return glib::ControlFlow::Break;
                                    }
                                    Ok(None) => {
                                        return glib::ControlFlow::Break;
                                    }
                                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                                        // 继续等待
                                        glib::ControlFlow::Continue
                                    }
                                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                                        return glib::ControlFlow::Break;
                                    }
                                }
                            });
                        }
                    }
                    glib::Propagation::Stop
                }
                gtk4::gdk::Key::Up => {
                    let history = history_for_nav.borrow();
                    if !history.is_empty() {
                        let mut idx = history_index.borrow_mut();
                        if *idx < history.len() {
                            *idx += 1;
                            let hist_idx = history.len() - *idx;
                            command_entry_for_nav.set_text(&history[hist_idx]);
                            command_entry_for_nav.set_position(-1);
                        }
                    }
                    glib::Propagation::Stop
                }
                gtk4::gdk::Key::Down => {
                    let history = history_for_nav.borrow();
                    let mut idx = history_index.borrow_mut();
                    if *idx > 0 {
                        *idx -= 1;
                        if *idx == 0 {
                            command_entry_for_nav.set_text("");
                        } else {
                            let hist_idx = history.len() - *idx;
                            command_entry_for_nav.set_text(&history[hist_idx]);
                            command_entry_for_nav.set_position(-1);
                        }
                    }
                    glib::Propagation::Stop
                }
                _ => {
                    // 其他键重置历史索引
                    *history_index.borrow_mut() = 0;
                    glib::Propagation::Proceed
                }
            }
        });
        command_entry.add_controller(key_controller);

        // 历史按钮点击事件
        let history_ref = history_ref.clone();
        let command_entry_clone = command_entry.clone();
        let history_btn_for_popover = history_btn.clone();
        history_btn.connect_clicked(move |_| {
            let history = history_ref.borrow().clone();
            if history.is_empty() {
                return;
            }

            // 创建历史菜单
            let popover = gtk4::Popover::builder()
                .has_arrow(true)
                .build();
            let list_box = gtk4::ListBox::new();

            for cmd in &history {
                let label = Label::new(Some(cmd));
                label.set_halign(gtk4::Align::Start);
                label.set_margin_start(8);
                label.set_margin_end(8);
                label.set_margin_top(4);
                label.set_margin_bottom(4);
                list_box.append(&label);
            }

            let command_entry_for_cb = command_entry_clone.clone();
            let popover_clone = popover.clone();
            list_box.connect_row_activated(move |_, row| {
                if let Some(label) = row.child().and_then(|c| c.downcast::<Label>().ok()) {
                    let text = label.text().to_string();
                    command_entry_for_cb.set_text(&text);
                    command_entry_for_cb.emit_activate();  // 直接执行
                    popover_clone.popdown();  // 关闭菜单
                }
            });

            let scrolled = gtk4::ScrolledWindow::builder()
                .max_content_height(300)
                .hscrollbar_policy(gtk4::PolicyType::Never)
                .build();
            scrolled.set_child(Some(&list_box));
            popover.set_child(Some(&scrolled));
            popover.set_parent(&history_btn_for_popover);
            popover.popup();
        });

        // 切换到新标签页
        self.notebook.set_current_page(Some(page_num as u32));

        self.tabs.borrow_mut().push(tab.clone());
        tab
    }

    /// 创建标签标题，返回 (容器, 标签文本widget)
    fn create_tab_label(&self, name: &str, tab_id: usize, tabs: Rc<RefCell<Vec<Rc<RefCell<LogTab>>>>>, notebook: Notebook) -> (gtk4::Box, gtk4::Label) {
        let hbox = gtk4::Box::new(Orientation::Horizontal, 6);

        let label = Label::new(Some(name));
        hbox.append(&label);

        let close_btn = Button::builder()
            .icon_name("window-close-symbolic")
            .has_frame(false)
            .build();

        hbox.append(&close_btn);

        // 绑定关闭按钮事件
        close_btn.connect_clicked(move |_| {
            // 查找标签页索引
            let mut tabs_vec = tabs.borrow_mut();
            if let Some(pos) = tabs_vec.iter().position(|t| t.borrow().id == tab_id) {
                // 停止日志源
                tabs_vec[pos].borrow_mut().stop_source();

                // 从notebook移除
                notebook.remove_page(Some(pos as u32));

                // 从列表移除
                tabs_vec.remove(pos);
            }
        });

        (hbox, label)
    }

    /// 更新指定标签页的标题
    pub fn update_tab_title(&self, tab_id: usize) {
        let tabs = self.tabs.borrow();
        if let Some(pos) = tabs.iter().position(|t| t.borrow().id == tab_id) {
            let tab = tabs[pos].borrow();
            if let Some(ref label_widget) = tab.tab_label_widget {
                let new_title = tab.get_tab_title();
                label_widget.set_text(&new_title);
            }
        }
    }

    /// 更新所有标签页的标题（用于刷新连接状态）
    pub fn refresh_all_tab_titles(&self) {
        let tabs = self.tabs.borrow();
        for tab in tabs.iter() {
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
        self.tabs.borrow().get(current_page as usize).cloned()
    }

    /// 根据ID获取标签页
    pub fn get_tab_by_id(&self, tab_id: usize) -> Option<Rc<RefCell<LogTab>>> {
        let tabs = self.tabs.borrow();
        tabs.iter()
            .find(|tab| tab.borrow().id == tab_id)
            .cloned()
    }

    /// 获取所有标签页（只读）
    pub fn tabs(&self) -> Rc<RefCell<Vec<Rc<RefCell<LogTab>>>>> {
        self.tabs.clone()
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
        let mut tabs = self.tabs.borrow_mut();
        if let Some(pos) = tabs.iter().position(|t| t.borrow().id == tab_id) {
            // 停止日志源
            tabs[pos].borrow_mut().stop_source();

            // 从notebook移除
            self.notebook.remove_page(Some(pos as u32));

            // 从列表移除
            tabs.remove(pos);
        }
    }

    /// 关闭当前标签页
    pub fn close_current_tab(&mut self) {
        if let Some(current_page) = self.notebook.current_page() {
            let tabs = self.tabs.borrow();
            if let Some(tab) = tabs.get(current_page as usize) {
                let tab_id = tab.borrow().id;
                drop(tabs); // 释放借用
                self.close_tab(tab_id);
            }
        }
    }

    /// 获取标签页数量
    pub fn tab_count(&self) -> usize {
        self.tabs.borrow().len()
    }

    /// 获取notebook
    pub fn notebook(&self) -> &Notebook {
        &self.notebook
    }
}

/// TAB 补全：获取服务器上的补全建议
fn get_tab_completions(ssh_config: &SshConfig, partial: &str, current_path: Option<&str>) -> Option<Vec<String>> {
    use ssh2::Session;
    use std::net::TcpStream;
    use std::io::Read;
    use std::time::Duration;

    let addr = format!("{}:{}", ssh_config.host, ssh_config.port);

    // 建立 TCP 连接
    let tcp = TcpStream::connect_timeout(
        &addr.parse().ok()?,
        Duration::from_secs(5),
    ).ok()?;

    let mut session = Session::new().ok()?;
    session.set_tcp_stream(tcp);
    session.handshake().ok()?;

    // 认证
    match &ssh_config.auth {
        crate::ssh::config::AuthMethod::Password(password) => {
            // 尝试多种认证方式
            if session.userauth_password(&ssh_config.username, "").is_ok() && session.authenticated() {
                // none auth succeeded
            } else if session.userauth_password(&ssh_config.username, password).is_ok() && session.authenticated() {
                // password auth succeeded
            } else {
                let mut handler = crate::log::remote::PasswordPromptHandler { password: password.clone() };
                session.userauth_keyboard_interactive(&ssh_config.username, &mut handler).ok()?;
            }
        }
        crate::ssh::config::AuthMethod::KeyFile(key_file) => {
            session.userauth_pubkey_file(
                &ssh_config.username,
                None,
                key_file,
                ssh_config.key_passphrase.as_deref(),
            ).ok()?;
        }
    }

    if !session.authenticated() {
        return None;
    }

    // 构建补全命令
    let escaped_partial = partial.replace("'", "'\\''");
    let compgen_cmd = if let Some(path) = current_path {
        format!("cd {} 2>/dev/null; compgen -f -d -- '{}'", path, escaped_partial)
    } else {
        format!("compgen -f -d -- '{}'", escaped_partial)
    };

    let mut channel = session.channel_session().ok()?;
    channel.exec(&compgen_cmd).ok()?;

    let mut output = String::new();
    channel.read_to_string(&mut output).ok();
    let _ = channel.wait_close();

    let completions: Vec<String> = output.lines()
        .filter(|line| !line.is_empty())
        .map(|s| s.to_string())
        .collect();

    if completions.is_empty() {
        None
    } else {
        Some(completions)
    }
}

/// 找到多个字符串的共同前缀
fn find_common_prefix(strings: &[String]) -> String {
    if strings.is_empty() {
        return String::new();
    }

    let first = &strings[0];
    let mut prefix_len = first.len();

    for s in strings.iter().skip(1) {
        let mut i = 0;
        while i < prefix_len && i < s.len() && first.as_bytes()[i] == s.as_bytes()[i] {
            i += 1;
        }
        prefix_len = i;
    }

    first[..prefix_len].to_string()
}
