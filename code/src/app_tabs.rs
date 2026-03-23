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
use crate::ui::TabSourceType as SourceType;
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

    // 启动标签页标题刷新定时器（每秒更新 SSH 连接状态）
    let state_clone = state.clone();
    glib::timeout_add_local(Duration::from_secs(1), move || {
        refresh_tab_titles(state_clone.clone());
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

/// 刷新所有标签页标题（更新 SSH 连接状态）
fn refresh_tab_titles(state: Rc<RefCell<AppState>>) -> glib::ControlFlow {
    let tab_manager = {
        let state_ref = state.borrow();
        state_ref.tab_manager.clone()
    };
    
    if let Some(tm) = tab_manager {
        tm.borrow().refresh_all_tab_titles();
    }
    
    glib::ControlFlow::Continue
}

fn create_toolbar(state: Rc<RefCell<AppState>>, window: &ApplicationWindow) -> gtk4::Box {
    let toolbar = gtk4::Box::builder()
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
        "SSH Command...",
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

    let sep4 = Separator::new(Orientation::Vertical);
    toolbar.append(&sep4);

    // 设置按钮
    let settings_btn = Button::builder()
        .label("Settings")
        .tooltip_text("Export/Import settings")
        .build();
    toolbar.append(&settings_btn);

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
                tab.borrow_mut().set_source_info(SourceType::Dmesg);
                let mut source = CommandSource::new("dmesg".to_string());
                source = CommandSource::with_args(
                    "dmesg".to_string(),
                    vec!["-w".to_string(), "--time-format=iso".to_string()]
                );
                if let Err(e) = source.start() {
                    crate::ui::dialogs::show_error_dialog(&window_ref, "Failed to Start dmesg", &e.to_string());
                } else {
                    tab.borrow_mut().set_source(std::boxed::Box::new(source));
                    // 更新标签页标题
                    if let Some(ref tm) = state_ref.borrow().tab_manager {
                        tm.borrow().update_tab_title(tab.borrow().id);
                    }
                }
            }
            1 => {
                // journalctl
                tab.borrow_mut().clear_logs();
                tab.borrow_mut().set_source_info(SourceType::Journalctl);
                let mut source = CommandSource::with_args(
                    "journalctl".to_string(),
                    vec!["-f".to_string(), "-o".to_string(), "short-iso".to_string()]
                );
                if let Err(e) = source.start() {
                    crate::ui::dialogs::show_error_dialog(&window_ref, "Failed to Start journalctl", &e.to_string());
                } else {
                    tab.borrow_mut().set_source(std::boxed::Box::new(source));
                    // 更新标签页标题
                    if let Some(ref tm) = state_ref.borrow().tab_manager {
                        tm.borrow().update_tab_title(tab.borrow().id);
                    }
                }
            }
            2 => {
                // File - 显示文件选择对话框
                let tab_ref = tab.clone();
                let state_ref_clone = state_ref.clone();
                crate::ui::dialogs::show_file_dialog(&window_ref, move |path| {
                    tab_ref.borrow_mut().clear_logs();
                    tab_ref.borrow_mut().set_source_info(SourceType::File(path.clone()));
                    let mut source = FileWatchSource::new(path);
                    if let Err(e) = source.start() {
                        eprintln!("Failed to start file watch: {}", e);
                    } else {
                        tab_ref.borrow_mut().set_source(std::boxed::Box::new(source));
                        // 更新标签页标题
                        if let Some(ref tm) = state_ref_clone.borrow().tab_manager {
                            let tab_id = tab_ref.borrow().id;
                            tm.borrow().update_tab_title(tab_id);
                        }
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
                    let host = ssh_config.host.clone();
                    tab_ref.borrow_mut().set_source_info(SourceType::Ssh(host.clone(), "journalctl".to_string()));
                    let mut source = SshSource::new(ssh_config, "journalctl -f -o short-iso".to_string());
                    if let Err(e) = source.start() {
                        eprintln!("Failed to start SSH source: {}", e);
                    } else {
                        tab_ref.borrow_mut().set_source(std::boxed::Box::new(source));
                        // 更新标签页标题
                        if let Some(ref tm) = state_ref.borrow().tab_manager {
                            let tab_id = tab_ref.borrow().id;
                            tm.borrow().update_tab_title(tab_id);
                        }
                    }
                });
            }
            4 => {
                // SSH Command - 显示SSH命令执行对话框
                let tab_ref = tab.clone();
                let window_ref = window_ref.clone();
                let state_ref = state_ref.clone();
                
                // 获取已保存的SSH服务器列表
                let saved_servers = state_ref.borrow().config.ssh_servers.clone();
                
                if saved_servers.is_empty() {
                    crate::ui::dialogs::show_info_dialog(&window_ref, "No Saved Servers", 
                        "Please connect to an SSH server first using 'SSH...' option.");
                } else {
                    let state_ref_clone = state_ref.clone();
                    crate::ui::dialogs::show_ssh_command_dialog(&window_ref, saved_servers, 
                        move |server_config, command| {
                            tab_ref.borrow_mut().clear_logs();
                            
                            let host = server_config.host.clone();
                            let cmd = command.clone();
                            tab_ref.borrow_mut().set_source_info(SourceType::SshCommand(host, cmd));
                            
                            // 创建SSH源执行自定义命令
                            let mut source = SshSource::new(server_config, command);
                            if let Err(e) = source.start() {
                                eprintln!("Failed to start SSH command: {}", e);
                            } else {
                                tab_ref.borrow_mut().set_source(std::boxed::Box::new(source));
                                // 更新标签页标题
                                if let Some(ref tm) = state_ref_clone.borrow().tab_manager {
                                    let tab_id = tab_ref.borrow().id;
                                    tm.borrow().update_tab_title(tab_id);
                                }
                            }
                        });
                }
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

    // 设置按钮事件 - 显示导出/导入菜单
    let state_clone = state.clone();
    let window_clone = window.clone();
    settings_btn.connect_clicked(move |_| {
        // 创建设置菜单对话框
        let dialog = gtk4::Dialog::builder()
            .title("Settings")
            .transient_for(&window_clone)
            .modal(true)
            .default_width(300)
            .build();
        
        let content = dialog.content_area();
        content.set_spacing(12);
        content.set_margin_top(12);
        content.set_margin_bottom(12);
        content.set_margin_start(12);
        content.set_margin_end(12);
        
        // 导出按钮
        let export_btn = Button::builder()
            .label("Export Settings")
            .tooltip_text("Export all settings to a file")
            .hexpand(true)
            .build();
        content.append(&export_btn);
        
        // 导入按钮
        let import_btn = Button::builder()
            .label("Import Settings")
            .tooltip_text("Import settings from a file")
            .hexpand(true)
            .build();
        content.append(&import_btn);
        
        // 关闭按钮
        dialog.add_button("Close", gtk4::ResponseType::Close);
        
        // 导出按钮事件
        let state_ref = state_clone.clone();
        let window_ref = window_clone.clone();
        export_btn.connect_clicked(move |_| {
            let state_ref = state_ref.clone();
            let window_ref = window_ref.clone();
            let window_ref_for_dialog = window_ref.clone();
            crate::ui::dialogs::show_export_settings_dialog(&window_ref_for_dialog, move |path| {
                let state = state_ref.borrow();
                match state.config.export_to(&path) {
                    Ok(_) => {
                        crate::ui::dialogs::show_info_dialog(&window_ref, "Export Successful", 
                            &format!("Settings exported to:\n{}", path.display()));
                    }
                    Err(e) => {
                        crate::ui::dialogs::show_error_dialog(&window_ref, "Export Failed", 
                            &format!("Failed to export settings:\n{}", e));
                    }
                }
            });
        });
        
        // 导入按钮事件
        let state_ref = state_clone.clone();
        let window_ref = window_clone.clone();
        import_btn.connect_clicked(move |_| {
            let state_ref = state_ref.clone();
            let window_ref = window_ref.clone();
            let window_ref_for_dialog = window_ref.clone();
            crate::ui::dialogs::show_import_settings_dialog(&window_ref_for_dialog, move |path| {
                match Config::import_from(&path) {
                    Ok(imported_config) => {
                        let mut state = state_ref.borrow_mut();
                        // 合并配置
                        state.config.merge(imported_config);
                        // 保存合并后的配置
                        if let Err(e) = state.config.save() {
                            crate::ui::dialogs::show_error_dialog(&window_ref, "Save Failed", 
                                &format!("Failed to save imported settings:\n{}", e));
                        } else {
                            crate::ui::dialogs::show_info_dialog(&window_ref, "Import Successful", 
                                "Settings imported and merged successfully.\nSome changes may require restart.");
                        }
                    }
                    Err(e) => {
                        crate::ui::dialogs::show_error_dialog(&window_ref, "Import Failed", 
                            &format!("Failed to import settings:\n{}", e));
                    }
                }
            });
        });
        
        dialog.connect_response(|dialog, _| {
            dialog.close();
        });
        
        dialog.present();
    });

    toolbar
}

fn create_filter_bar(state: Rc<RefCell<AppState>>) -> gtk4::Box {
    let filter_bar = gtk4::Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(6)
        .build();

    let search_entry = SearchEntry::builder()
        .placeholder_text("Filter logs (Enter to apply)...")
        .hexpand(true)
        .build();
    filter_bar.append(&search_entry);

    // 搜索按钮
    let search_btn = Button::builder()
        .label("Apply")
        .tooltip_text("Apply filter to current tab")
        .build();
    filter_bar.append(&search_btn);

    // 清除过滤按钮
    let clear_filter_btn = Button::builder()
        .label("Clear")
        .tooltip_text("Clear all filters")
        .build();
    filter_bar.append(&clear_filter_btn);

    // 搜索框回车事件
    let state_clone = state.clone();
    search_entry.connect_activate(move |entry| {
        let text = entry.text().to_string();
        apply_filter_to_current_tab(state_clone.clone(), &text);
    });

    // 搜索按钮事件
    let state_clone = state.clone();
    let search_entry_clone = search_entry.clone();
    search_btn.connect_clicked(move |_| {
        let text = search_entry_clone.text().to_string();
        apply_filter_to_current_tab(state_clone.clone(), &text);
    });

    // 清除过滤按钮事件
    let state_clone = state.clone();
    let search_entry_clone = search_entry.clone();
    clear_filter_btn.connect_clicked(move |_| {
        search_entry_clone.set_text("");
        clear_filter_on_current_tab(state_clone.clone());
    });

    filter_bar
}

/// 应用过滤器到当前标签页
fn apply_filter_to_current_tab(state: Rc<RefCell<AppState>>, filter_text: &str) {
    if filter_text.is_empty() {
        clear_filter_on_current_tab(state);
        return;
    }

    let tab_manager = {
        let state_ref = state.borrow();
        state_ref.tab_manager.clone()
    };

    if let Some(tm) = tab_manager {
        if let Some(tab) = tm.borrow().current_tab() {
            let mut tab_ref = tab.borrow_mut();
            
            // 清除现有关键字过滤器
            tab_ref.filter.keywords.clear();
            
            // 添加新的关键字过滤器
            let keyword_filter = crate::filter::KeywordFilter::new(
                filter_text.to_string(),
                false,  // 不区分大小写
                false,  // 不需要全词匹配
            );
            tab_ref.filter.keywords.push(keyword_filter);
            
            // 重新过滤并刷新显示
            tab_ref.filtered_entries.clear();
            for entry in &tab_ref.log_entries {
                if tab_ref.filter.matches(entry) {
                    tab_ref.filtered_entries.push(entry.clone());
                }
            }
            tab_ref.filtered_count = tab_ref.filtered_entries.len();
            tab_ref.refresh_filtered_display();
        }
    }
}

/// 清除当前标签页的过滤器
fn clear_filter_on_current_tab(state: Rc<RefCell<AppState>>) {
    let tab_manager = {
        let state_ref = state.borrow();
        state_ref.tab_manager.clone()
    };

    if let Some(tm) = tab_manager {
        if let Some(tab) = tm.borrow().current_tab() {
            let mut tab_ref = tab.borrow_mut();
            
            // 清除关键字过滤器
            tab_ref.filter.keywords.clear();
            tab_ref.filter.regex = None;
            
            // 重新显示所有日志
            tab_ref.filtered_entries = tab_ref.log_entries.clone();
            tab_ref.filtered_count = tab_ref.filtered_entries.len();
            tab_ref.refresh_filtered_display();
        }
    }
}