use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Notebook, Statusbar, Label, Separator, Orientation, Button, SearchEntry, Paned, ScrolledWindow, ListBox, ListBoxRow, Entry, Popover};
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
use crate::ui::{SearchBar, SearchManager};
use crate::ssh::config::SshConfig;
use crate::stats::{StatsPanel, StatsDialog, LogStatistics};
use crate::export::{ExportManager, ExportFormat};
use crate::filter::enhanced::{EnhancedRegexFilter, FilterDialog};
use crate::i18n::{t, I18nKey};

/// 应用主题
fn apply_theme(theme: &str) {
    let display = gtk4::gdk::Display::default();
    if let Some(display) = display {
        let provider = gtk4::CssProvider::new();
        
        if theme == "dark" {
            // 暗色主题 CSS
            let css = "
                window {
                    background-color: #1e1e1e;
                    color: #ffffff;
                }
                textview {
                    background-color: #1e1e1e;
                    color: #ffffff;
                }
                button {
                    background-color: #3c3c3c;
                    color: #ffffff;
                }
                entry {
                    background-color: #2d2d2d;
                    color: #ffffff;
                }
                dropdown {
                    background-color: #2d2d2d;
                    color: #ffffff;
                }
            ";
            provider.load_from_data(css);
        } else {
            // 浅色主题 CSS
            let css = "
                window {
                    background-color: #ffffff;
                    color: #000000;
                }
                textview {
                    background-color: #ffffff;
                    color: #000000;
                }
                button {
                    background-color: #f0f0f0;
                    color: #000000;
                }
                entry {
                    background-color: #ffffff;
                    color: #000000;
                }
                dropdown {
                    background-color: #ffffff;
                    color: #000000;
                }
            ";
            provider.load_from_data(css);
        }
        
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

/// 应用状态（支持多标签页）
pub struct AppState {
    pub config: Config,
    pub tab_manager: Option<Rc<RefCell<TabManager>>>,
    pub search_bar: Option<Rc<RefCell<SearchBar>>>,
    pub search_manager: Rc<RefCell<SearchManager>>,
    pub filter_entry: Option<gtk4::SearchEntry>,  // 过滤输入框
    pub export_manager: Rc<RefCell<ExportManager>>,
    pub enhanced_filter: Rc<RefCell<EnhancedRegexFilter>>,
    pub command_shortcuts_list: Option<ListBox>,  // 命令快捷方式列表
}

impl AppState {
    pub fn new() -> Self {
        Self {
            config: Config::load().unwrap_or_default(),
            tab_manager: None,
            search_bar: None,
            search_manager: Rc::new(RefCell::new(SearchManager::new())),
            filter_entry: None,
            export_manager: Rc::new(RefCell::new(ExportManager::new())),
            enhanced_filter: Rc::new(RefCell::new(EnhancedRegexFilter::new())),
            command_shortcuts_list: None,
        }
    }
}

pub fn build_ui(app: &Application, state: Rc<RefCell<AppState>>) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title(&t(I18nKey::AppTitle))
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
    let (filter_bar, filter_entry) = create_filter_bar(state.clone(), &window);
    vbox.append(&filter_bar);

    // 保存过滤输入框到状态
    {
        let mut state_ref = state.borrow_mut();
        state_ref.filter_entry = Some(filter_entry.clone());
    }

    // 创建搜索栏（默认隐藏）
    let search_bar = Rc::new(RefCell::new(SearchBar::new()));
    search_bar.borrow().widget().set_visible(false);
    vbox.append(search_bar.borrow().widget());

    {
        let mut state_ref = state.borrow_mut();
        state_ref.search_bar = Some(search_bar.clone());
    }

    // 创建 Paned 布局：左侧命令快捷方式，右侧日志窗口
    let paned = Paned::builder()
        .orientation(Orientation::Horizontal)
        .hexpand(true)
        .vexpand(true)
        .build();

    // 创建左侧命令快捷方式侧边栏
    let (sidebar, shortcuts_list) = create_command_sidebar(state.clone());
    paned.set_start_child(Some(&sidebar));
    paned.set_shrink_start_child(false);  // 不允许收缩侧边栏
    paned.set_resize_start_child(true);   // 允许调整大小
    paned.set_position(200);  // 默认宽度

    // 保存快捷方式列表引用
    {
        let mut state_ref = state.borrow_mut();
        state_ref.command_shortcuts_list = Some(shortcuts_list.clone());
    }

    // 创建Notebook（标签页容器）
    let notebook = Notebook::builder()
        .hexpand(true)
        .vexpand(true)
        .scrollable(true)  // 启用滚动
        .build();

    // 设置标签页位置为顶部
    notebook.set_tab_pos(gtk4::PositionType::Top);

    // 将 notebook 添加到 paned 的右侧
    paned.set_end_child(Some(&notebook));

    // 初始化标签页管理器
    let tab_manager = Rc::new(RefCell::new(TabManager::new(notebook.clone())));

    // 从配置加载命令历史
    {
        let history = state.borrow().config.command_history.clone();
        tab_manager.borrow_mut().set_command_history(history);
    }

    {
        let mut state_ref = state.borrow_mut();
        state_ref.tab_manager = Some(tab_manager.clone());
    }

    // 创建默认标签页
    let tab_name = format!("{} 1", t(I18nKey::TabName));
    let first_tab = tab_manager.borrow_mut().create_tab(&tab_name);

    // 初始化搜索标签
    state.borrow().search_manager.borrow_mut().setup_tags(&first_tab.borrow().text_buffer);

    vbox.append(&paned);

    // 标签页切换事件 - 同步过滤输入框
    let state_for_switch = state.clone();
    let last_page: Rc<RefCell<Option<u32>>> = Rc::new(RefCell::new(None));

    glib::timeout_add_local(std::time::Duration::from_millis(200), move || {
        // 只在标签页切换时同步过滤输入框
        let tab_manager = {
            let state_ref = state_for_switch.borrow();
            state_ref.tab_manager.clone()
        };

        if let Some(ref tm) = tab_manager {
            let current_page = tm.borrow().notebook().current_page();

            let should_update = {
                let mut last = last_page.borrow_mut();
                if *last != current_page {
                    *last = current_page;
                    true
                } else {
                    false
                }
            };

            if should_update {
                let filter_entry = {
                    let state_ref = state_for_switch.borrow();
                    state_ref.filter_entry.clone()
                };

                if let Some(ref filter_entry) = filter_entry {
                    if let Some(tab) = tm.borrow().current_tab() {
                        let filter_text = tab.borrow().get_filter_text().to_string();
                        filter_entry.set_text(&filter_text);
                    }
                }
            }
        }
        glib::ControlFlow::Continue
    });

    window.set_child(Some(&vbox));

    // 设置搜索栏事件
    setup_search_bar_events(state.clone(), search_bar.clone(), tab_manager.clone());

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
    let tab_manager_clone = tab_manager.clone();
    glib::timeout_add_local(Duration::from_secs(1), move || {
        refresh_tab_titles(state_clone.clone());

        // 同步命令历史到配置（每秒检查一次）
        let history = tab_manager_clone.borrow().get_command_history();
        let mut state_ref = state_clone.borrow_mut();
        if state_ref.config.command_history != history {
            state_ref.config.command_history = history;
            let _ = state_ref.config.save();
        }

        glib::ControlFlow::Continue
    });

    window.present();
}

/// 设置搜索栏事件
fn setup_search_bar_events(
    state: Rc<RefCell<AppState>>,
    search_bar: Rc<RefCell<SearchBar>>,
    tab_manager: Rc<RefCell<TabManager>>,
) {
    // 搜索文本变化事件
    let state_clone = state.clone();
    let tab_manager_clone = tab_manager.clone();
    search_bar.borrow().connect_search_changed(move |text| {
        if let Some(tab) = tab_manager_clone.borrow().current_tab() {
            let buffer = tab.borrow().text_buffer.clone();
            let count = state_clone.borrow().search_manager.borrow_mut().search(&buffer, text, false);
            
            if let Some(ref sb) = state_clone.borrow().search_bar {
                sb.borrow().set_total_matches(count);
                sb.borrow().set_current_match(0);
                
                // 滚动到第一个匹配项
                if count > 0 {
                    if let Some(iter) = state_clone.borrow().search_manager.borrow_mut().navigate_to_match(&buffer, 0) {
                        let mark = buffer.create_mark(None, &iter, true);
                        tab.borrow().text_view.scroll_to_mark(&mark, 0.0, true, 0.0, 0.5);
                    }
                }
            }
        }
    });

    // 上一个按钮
    let state_clone = state.clone();
    let tab_manager_clone = tab_manager.clone();
    search_bar.borrow().connect_prev_clicked(move || {
        if let Some(ref sb) = state_clone.borrow().search_bar {
            let current = sb.borrow().current_match();
            let total = sb.borrow().total_matches();
            if total > 0 {
                let new_index = if current > 0 { current - 1 } else { total - 1 };
                sb.borrow().set_current_match(new_index);
                
                if let Some(tab) = tab_manager_clone.borrow().current_tab() {
                    let buffer = tab.borrow().text_buffer.clone();
                    if let Some(iter) = state_clone.borrow().search_manager.borrow_mut().navigate_to_match(&buffer, new_index) {
                        let mark = buffer.create_mark(None, &iter, true);
                        tab.borrow().text_view.scroll_to_mark(&mark, 0.0, true, 0.0, 0.5);
                    }
                }
            }
        }
    });

    // 下一个按钮
    let state_clone = state.clone();
    let tab_manager_clone = tab_manager.clone();
    search_bar.borrow().connect_next_clicked(move || {
        if let Some(ref sb) = state_clone.borrow().search_bar {
            let current = sb.borrow().current_match();
            let total = sb.borrow().total_matches();
            if total > 0 {
                let new_index = (current + 1) % total;
                sb.borrow().set_current_match(new_index);
                
                if let Some(tab) = tab_manager_clone.borrow().current_tab() {
                    let buffer = tab.borrow().text_buffer.clone();
                    if let Some(iter) = state_clone.borrow().search_manager.borrow_mut().navigate_to_match(&buffer, new_index) {
                        let mark = buffer.create_mark(None, &iter, true);
                        tab.borrow().text_view.scroll_to_mark(&mark, 0.0, true, 0.0, 0.5);
                    }
                }
            }
        }
    });

    // 关闭按钮
    let state_clone = state.clone();
    let tab_manager_clone = tab_manager.clone();
    search_bar.borrow().connect_close_clicked(move || {
        if let Some(ref sb) = state_clone.borrow().search_bar {
            sb.borrow().hide();
            sb.borrow().clear();
        }
        if let Some(tab) = tab_manager_clone.borrow().current_tab() {
            let buffer = tab.borrow().text_buffer.clone();
            state_clone.borrow().search_manager.borrow_mut().clear_highlights(&buffer);
        }
        state_clone.borrow().search_manager.borrow_mut().clear();
    });

    // 回车键 - 跳转到下一个匹配
    let state_clone = state.clone();
    let tab_manager_clone = tab_manager.clone();
    search_bar.borrow().connect_activate(move || {
        if let Some(ref sb) = state_clone.borrow().search_bar {
            let current = sb.borrow().current_match();
            let total = sb.borrow().total_matches();
            if total > 0 {
                let new_index = (current + 1) % total;
                sb.borrow().set_current_match(new_index);
                
                if let Some(tab) = tab_manager_clone.borrow().current_tab() {
                    let buffer = tab.borrow().text_buffer.clone();
                    if let Some(iter) = state_clone.borrow().search_manager.borrow_mut().navigate_to_match(&buffer, new_index) {
                        let mark = buffer.create_mark(None, &iter, true);
                        tab.borrow().text_view.scroll_to_mark(&mark, 0.0, true, 0.0, 0.5);
                    }
                }
            }
        }
    });
}

/// 设置键盘快捷键
fn setup_shortcuts(window: &ApplicationWindow, state: Rc<RefCell<AppState>>, tab_manager: Rc<RefCell<TabManager>>) {
    let key_controller = gtk4::EventControllerKey::new();
    
    key_controller.connect_key_pressed(move |_, key, _keycode, modifiers| {
        // Ctrl+F - 打开搜索栏
        if modifiers.contains(ModifierType::CONTROL_MASK) 
            && (key == gtk4::gdk::Key::f || key == gtk4::gdk::Key::F) {
            if let Some(ref sb) = state.borrow().search_bar {
                sb.borrow().show();
            }
            return glib::Propagation::Stop;
        }
        
        // Esc - 关闭搜索栏
        if key == gtk4::gdk::Key::Escape {
            if let Some(ref sb) = state.borrow().search_bar {
                if sb.borrow().is_visible() {
                    sb.borrow().hide();
                    sb.borrow().clear();
                    if let Some(tab) = tab_manager.borrow().current_tab() {
                        let buffer = tab.borrow().text_buffer.clone();
                        state.borrow().search_manager.borrow_mut().clear_highlights(&buffer);
                    }
                    state.borrow().search_manager.borrow_mut().clear();
                    return glib::Propagation::Stop;
                }
            }
        }
        
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
        
        // Ctrl+B - 添加书签
        if modifiers.contains(ModifierType::CONTROL_MASK) 
            && (key == gtk4::gdk::Key::b || key == gtk4::gdk::Key::B) {
            // 触发书签按钮点击
            // 这里简化处理，实际应该调用书签添加逻辑
            return glib::Propagation::Stop;
        }
        
        // Ctrl+Shift+B - 查看书签
        if modifiers.contains(ModifierType::CONTROL_MASK | ModifierType::SHIFT_MASK) 
            && (key == gtk4::gdk::Key::b || key == gtk4::gdk::Key::B) {
            // 触发查看书签对话框
            return glib::Propagation::Stop;
        }
        
        // Ctrl+Shift+E - 导出日志
        if modifiers.contains(ModifierType::CONTROL_MASK | ModifierType::SHIFT_MASK) 
            && (key == gtk4::gdk::Key::e || key == gtk4::gdk::Key::E) {
            // 触发导出对话框
            return glib::Propagation::Stop;
        }
        
        // Ctrl+Shift+F - 打开高级过滤对话框
        if modifiers.contains(ModifierType::CONTROL_MASK | ModifierType::SHIFT_MASK) 
            && (key == gtk4::gdk::Key::f || key == gtk4::gdk::Key::F) {
            // 触发高级过滤对话框
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

            // 检查是否是SSH终端模式
            let is_ssh_terminal = tab.borrow().ssh_config.is_some();

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

                // 检查是否是cd命令后的pwd输出（更新路径）
                let pending_cd = tab_ref.pending_cd;
                if pending_cd {
                    // 最后一行是路径
                    if let Some(last_entry) = entries.last() {
                        let msg = last_entry.message.trim();
                        if msg.starts_with('/') {
                            tab_ref.set_current_path(msg.to_string());
                            tab_ref.append_terminal_output(&format!("-> {}", msg));
                        }
                    }
                    tab_ref.pending_cd = false;
                }

                // 所有日志都通过过滤器（包括SSH终端模式）
                for entry in entries {
                    tab_ref.append_log_entry(&entry);
                }
            }
        }
    }

    glib::ControlFlow::Continue
}

/// 刷新所有标签页标题（更新 SSH 连接状态）并检查自动重连
fn refresh_tab_titles(state: Rc<RefCell<AppState>>) -> glib::ControlFlow {
    let tab_manager = {
        let state_ref = state.borrow();
        state_ref.tab_manager.clone()
    };

    if let Some(tm) = tab_manager {
        tm.borrow().refresh_all_tab_titles();

        // 检查是否需要SSH自动重连
        let tabs_to_reconnect: Vec<(usize, SshConfig, Option<String>)> = {
            let tabs = tm.borrow().tabs();
            let tabs_ref = tabs.borrow();
            tabs_ref.iter()
                .filter_map(|tab| {
                    let tab_ref = tab.borrow();
                    if tab_ref.should_reconnect_ssh() {
                        if let Some(ref ssh_config) = tab_ref.ssh_config {
                            // 获取当前命令（如果有）
                            let cmd: Option<String> = match &tab_ref.source_type {
                                SourceType::SshCommand(_, cmd) => Some(cmd.clone()),
                                SourceType::Ssh(_, cmd) => Some(cmd.clone()),
                                _ => None,
                            };
                            return Some((tab_ref.id, ssh_config.clone(), cmd));
                        }
                    }
                    None
                })
                .collect()
        };

        // 执行重连
        for (tab_id, ssh_config, cmd) in tabs_to_reconnect {
            if let Some(tab) = tm.borrow().get_tab_by_id(tab_id) {
                tab.borrow().start_reconnect();
                tab.borrow_mut().append_terminal_output("SSH 连接断开，正在尝试重连...");

                let tab_clone = tab.clone();
                let tm_clone = tm.clone();

                // 使用 glib 定时器延迟 2 秒后重连
                glib::timeout_add_local(std::time::Duration::from_secs(2), move || {
                    // 尝试重新连接
                    let test_cmd: String = cmd.clone().unwrap_or_else(|| "echo connected".to_string());
                    let mut source = SshSource::new(ssh_config.clone(), test_cmd);

                    match source.start() {
                        Ok(_) => {
                            // 连接成功
                            source.stop().ok();
                            tab_clone.borrow().finish_reconnect(true);
                            tab_clone.borrow_mut().set_source(std::boxed::Box::new(source));

                            if let Some(ref cmd_str) = cmd {
                                tab_clone.borrow_mut().append_terminal_output(&format!("重连成功，继续执行: {}", cmd_str));
                            } else {
                                tab_clone.borrow_mut().append_terminal_output("重连成功");
                            }

                            // 更新标签页标题
                            tm_clone.borrow().update_tab_title(tab_id);
                        }
                        Err(e) => {
                            tab_clone.borrow().finish_reconnect(false);
                            tab_clone.borrow_mut().append_terminal_output(&format!("重连失败: {}", e));
                            tm_clone.borrow().update_tab_title(tab_id);
                        }
                    }

                    glib::ControlFlow::Break
                });
            }
        }
    }

    glib::ControlFlow::Continue
}

fn create_toolbar(state: Rc<RefCell<AppState>>, window: &ApplicationWindow) -> gtk4::Box {
    let toolbar = gtk4::Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(6)
        .build();

    // 新建标签页按钮（本地日志源）
    let new_tab_btn = Button::builder()
        .label(&t(I18nKey::ButtonNewTab))
        .tooltip_text(&t(I18nKey::TooltipNewTab))
        .build();
    toolbar.append(&new_tab_btn);

    // SSH 连接按钮
    let ssh_btn = Button::builder()
        .label(&format!("📡 {}", t(I18nKey::SourceSsh).replace("...", "")))
        .tooltip_text(&t(I18nKey::DialogSshConnection))
        .build();
    toolbar.append(&ssh_btn);

    let sep = Separator::new(Orientation::Vertical);
    toolbar.append(&sep);

    let level_label = Label::new(Some(&t(I18nKey::LabelMinLevel)));
    toolbar.append(&level_label);

    let level_combo = gtk4::DropDown::from_strings(&[
        &t(I18nKey::LevelVerbose),
        &t(I18nKey::LevelDebug),
        &t(I18nKey::LevelInfo),
        &t(I18nKey::LevelWarn),
        &t(I18nKey::LevelError),
        &t(I18nKey::LevelFatal),
    ]);
    level_combo.set_selected(2);
    toolbar.append(&level_combo);

    let sep2 = Separator::new(Orientation::Vertical);
    toolbar.append(&sep2);

    let clear_btn = Button::builder()
        .label(&t(I18nKey::ButtonClear))
        .tooltip_text(&t(I18nKey::TooltipClearLogs))
        .build();
    toolbar.append(&clear_btn);

    let pause_btn = Button::builder()
        .label(&t(I18nKey::ButtonPause))
        .tooltip_text(&t(I18nKey::TooltipPauseResume))
        .build();
    toolbar.append(&pause_btn);

    let sep3 = Separator::new(Orientation::Vertical);
    toolbar.append(&sep3);

    // 自动滚动按钮
    let auto_scroll_btn = Button::builder()
        .label(&format!("⬇️ {}", t(I18nKey::ButtonAuto)))
        .tooltip_text(&t(I18nKey::TooltipAuto))
        .build();
    toolbar.append(&auto_scroll_btn);

    // 跳转到最新按钮
    let jump_latest_btn = Button::builder()
        .label(&t(I18nKey::ButtonLatest))
        .tooltip_text(&t(I18nKey::TooltipLatest))
        .build();
    toolbar.append(&jump_latest_btn);

    let sep4 = Separator::new(Orientation::Vertical);
    toolbar.append(&sep4);

    // 导出按钮
    let export_btn = Button::builder()
        .label(&format!("📤 {}", t(I18nKey::ButtonExport)))
        .tooltip_text(&t(I18nKey::TooltipExport))
        .build();
    toolbar.append(&export_btn);

    let sep5 = Separator::new(Orientation::Vertical);
    toolbar.append(&sep5);

    // 统计按钮
    let stats_btn = Button::builder()
        .label(&t(I18nKey::ButtonStats))
        .tooltip_text(&t(I18nKey::TooltipStats))
        .build();
    toolbar.append(&stats_btn);

    // 设置按钮
    let settings_btn = Button::builder()
        .label(&t(I18nKey::ButtonSettings))
        .tooltip_text(&t(I18nKey::TooltipSettings))
        .build();
    toolbar.append(&settings_btn);

    // 新建标签页按钮事件 - 创建空标签页
    let state_clone = state.clone();
    new_tab_btn.connect_clicked(move |_| {
        if let Some(ref tm) = state_clone.borrow().tab_manager {
            let count = tm.borrow().tab_count();
            let tab_name = format!("{} {}", t(I18nKey::TabName), count + 1);
            tm.borrow_mut().create_tab(&tab_name);
        }
    });

    // SSH 连接按钮事件
    let state_clone = state.clone();
    let window_clone = window.clone();
    ssh_btn.connect_clicked(move |_| {
        let state_ref = state_clone.clone();
        let window_ref = window_clone.clone();

        // 检查是否有标签页
        let has_tabs = state_ref.borrow().tab_manager.as_ref()
            .map(|tm| tm.borrow().tab_count() > 0)
            .unwrap_or(false);

        // 如果没有标签页，先创建一个
        let tab = if !has_tabs {
            if let Some(ref tm) = state_ref.borrow().tab_manager {
                let tab_name = format!("{} 1", t(I18nKey::TabName));
                Some(tm.borrow_mut().create_tab(&tab_name))
            } else {
                None
            }
        } else {
            state_ref.borrow().tab_manager.as_ref()
                .and_then(|tm| tm.borrow().current_tab())
        };

        if let Some(tab) = tab {
            let window_for_error = window_ref.clone();

            // 获取上次SSH输入
            let last_input = state_ref.borrow().config.get_last_ssh_input().cloned();

            crate::ui::dialogs::show_ssh_dialog(&window_ref, last_input.as_ref(), move |ssh_config| {
                // 保存SSH配置到应用配置
                {
                    let mut state = state_ref.borrow_mut();
                    // 保存上次输入（不保存密码）
                    state.config.save_last_ssh_input(
                        &ssh_config.name,
                        &ssh_config.host,
                        ssh_config.port,
                        &ssh_config.username,
                    );
                    state.config.add_ssh_server(ssh_config.clone());
                    let _ = state.config.save();
                }

                // 立即更新标签页状态（连接中）
                let host = ssh_config.host.clone();
                let username = ssh_config.username.clone();
                let tab_id = tab.borrow().id;
                tab.borrow_mut().set_ssh_config(ssh_config.clone());
                tab.borrow_mut().set_source_info(SourceType::Ssh(host.clone(), String::new()));
                tab.borrow_mut().set_current_path("~".to_string());
                tab.borrow_mut().append_terminal_output(&format!("正在连接 {}@{}...", username, host));

                // 更新标签页标题
                if let Some(ref tm) = state_ref.borrow().tab_manager {
                    tm.borrow().update_tab_title(tab_id);
                }

                // 使用 channel 在后台线程执行连接
                let (sender, receiver) = std::sync::mpsc::channel::<Result<(), String>>();
                let ssh_config_for_thread = ssh_config.clone();

                std::thread::spawn(move || {
                    let test_cmd = "echo connected".to_string();
                    let mut source = SshSource::new(ssh_config_for_thread, test_cmd);

                    match source.start() {
                        Ok(_) => {
                            // 连接成功，停止测试命令
                            let _ = source.stop();
                            let _ = sender.send(Ok(()));
                        }
                        Err(e) => {
                            let _ = sender.send(Err(e.to_string()));
                        }
                    }
                });

                // 使用 idle_add 检查连接结果
                let tab_clone = tab.clone();
                let state_ref_clone = state_ref.clone();
                let window_for_error_clone = window_for_error.clone();
                let ssh_config_clone = ssh_config.clone();

                glib::idle_add_local(move || {
                    match receiver.try_recv() {
                        Ok(Ok(_)) => {
                            // 连接成功
                            tab_clone.borrow_mut().set_ssh_connected(true);
                            tab_clone.borrow_mut().append_terminal_output("已连接");
                            tab_clone.borrow_mut().append_terminal_output("输入命令开始执行...\n");

                            if let Some(ref tm) = state_ref_clone.borrow().tab_manager {
                                tm.borrow().update_tab_title(tab_id);
                            }
                            glib::ControlFlow::Break
                        }
                        Ok(Err(e)) => {
                            // 连接失败
                            tab_clone.borrow_mut().append_terminal_output(&format!("连接失败: {}", e));
                            tab_clone.borrow_mut().set_ssh_connected(false);

                            crate::ui::dialogs::show_error_dialog(
                                &window_for_error_clone,
                                &t(I18nKey::ErrorSshConnection),
                                &e
                            );

                            if let Some(ref tm) = state_ref_clone.borrow().tab_manager {
                                tm.borrow().update_tab_title(tab_id);
                            }
                            glib::ControlFlow::Break
                        }
                        Err(std::sync::mpsc::TryRecvError::Empty) => {
                            // 还在连接中，继续等待
                            glib::ControlFlow::Continue
                        }
                        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                            // 线程意外结束
                            glib::ControlFlow::Break
                        }
                    }
                });
            });
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
                let resume_label = t(I18nKey::ButtonResume);
                let pause_label = t(I18nKey::ButtonPause);
                pause_btn_clone.set_label(if paused { &resume_label } else { &pause_label });
            }
        }
    });

    // 自动滚动按钮事件
    let state_clone = state.clone();
    let auto_scroll_btn_clone = auto_scroll_btn.clone();
    auto_scroll_btn.connect_clicked(move |_| {
        if let Some(ref tm) = state_clone.borrow().tab_manager {
            if let Some(tab) = tm.borrow().current_tab() {
                tab.borrow_mut().toggle_auto_scroll();
                let enabled = tab.borrow().is_auto_scroll_enabled();
                let auto_label = t(I18nKey::ButtonAuto);
                let enabled_label = format!("⬇️ {}", auto_label);
                let disabled_label = format!("⏸️ {}", auto_label);
                auto_scroll_btn_clone.set_label(if enabled { &enabled_label } else { &disabled_label });
            }
        }
    });

    // 跳转到最新按钮事件
    let state_clone = state.clone();
    let auto_scroll_btn_clone = auto_scroll_btn.clone();
    jump_latest_btn.connect_clicked(move |_| {
        if let Some(ref tm) = state_clone.borrow().tab_manager {
            if let Some(tab) = tm.borrow().current_tab() {
                tab.borrow().jump_to_latest();
                // 确保自动滚动是启用的
                if !tab.borrow().is_auto_scroll_enabled() {
                    tab.borrow_mut().toggle_auto_scroll();
                }
                let auto_label = t(I18nKey::ButtonAuto);
                let enabled_label = format!("⬇️ {}", auto_label);
                auto_scroll_btn_clone.set_label(&enabled_label);
            }
        }
    });

    // 导出按钮事件
    let state_clone = state.clone();
    let window_clone = window.clone();
    export_btn.connect_clicked(move |_| {
        if let Some(ref tm) = state_clone.borrow().tab_manager {
            if let Some(tab) = tm.borrow().current_tab() {
                let buffer = tab.borrow().text_buffer.clone();
                let export_manager = state_clone.borrow().export_manager.clone();
                let window_ref = window_clone.clone();
                
                ExportManager::show_export_dialog(
                    window_clone.upcast_ref::<gtk4::Window>(),
                    move |format, path| {
                        if let Err(e) = export_manager.borrow().export_text_buffer(&buffer, &path, format) {
                            crate::ui::dialogs::show_error_dialog(
                                &window_ref,
                                &t(I18nKey::ErrorExportFailed),
                                &e
                            );
                        } else {
                            crate::ui::dialogs::show_info_dialog(
                                &window_ref,
                                &t(I18nKey::InfoExportSuccessful),
                                &format!("{}:\n{}", t(I18nKey::InfoExportSuccessful), path)
                            );
                        }
                    }
                );
            } else {
                crate::ui::dialogs::show_info_dialog(
                    &window_clone,
                    &t(I18nKey::DialogInfo),
                    &t(I18nKey::InfoNoActiveTab)
                );
            }
        }
    });

    // 统计按钮事件
    let state_clone = state.clone();
    let window_clone = window.clone();
    stats_btn.connect_clicked(move |_| {
        // 获取当前标签页的统计信息
        if let Some(ref tm) = state_clone.borrow().tab_manager {
            if let Some(tab) = tm.borrow().current_tab() {
                let stats = tab.borrow().get_statistics();
                StatsDialog::show(window_clone.upcast_ref::<gtk4::Window>(), &stats);
            } else {
                // 没有活动标签页，显示空统计
                let stats = LogStatistics::new();
                StatsDialog::show(window_clone.upcast_ref::<gtk4::Window>(), &stats);
            }
        }
    });

    // 设置按钮事件 - 显示导出/导入菜单
    let state_clone = state.clone();
    let window_clone = window.clone();
    settings_btn.connect_clicked(move |_| {
        // 创建设置菜单对话框
        let dialog = gtk4::Dialog::builder()
            .title(&t(I18nKey::DialogSettings))
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

        // 主题设置按钮
        let theme_btn = Button::builder()
            .label(&t(I18nKey::ButtonTheme))
            .tooltip_text(&t(I18nKey::TooltipTheme))
            .hexpand(true)
            .build();
        content.append(&theme_btn);

        // 语言设置按钮
        let lang_btn = Button::builder()
            .label(&t(I18nKey::ButtonLanguage))
            .tooltip_text(&t(I18nKey::TooltipLanguage))
            .hexpand(true)
            .build();
        content.append(&lang_btn);

        // 分隔线
        let sep = Separator::new(Orientation::Horizontal);
        content.append(&sep);

        // 导出按钮
        let export_btn = Button::builder()
            .label(&t(I18nKey::DialogExportSettings))
            .tooltip_text(&t(I18nKey::TooltipExportSettings))
            .hexpand(true)
            .build();
        content.append(&export_btn);

        // 导入按钮
        let import_btn = Button::builder()
            .label(&t(I18nKey::DialogImportSettings))
            .tooltip_text(&t(I18nKey::TooltipImportSettings))
            .hexpand(true)
            .build();
        content.append(&import_btn);

        // 分隔线
        let sep2 = Separator::new(Orientation::Horizontal);
        content.append(&sep2);

        // 关于信息
        let about_box = gtk4::Box::new(Orientation::Vertical, 6);
        about_box.set_halign(gtk4::Align::Center);
        about_box.set_margin_top(12);

        let version_label = Label::builder()
            .label("iLogCat v0.4.0")
            .css_classes(vec!["title-4".to_string()])
            .build();
        about_box.append(&version_label);

        let author_label = Label::builder()
            .label("Author: ayukyo")
            .css_classes(vec!["dim-label".to_string()])
            .build();
        about_box.append(&author_label);

        let repo_label = Label::builder()
            .label("https://github.com/ayukyo/ilogcat")
            .css_classes(vec!["dim-label".to_string()])
            .build();
        about_box.append(&repo_label);

        content.append(&about_box);

        // 关闭按钮
        dialog.add_button(&t(I18nKey::ButtonCancel), gtk4::ResponseType::Close);

        // 语言设置按钮事件
        let state_ref = state_clone.clone();
        let window_ref = Rc::new(RefCell::new(window_clone.clone()));
        lang_btn.connect_clicked(move |_| {
            let state_ref = state_ref.clone();
            let window_for_dialog = window_ref.borrow().clone();
            let current_lang = state_ref.borrow().config.ui.language.clone();
            let window_ref_clone = window_ref.clone();
            
            crate::ui::dialogs::show_language_dialog(&window_for_dialog, &current_lang, move |lang, lang_changed| {
                let mut state = state_ref.borrow_mut();
                state.config.ui.set_language(&lang);

                // 保存配置
                if let Err(e) = state.config.save() {
                    eprintln!("Failed to save language setting: {}", e);
                    crate::ui::dialogs::show_error_dialog(&window_ref_clone.borrow(), &t(I18nKey::DialogError),
                        &format!("{}: {}", t(I18nKey::ErrorSaveFailed), e));
                } else if lang_changed {
                    // 立即应用语言设置到 i18n 系统
                    let lang_enum = crate::i18n::Language::from_str(&lang);
                    crate::i18n::set_language(lang_enum);

                    // 显示重启提示对话框
                    let window_ref_for_confirm = window_ref_clone.clone();
                    crate::ui::dialogs::show_confirm_dialog(
                        &window_ref_clone.borrow(),
                        &t(I18nKey::ConfirmRestartRequired),
                        &t(I18nKey::ConfirmRestartMessage),
                        move || {
                            // 用户确认重启，触发应用重启
                            let window = window_ref_for_confirm.borrow();
                            if let Some(app) = window.application() {
                                // 使用 Gio 的 Application 方法来重启
                                // 先退出，然后重新激活
                                app.quit();
                            }
                        }
                    );
                }
            });
        });
        
        // 主题设置按钮事件
        let state_ref = state_clone.clone();
        let window_ref = Rc::new(RefCell::new(window_clone.clone()));
        theme_btn.connect_clicked(move |_| {
            let state_ref = state_ref.clone();
            let window_for_dialog = window_ref.borrow().clone();
            let current_theme = state_ref.borrow().config.ui.current_theme().to_string();
            let window_ref_clone = window_ref.clone();
            
            crate::ui::dialogs::show_theme_dialog(&window_for_dialog, &current_theme, move |theme| {
                let mut state = state_ref.borrow_mut();
                state.config.ui.set_theme(&theme);

                // 保存配置
                if let Err(e) = state.config.save() {
                    eprintln!("Failed to save theme setting: {}", e);
                } else {
                    // 立即应用主题设置
                    apply_theme(&theme);

                    // 显示提示
                    crate::ui::dialogs::show_info_dialog(&window_ref_clone.borrow(), &t(I18nKey::ThemeLight),
                        &t(I18nKey::InfoThemeChanged));
                }
            });
        });
        
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
                        crate::ui::dialogs::show_info_dialog(&window_ref, &t(I18nKey::InfoExportSuccessful),
                            &format!("{}:\n{}", t(I18nKey::InfoExportSuccessful), path.display()));
                    }
                    Err(e) => {
                        crate::ui::dialogs::show_error_dialog(&window_ref, &t(I18nKey::ErrorExportFailed),
                            &format!("{}:\n{}", t(I18nKey::ErrorExportFailed), e));
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
                            crate::ui::dialogs::show_error_dialog(&window_ref, &t(I18nKey::ErrorSaveFailed),
                                &format!("{}:\n{}", t(I18nKey::ErrorSaveFailed), e));
                        } else {
                            crate::ui::dialogs::show_info_dialog(&window_ref, &t(I18nKey::InfoImportSuccessful),
                                &t(I18nKey::MsgRestartRequired));
                        }
                    }
                    Err(e) => {
                        crate::ui::dialogs::show_error_dialog(&window_ref, &t(I18nKey::ErrorImportFailed),
                            &format!("{}:\n{}", t(I18nKey::ErrorImportFailed), e));
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

fn create_filter_bar(state: Rc<RefCell<AppState>>, _window: &ApplicationWindow) -> (gtk4::Box, SearchEntry) {
    let filter_bar = gtk4::Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(6)
        .build();

    let search_entry = SearchEntry::builder()
        .placeholder_text(&t(I18nKey::MsgFilterLogs))
        .hexpand(true)
        .build();
    filter_bar.append(&search_entry);

    // 大小写敏感开关
    let case_check = gtk4::CheckButton::builder()
        .label("大小写")
        .tooltip_text("区分大小写")
        .build();
    filter_bar.append(&case_check);

    // 正则表达式开关
    let regex_check = gtk4::CheckButton::builder()
        .label("正则")
        .tooltip_text("使用正则表达式过滤")
        .build();
    filter_bar.append(&regex_check);

    // 搜索按钮
    let search_btn = Button::builder()
        .label(&t(I18nKey::ButtonApply))
        .tooltip_text(&t(I18nKey::TooltipApplyFilter))
        .build();
    filter_bar.append(&search_btn);

    // 清除过滤按钮
    let clear_filter_btn = Button::builder()
        .label(&t(I18nKey::ButtonClear))
        .tooltip_text(&t(I18nKey::TooltipClearFilter))
        .build();
    filter_bar.append(&clear_filter_btn);

    // 搜索框回车事件
    let state_clone = state.clone();
    let case_check_clone = case_check.clone();
    let regex_check_clone = regex_check.clone();
    search_entry.connect_activate(move |entry| {
        let text = entry.text().to_string();
        let case_sensitive = case_check_clone.is_active();
        let use_regex = regex_check_clone.is_active();
        apply_filter_to_current_tab(state_clone.clone(), &text, case_sensitive, use_regex);
    });

    // 搜索按钮事件
    let state_clone = state.clone();
    let search_entry_clone = search_entry.clone();
    let case_check_clone = case_check.clone();
    let regex_check_clone = regex_check.clone();
    search_btn.connect_clicked(move |_| {
        let text = search_entry_clone.text().to_string();
        let case_sensitive = case_check_clone.is_active();
        let use_regex = regex_check_clone.is_active();
        apply_filter_to_current_tab(state_clone.clone(), &text, case_sensitive, use_regex);
    });

    // 清除过滤按钮事件
    let state_clone = state.clone();
    let search_entry_clone = search_entry.clone();
    clear_filter_btn.connect_clicked(move |_| {
        search_entry_clone.set_text("");
        clear_filter_on_current_tab(state_clone.clone());
    });

    (filter_bar, search_entry)
}

/// 应用过滤器到当前标签页
fn apply_filter_to_current_tab(state: Rc<RefCell<AppState>>, filter_text: &str, case_sensitive: bool, use_regex: bool) {
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

            // 保存过滤文本
            tab_ref.set_filter_text(filter_text.to_string());

            // 清除现有关键字过滤器
            tab_ref.filter.keywords.clear();

            // 添加新的关键字过滤器（支持大小写和正则）
            let keyword_filter = crate::filter::KeywordFilter::with_regex(
                filter_text.to_string(),
                case_sensitive,  // 大小写敏感
                false,           // 不需要全词匹配
                use_regex,       // 是否使用正则
            );
            tab_ref.filter.keywords.push(keyword_filter);

            // 重新过滤并刷新显示
            tab_ref.filtered_entries.clear();
            let entries_to_add: Vec<_> = tab_ref.log_entries.iter()
                .filter(|entry| tab_ref.filter.matches(entry))
                .cloned()
                .collect();
            tab_ref.filtered_entries = entries_to_add;
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

            // 清除过滤文本
            tab_ref.set_filter_text(String::new());

            // 清除所有过滤器
            tab_ref.filter.keywords.clear();
            tab_ref.filter.regex = None;
            tab_ref.filter.clear_level_filter();

            // 重新显示所有日志
            tab_ref.filtered_entries = tab_ref.log_entries.clone();
            tab_ref.filtered_count = tab_ref.filtered_entries.len();
            tab_ref.refresh_filtered_display();
        }
    }
}

/// 应用增强过滤器到当前标签页
fn apply_enhanced_filter_to_current_tab(state: Rc<RefCell<AppState>>) {
    let tab_manager = {
        let state_ref = state.borrow();
        state_ref.tab_manager.clone()
    };

    if let Some(tm) = tab_manager {
        if let Some(tab) = tm.borrow().current_tab() {
            let mut tab_ref = tab.borrow_mut();
            
            // 获取增强过滤器
            let enhanced_filter = state.borrow().enhanced_filter.clone();
            
            // 重新过滤并刷新显示
            tab_ref.filtered_entries.clear();
            let entries_to_add: Vec<_> = tab_ref.log_entries.iter()
                .filter(|entry| {
                    // 先检查基本过滤器
                    if !tab_ref.filter.matches(entry) {
                        return false;
                    }
                    // 再检查增强过滤器
                    enhanced_filter.borrow().matches(entry)
                })
                .cloned()
                .collect();
            tab_ref.filtered_entries = entries_to_add;
            tab_ref.filtered_count = tab_ref.filtered_entries.len();
            tab_ref.refresh_filtered_display();
        }
    }
}

/// 创建命令快捷方式侧边栏
fn create_command_sidebar(state: Rc<RefCell<AppState>>) -> (gtk4::Box, ListBox) {
    let sidebar = gtk4::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(4)
        .margin_top(4)
        .margin_bottom(4)
        .margin_start(4)
        .margin_end(4)
        .width_request(150)  // 最小宽度
        .build();

    // 标题行
    let title_box = gtk4::Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(4)
        .build();

    let title_label = Label::builder()
        .label("命令快捷方式")
        .hexpand(true)
        .halign(gtk4::Align::Start)
        .css_classes(vec!["title-4".to_string()])
        .build();
    title_box.append(&title_label);

    // 添加按钮
    let add_btn = Button::builder()
        .icon_name("list-add-symbolic")
        .tooltip_text("添加命令快捷方式")
        .build();
    title_box.append(&add_btn);

    sidebar.append(&title_box);

    // 快捷方式列表（可滚动）
    let scrolled = ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .build();

    let shortcuts_list = ListBox::builder()
        .selection_mode(gtk4::SelectionMode::Single)
        .build();
    scrolled.set_child(Some(&shortcuts_list));
    sidebar.append(&scrolled);

    // 从配置加载已有快捷方式
    let shortcuts = state.borrow().config.command_shortcuts.clone();
    for shortcut in shortcuts {
        add_shortcut_item(&shortcuts_list, &shortcut.name, &shortcut.command, state.clone());
    }

    // 添加按钮点击事件
    let state_clone = state.clone();
    let shortcuts_list_clone = shortcuts_list.clone();
    add_btn.connect_clicked(move |_| {
        show_edit_shortcut_dialog(
            None,  // 新建
            None,
            shortcuts_list_clone.clone(),
            state_clone.clone(),
        );
    });

    (sidebar, shortcuts_list)
}

/// 保存快捷方式顺序
fn save_shortcuts_order(list: &ListBox, state: Rc<RefCell<AppState>>) {
    let mut new_shortcuts = Vec::new();

    let mut child = list.first_child();
    while let Some(widget) = child {
        let next = widget.next_sibling();
        if let Some(listbox_row) = widget.dynamic_cast::<ListBoxRow>().ok() {
            // 从 row 获取名称和命令
            if let Some(row_box) = listbox_row.first_child() {
                if let Some(hbox) = row_box.dynamic_cast::<gtk4::Box>().ok() {
                    // 第一个子元素是名称按钮
                    if let Some(first_child) = hbox.first_child() {
                        if let Some(btn) = first_child.dynamic_cast::<Button>().ok() {
                            if let Some(label) = btn.child().and_then(|c| c.dynamic_cast::<Label>().ok()) {
                                let name = label.label();
                                // 从配置中查找对应的命令
                                let state_ref = state.borrow();
                                if let Some(shortcut) = state_ref.config.command_shortcuts.iter().find(|s| s.name == name) {
                                    new_shortcuts.push(shortcut.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
        child = next;
    }

    // 保存新顺序
    let mut state_ref = state.borrow_mut();
    state_ref.config.command_shortcuts = new_shortcuts;
    let _ = state_ref.config.save();
}

/// 添加快捷方式条目到列表
fn add_shortcut_item(
    list: &ListBox,
    name: &str,
    command: &str,
    state: Rc<RefCell<AppState>>,
) {
    let row_box = gtk4::Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(2)
        .margin_top(2)
        .margin_bottom(2)
        .margin_start(4)
        .margin_end(4)
        .build();

    // 名称按钮（点击执行）
    let name_btn = Button::builder()
        .label(name)
        .hexpand(true)
        .halign(gtk4::Align::Start)
        .tooltip_text(&format!("{}: {}", name, command))
        .css_classes(vec!["flat".to_string()])
        .build();

    // 上移按钮
    let up_btn = Button::builder()
        .icon_name("go-up-symbolic")
        .css_classes(vec!["flat".to_string()])
        .tooltip_text("上移")
        .build();

    // 编辑按钮
    let edit_btn = Button::builder()
        .icon_name("document-edit-symbolic")
        .css_classes(vec!["flat".to_string()])
        .tooltip_text("编辑")
        .build();

    // 删除按钮
    let delete_btn = Button::builder()
        .icon_name("user-trash-symbolic")
        .css_classes(vec!["flat".to_string()])
        .tooltip_text("删除")
        .build();

    row_box.append(&name_btn);
    row_box.append(&up_btn);
    row_box.append(&edit_btn);
    row_box.append(&delete_btn);

    let row = ListBoxRow::builder()
        .child(&row_box)
        .activatable(false)
        .build();

    // 点击执行命令
    let state_clone = state.clone();
    let command_for_exec = command.to_string();
    name_btn.connect_clicked(move |_| {
        execute_shortcut_command(state_clone.clone(), &command_for_exec);
    });

    // 上移按钮
    let list_clone = list.clone();
    let row_clone = row.clone();
    let state_clone = state.clone();
    up_btn.connect_clicked(move |_| {
        move_row_up(&list_clone, &row_clone, state_clone.clone());
    });

    // 编辑按钮
    let state_clone = state.clone();
    let list_clone = list.clone();
    let name_clone = name.to_string();
    let command_clone = command.to_string();
    let row_clone = row.clone();
    edit_btn.connect_clicked(move |_| {
        show_edit_shortcut_dialog(
            Some(&name_clone),
            Some(&command_clone),
            list_clone.clone(),
            state_clone.clone(),
        );
        // 移除旧行
        list_clone.remove(&row_clone);
    });

    // 删除按钮
    let state_clone = state.clone();
    let list_clone = list.clone();
    let name_clone = name.to_string();
    let row_clone = row.clone();
    delete_btn.connect_clicked(move |_| {
        // 从配置中删除
        {
            let mut state_ref = state_clone.borrow_mut();
            state_ref.config.command_shortcuts.retain(|s| s.name != name_clone);
            let _ = state_ref.config.save();
        }
        // 从列表中移除
        list_clone.remove(&row_clone);
    });

    list.append(&row);
}

/// 上移行
fn move_row_up(list: &ListBox, row: &ListBoxRow, state: Rc<RefCell<AppState>>) {
    let index = row.index();
    if index > 0 {
        list.remove(row);
        list.insert(row, index - 1);
        save_shortcuts_order(list, state);
    }
}

/// 执行快捷方式命令
fn execute_shortcut_command(state: Rc<RefCell<AppState>>, command: &str) {
    let tab_manager = {
        let state_ref = state.borrow();
        state_ref.tab_manager.clone()
    };

    if let Some(tm) = tab_manager {
        if let Some(tab) = tm.borrow().current_tab() {
            let command_entry = tab.borrow().command_entry.clone();
            command_entry.set_text(command);
            command_entry.emit_activate();
        }
    }
}

/// 显示编辑快捷方式对话框
fn show_edit_shortcut_dialog(
    initial_name: Option<&str>,
    initial_command: Option<&str>,
    list: ListBox,
    state: Rc<RefCell<AppState>>,
) {
    let dialog = gtk4::Dialog::builder()
        .title(if initial_name.is_some() { "编辑命令" } else { "添加命令" })
        .modal(true)
        .use_header_bar(1)
        .build();

    let content = dialog.content_area();

    let vbox = gtk4::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .margin_top(8)
        .margin_bottom(8)
        .margin_start(12)
        .margin_end(12)
        .build();

    // 名称输入
    let name_label = Label::builder()
        .label("名称:")
        .halign(gtk4::Align::Start)
        .build();
    vbox.append(&name_label);

    let name_entry = Entry::builder()
        .placeholder_text("显示名称")
        .hexpand(true)
        .build();
    if let Some(name) = initial_name {
        name_entry.set_text(name);
    }
    vbox.append(&name_entry);

    // 命令输入
    let cmd_label = Label::builder()
        .label("命令:")
        .halign(gtk4::Align::Start)
        .build();
    vbox.append(&cmd_label);

    let cmd_entry = Entry::builder()
        .placeholder_text("要执行的命令")
        .hexpand(true)
        .build();
    if let Some(cmd) = initial_command {
        cmd_entry.set_text(cmd);
    }
    vbox.append(&cmd_entry);

    content.append(&vbox);

    // 添加按钮
    dialog.add_button("取消", gtk4::ResponseType::Cancel);
    dialog.add_button("保存", gtk4::ResponseType::Accept);

    let state_clone = state.clone();
    let list_clone = list.clone();
    let name_entry_clone = name_entry.clone();
    let cmd_entry_clone = cmd_entry.clone();
    let old_name = initial_name.map(|s| s.to_string());

    dialog.connect_response(move |dialog, response| {
        if response == gtk4::ResponseType::Accept {
            let name = name_entry_clone.text().to_string();
            let command = cmd_entry_clone.text().to_string();

            if !name.is_empty() && !command.is_empty() {
                // 更新配置
                {
                    let mut state_ref = state_clone.borrow_mut();
                    // 如果是编辑，先删除旧的
                    if let Some(ref old) = old_name {
                        state_ref.config.command_shortcuts.retain(|s| s.name != *old);
                    }
                    state_ref.config.command_shortcuts.push(crate::config::CommandShortcut {
                        name: name.clone(),
                        command: command.clone(),
                    });
                    let _ = state_ref.config.save();
                }

                // 更新列表
                add_shortcut_item(&list_clone, &name, &command, state_clone.clone());
            }
        }
        dialog.close();
    });

    dialog.show();
}