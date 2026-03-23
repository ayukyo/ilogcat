use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Notebook, Statusbar, Label, Separator, Orientation, Button, SearchEntry};
use gtk4::gdk::ModifierType;
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
use crate::ui::tabs::{TabManager, LogTab};
use crate::config::SshServerConfig;

/// 应用状态（支持多标签页）
pub struct AppState {
    pub config: Config,
    pub tab_manager: Option<Rc<RefCell<TabManager>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            config: Config::load().unwrap_or_default(),
            tab_manager: None,
        }
    }
}

pub fn build_ui(app: &Application, state: Rc<RefCell<AppState>>) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("iLogCat - Linux Log Viewer")
        .default_width(1200)
        .default_height(800)
        .build();

    let vbox = gtk4::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(6)
        .margin_top(6)
        .margin_bottom(6)
        .margin_start(6)
        .margin_end(6)
        .build();

    // 创建工具栏
    let toolbar = create_toolbar(state.clone(), &window);
    vbox.append(&toolbar);

    // 创建过滤栏
    let filter_bar = create_filter_bar(state.clone());
    vbox.append(&filter_bar);

    // 创建Notebook（标签页容器）
    let notebook = Notebook::builder()
        .hexpand(true)
        .vexpand(true)
        .build();
    
    // 初始化标签页管理器
    let tab_manager = Rc::new(RefCell::new(TabManager::new(notebook.clone())));
    {
        let mut state_ref = state.borrow_mut();
        state_ref.tab_manager = Some(tab_manager.clone());
    }
    
    // 创建默认标签页
    tab_manager.borrow_mut().create_tab("Log 1");
    
    vbox.append(&notebook);

    // 创建状态栏
    let status_bar = Statusbar::new();
    let context_id = status_bar.context_id("main");
    status_bar.push(context_id, "Ready - Select a log source to begin");
    vbox.append(&status_bar);

    window.set_child(Some(&vbox));

    // 设置快捷键
    setup_shortcuts(&window, state.clone(), tab_manager.clone());

    // 启动日志刷新定时器
    let state_clone = state.clone();
    glib::timeout_add_local(Duration::from_millis(100), move || {
        refresh_logs(state_clone.clone());
        glib::ControlFlow::Continue
    });

    window.present();
}

/// 设置键盘快捷键
fn setup_shortcuts(window: &ApplicationWindow, state: Rc<RefCell<AppState>>, tab_manager: Rc<RefCell<TabManager>>) {
    let key_controller = gtk4::EventControllerKey::new();
    
    key_controller.connect_key_pressed(move |_, key, _keycode, modifiers| {
        // Ctrl+Tab - 切换到下一个标签页
        if modifiers.contains(ModifierType::CONTROL_MASK) && key == gtk4::gdk::Key::Tab {
            tab_manager.borrow().next_tab();
            return glib::Propagation::Stop;
        }
        
        // Ctrl+Shift+Tab - 切换到上一个标签页
        if modifiers.contains(ModifierType::CONTROL_MASK | ModifierType::SHIFT_MASK) 
            && key == gtk4::gdk::Key::ISO_Left_Tab {
            tab_manager.borrow().prev_tab();
            return glib::Propagation::Stop;
        }
        
        // Ctrl+T - 新建标签页
        if modifiers.contains(ModifierType::CONTROL_MASK) 
            && (key == gtk4::gdk::Key::t || key == gtk4::gdk::Key::T) {
            let count = tab_manager.borrow().tab_count();
            tab_manager.borrow_mut().create_tab(&format!("Log {}", count + 1));
            return glib::Propagation::Stop;
        }
        
        // Ctrl+W - 关闭当前标签页
        if modifiers.contains(ModifierType::CONTROL_MASK) 
            && (key == gtk4::gdk::Key::w || key == gtk4::gdk::Key::W) {
            tab_manager.borrow_mut().close_current_tab();
            return glib::Propagation::Stop;
        }
        
        // Ctrl+L - 清除当前标签页日志
        if modifiers.contains(ModifierType::CONTROL_MASK) 
            && (key == gtk4::gdk::Key::l || key == gtk4::gdk::Key::L) {
            if let Some(tab) = tab_manager.borrow().current_tab() {
                tab.borrow_mut().clear_logs();
            }
            return glib::Propagation::Stop;
        }
        
        // Ctrl+S - 暂停/恢复当前标签页
        if modifiers.contains(ModifierType::CONTROL_MASK) 
            && (key == gtk4::gdk::Key::s || key == gtk4::gdk::Key::S) {
            if let Some(tab) = tab_manager.borrow().current_tab() {
                tab.borrow_mut().toggle_pause();
            }
            return glib::Propagation::Stop;
        }
        
        glib::Propagation::Proceed
    });
    
    window.add_controller(key_controller);
}

fn refresh_logs(state: Rc<RefCell<AppState>>) -> glib::ControlFlow {
    let tab_manager = {
        let state_ref = state.borrow();
        state_ref.tab_manager.clone()
    };
    
    if let Some(tm) = tab_manager {
        if let Some(tab) = tm.borrow().current_tab() {
            // 检查是否暂停
            if tab.borrow().is_paused() {
                return glib::ControlFlow::Continue;
            }
            
            // 收集日志条目
            let entries: Vec<LogEntry> = {
                let mut tab_ref = tab.borrow_mut();
                if let Some(ref mut source) = tab_ref.current_source {
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
                let mut tab_ref = tab.borrow_mut();
                for entry in entries {
                    tab_ref.append_log_entry(&entry);
                }
            }
        }
    }
    
    glib::ControlFlow::Continue
}

fn create_toolbar(state: Rc<RefCell<AppState>>, window: &ApplicationWindow) -> Box {
    let toolbar = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(6)
        .build();

    // 新建标签页按钮
    let new_tab_btn = Button::builder()
        .label("+ New Tab")
        .tooltip_text("Create new log tab (Ctrl+T)")
        .build();
    toolbar.append(&new_tab_btn);

    let sep = Separator::new(Orientation::Vertical);
    toolbar.append(&sep);

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

    let sep2 = Separator::new(Orientation::Vertical);
    toolbar.append(&sep2);

    let level_label = Label::new(Some("Min Level:"));
    toolbar.append(&level_label);

    let level_combo = gtk4::DropDown::from_strings(&[
        "Verbose", "Debug", "Info", "Warn", "Error", "Fatal",
    ]);
    level_combo.set_selected(2);
    toolbar.append(&level_combo);

    let sep3 = Separator::new(Orientation::Vertical);
    toolbar.append(&sep3);

    let clear_btn = Button::builder()
        .label("Clear")
        .tooltip_text("Clear current tab logs (Ctrl+L)")
        .build();
    toolbar.append(&clear_btn);

    let pause_btn = Button::builder()
        .label("Pause")
        .tooltip_text("Pause/Resume log stream (Ctrl+S)")
        .build();
    toolbar.append(&pause_btn);

    // 新建标签页按钮事件
    let state_clone = state.clone();
    new_tab_btn.connect_clicked(move |_| {
        if let Some(ref tm) = state_clone.borrow().tab_manager {
            let count = tm.borrow().tab_count();
            tm.borrow_mut().create_tab(&format!("Log {}", count + 1));
        }
    });

    // 日志源选择事件
    let state_clone = state.clone();
    let window_clone = window.clone();
    source_combo.connect_selected_notify(move |combo| {
        let idx = combo.selected();
        let state_ref = state_clone.clone();
        let window_ref = window_clone.clone();
        
        // 获取当前标签页
        let current_tab = {
            let state = state_ref.borrow();
            if let Some(ref tm) = state.tab_manager {
                tm.borrow().current_tab()
            } else {
                None
            }
        };
        
        if current_tab.is_none() {
            return;
        }
        let tab = current_tab.unwrap();
        
        match idx {
            0 => {
                // dmesg
                tab.borrow_mut().clear_logs();
                let mut source = CommandSource::new("dmesg".to_string());
                source = CommandSource::with_args(
                    "dmesg".to_string(),
                    vec!["-w".to_string(), "--time-format=iso".to_string()]
                );
                if let Err(e) = source.start() {
                    crate::ui::dialogs::show_error_dialog(&window_ref, "Failed to Start dmesg", &e.to_string());
                } else {
                    tab.borrow_mut().set_source(std::boxed::Box::new(source));
                }
            }
            1 => {
                // journalctl
                tab.borrow_mut().clear_logs();
                let mut source = CommandSource::with_args(
                    "journalctl".to_string(),
                    vec!["-f".to_string(), "-o".to_string(), "short-iso".to_string()]
                );
                if let Err(e) = source.start() {
                    crate::ui::dialogs::show_error_dialog(&window_ref, "Failed to Start journalctl", &e.to_string());
                } else {
                    tab.borrow_mut().set_source(std::boxed::Box::new(source));
                }
            }
            2 => {
                // File - 显示文件选择对话框
                let tab_ref = tab.clone();
                crate::ui::dialogs::show_file_dialog(&window_ref, move |path| {
                    tab_ref.borrow_mut().clear_logs();
                    let mut source = FileWatchSource::new(path);
                    if let Err(e) = source.start() {
                        eprintln!("Failed to start file watch: {}", e);
                    } else {
                        tab_ref.borrow_mut().set_source(std::boxed::Box::new(source));
                    }
                });
            }
            3 => {
                // SSH - 显示SSH连接对话框
                let tab_ref = tab.clone();
                let window_ref = window_ref.clone();
                let state_ref = state_ref.clone();
                crate::ui::dialogs::show_ssh_dialog(&window_ref, move |ssh_config| {
                    tab_ref.borrow_mut().clear_logs();
                    
                    // 保存SSH配置
                    {
                        let mut state = state_ref.borrow_mut();
                        state.config.add_ssh_server(crate::config::SshServerConfig::from(ssh_config.clone()));
                        let _ = state.config.save();
                    }
                    
                    // 创建SSH源
                    let ssh_config = crate::config::SshServerConfig::from(ssh_config);
                    let mut source = SshSource::new(ssh_config, "journalctl -f -o short-iso".to_string());
                    if let Err(e) = source.start() {
                        eprintln!("Failed to start SSH source: {}", e);
                    } else {
                        tab_ref.borrow_mut().set_source(std::boxed::Box::new(source));
                    }
                });
            }
            _ => {}
        }
    });

    // 日志级别选择事件
    let state_clone = state.clone();
    level_combo.connect_selected_notify(move |combo| {
        let idx = combo.selected();
        let min_level = match idx {
            0 => LogLevel::Verbose,
            1 => LogLevel::Debug,
            2 => LogLevel::Info,
            3 => LogLevel::Warn,
            4 => LogLevel::Error,
            5 => LogLevel::Fatal,
            _ => LogLevel::Verbose,
        };
        
        // 获取当前标签页并设置级别
        if let Some(ref tm) = state_clone.borrow().tab_manager {
            if let Some(tab) = tm.borrow().current_tab() {
                tab.borrow_mut().set_min_level(min_level);
            }
        }
    });

    // 清除按钮事件
    let state_clone = state.clone();
    clear_btn.connect_clicked(move |_| {
        if let Some(ref tm) = state_clone.borrow().tab_manager {
            if let Some(tab) = tm.borrow().current_tab() {
                tab.borrow_mut().clear_logs();
            }
        }
    });

    // 暂停按钮事件
    let state_clone = state.clone();
    let pause_btn_clone = pause_btn.clone();
    pause_btn.connect_clicked(move |_| {
        if let Some(ref tm) = state_clone.borrow().tab_manager {
            if let Some(tab) = tm.borrow().current_tab() {
                tab.borrow_mut().toggle_pause();
                let paused = tab.borrow().is_paused();
                pause_btn_clone.set_label(if paused { "Resume" } else { "Pause" });
            }
        }
    });

    toolbar
}

fn create_filter_bar(_state: Rc<RefCell<AppState>>) -> gtk4::Box {
    let filter_bar = gtk4::Box::builder()
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