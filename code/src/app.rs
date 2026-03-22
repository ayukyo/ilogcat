use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Box, ScrolledWindow, TextView, Statusbar, Label, Separator, Orientation, Button, SearchEntry};
use glib;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::log::{LogSource, LogEntry, LogLevel, CommandSource, FileWatchSource};
use crate::log::remote::SshSource;
use crate::filter::Filter;
use crate::config::Config;
use crate::ui::window::MainWindow;
use crate::config::SshServerConfig;
use std::path::PathBuf;

/// 应用状态
pub struct AppState {
    pub config: Config,
    pub main_window: Option<MainWindow>,
    pub current_source: Option<std::boxed::Box<dyn LogSource>>,
    pub is_paused: Arc<AtomicBool>,
    pub filter: Filter,
    pub log_entries: Vec<LogEntry>,
    pub filtered_entries: Vec<LogEntry>,
    pub log_count: usize,
    pub filtered_count: usize,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            config: Config::load().unwrap_or_default(),
            main_window: None,
            current_source: None,
            is_paused: Arc::new(AtomicBool::new(false)),
            filter: Filter::new(),
            log_entries: Vec::new(),
            filtered_entries: Vec::new(),
            log_count: 0,
            filtered_count: 0,
        }
    }

    pub fn clear_logs(&mut self) {
        self.log_entries.clear();
        self.filtered_entries.clear();
        self.log_count = 0;
        self.filtered_count = 0;
        if let Some(ref mut window) = self.main_window {
            window.clear_logs();
        }
    }

    pub fn toggle_pause(&mut self) {
        let paused = self.is_paused.load(Ordering::SeqCst);
        self.is_paused.store(!paused, Ordering::SeqCst);
    }

    pub fn append_log_entry(&mut self, entry: LogEntry) {
        self.log_entries.push(entry.clone());
        self.log_count = self.log_entries.len();

        if self.filter.matches(&entry) {
            self.filtered_entries.push(entry.clone());
            self.filtered_count = self.filtered_entries.len();

            if let Some(ref mut window) = self.main_window {
                window.append_log_entry(&entry);
            }
        }

        if self.log_entries.len() > 100000 {
            self.log_entries.drain(0..10000);
        }
    }

    pub fn stop_current_source(&mut self) {
        if let Some(ref mut source) = self.current_source {
            let _ = source.stop();
        }
        self.current_source = None;
    }
    
    pub fn is_source_running(&self) -> bool {
        self.current_source.as_ref().map(|s| s.is_running()).unwrap_or(false)
    }

    pub fn start_command_source(&mut self, command: &str, args: &[&str]) -> anyhow::Result<()> {
        self.stop_current_source();
        
        let mut source = CommandSource::new(command.to_string());
        if !args.is_empty() {
            source = CommandSource::with_args(
                command.to_string(),
                args.iter().map(|s| s.to_string()).collect()
            );
        }
        
        source.start()?;
        self.current_source = Some(std::boxed::Box::new(source));
        Ok(())
    }

    pub fn start_file_watch_source(&mut self, path: PathBuf) -> anyhow::Result<()> {
        self.stop_current_source();
        
        let mut source = FileWatchSource::new(path);
        source.start()?;
        self.current_source = Some(std::boxed::Box::new(source));
        Ok(())
    }

    pub fn start_ssh_source(&mut self, config: SshServerConfig, command: &str) -> anyhow::Result<()> {
        self.stop_current_source();
        
        let mut source = SshSource::new(config, command.to_string());
        source.start()?;
        self.current_source = Some(std::boxed::Box::new(source));
        Ok(())
    }
}

pub fn build_ui(app: &Application, state: Rc<RefCell<AppState>>) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("iLogCat - Linux Log Viewer")
        .default_width(1200)
        .default_height(800)
        .build();

    let vbox = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(6)
        .margin_top(6)
        .margin_bottom(6)
        .margin_start(6)
        .margin_end(6)
        .build();

    let toolbar = create_toolbar(state.clone(), &window);
    vbox.append(&toolbar);

    let filter_bar = create_filter_bar(state.clone());
    vbox.append(&filter_bar);

    let scrolled = ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .build();

    let text_view = TextView::builder()
        .editable(false)
        .monospace(true)
        .wrap_mode(gtk4::WrapMode::WordChar)
        .build();

    let buffer = text_view.buffer();
    setup_log_tags(&buffer);

    scrolled.set_child(Some(&text_view));
    vbox.append(&scrolled);

    let status_bar = Statusbar::new();
    let context_id = status_bar.context_id("main");
    status_bar.push(context_id, "Ready - Select a log source to begin");
    vbox.append(&status_bar);

    window.set_child(Some(&vbox));

    let main_window = MainWindow::from_widgets(
        window.clone(),
        text_view,
        buffer,
        status_bar,
    );
    
    {
        let mut state_ref = state.borrow_mut();
        state_ref.main_window = Some(main_window);
    }

    let state_clone = state.clone();
    glib::timeout_add_local(Duration::from_millis(100), move || {
        refresh_logs(state_clone.clone());
        glib::ControlFlow::Continue
    });

    window.present();
}

fn refresh_logs(state: Rc<RefCell<AppState>>) -> glib::ControlFlow {
    // 先检查是否暂停
    {
        let state_ref = state.borrow();
        if state_ref.is_paused.load(Ordering::SeqCst) {
            return glib::ControlFlow::Continue;
        }
    }

    // 收集日志条目
    let entries: Vec<LogEntry> = {
        let mut state_ref = state.borrow_mut();
        if let Some(ref mut source) = state_ref.current_source {
            let mut entries = Vec::new();
            while let Some(entry) = source.try_recv() {
                entries.push(entry);
            }
            entries
        } else {
            Vec::new()
        }
    };

    // 处理日志条目
    if !entries.is_empty() {
        let mut state_ref = state.borrow_mut();
        for entry in entries {
            state_ref.append_log_entry(entry);
        }
    }
    
    glib::ControlFlow::Continue
}

fn create_toolbar(state: Rc<RefCell<AppState>>, window: &ApplicationWindow) -> Box {
    let toolbar = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(6)
        .build();

    let source_label = Label::new(Some("Source:"));
    toolbar.append(&source_label);

    let source_combo = gtk4::DropDown::from_strings(&[
        "Local: dmesg",
        "Local: journalctl",
        "File...",
        "SSH...",
    ]);
    source_combo.set_selected(0);
    toolbar.append(&source_combo);

    let sep = Separator::new(Orientation::Vertical);
    toolbar.append(&sep);

    let level_label = Label::new(Some("Min Level:"));
    toolbar.append(&level_label);

    let level_combo = gtk4::DropDown::from_strings(&[
        "Verbose", "Debug", "Info", "Warn", "Error", "Fatal",
    ]);
    level_combo.set_selected(2);
    toolbar.append(&level_combo);

    let sep2 = Separator::new(Orientation::Vertical);
    toolbar.append(&sep2);

    let clear_btn = Button::builder()
        .label("Clear")
        .tooltip_text("Clear all logs (Ctrl+L)")
        .build();
    toolbar.append(&clear_btn);

    let pause_btn = Button::builder()
        .label("Pause")
        .tooltip_text("Pause/Resume log stream (Ctrl+S)")
        .build();
    toolbar.append(&pause_btn);

    let window_clone = window.clone();
    source_combo.connect_selected_notify(move |combo| {
        let idx = combo.selected();
        match idx {
            0 => { /* dmesg */ }
            1 => { /* journalctl */ }
            2 => { /* File */ }
            3 => { /* SSH */ }
            _ => {}
        }
    });

    let state_clone = state.clone();
    clear_btn.connect_clicked(move |_| {
        let mut state_ref = state_clone.borrow_mut();
        state_ref.clear_logs();
    });

    let state_clone = state.clone();
    let pause_btn_clone = pause_btn.clone();
    pause_btn.connect_clicked(move |_| {
        let mut state_ref = state_clone.borrow_mut();
        state_ref.toggle_pause();
        let paused = state_ref.is_paused.load(Ordering::SeqCst);
        pause_btn_clone.set_label(if paused { "Resume" } else { "Pause" });
    });

    toolbar
}

fn create_filter_bar(state: Rc<RefCell<AppState>>) -> Box {
    let filter_bar = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(6)
        .build();

    let search_entry = SearchEntry::builder()
        .placeholder_text("Filter logs...")
        .hexpand(true)
        .build();
    filter_bar.append(&search_entry);

    let add_filter_btn = Button::builder()
        .label("+ Filter")
        .build();
    filter_bar.append(&add_filter_btn);

    filter_bar
}

fn setup_log_tags(buffer: &gtk4::TextBuffer) {
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
