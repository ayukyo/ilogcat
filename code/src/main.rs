mod ui;
mod log;
mod filter;
mod ssh;
mod config;

use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Box, ScrolledWindow, TextView, Statusbar, DropDown, Button, SearchEntry, ToggleButton, Label, Separator};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::log::{LogSource, LogEntry, LogLevel, CommandSource, FileWatchSource};
use crate::filter::Filter;
use crate::config::Config;

const APP_ID: &str = "com.openclaw.ilogcat";

fn main() {
    // 加载配置
    let _config = Config::load().unwrap_or_default();

    // 创建 GTK 应用
    let app = Application::builder()
        .application_id(APP_ID)
        .build();

    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &Application) {
    // 创建主窗口
    let window = ApplicationWindow::builder()
        .application(app)
        .title("iLogCat")
        .default_width(1200)
        .default_height(800)
        .build();

    // 创建主垂直布局
    let vbox = Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(6)
        .margin_top(6)
        .margin_bottom(6)
        .margin_start(6)
        .margin_end(6)
        .build();

    // 创建工具栏
    let toolbar = create_toolbar(&window);
    vbox.append(&toolbar);

    // 创建过滤栏
    let filter_bar = create_filter_bar();
    vbox.append(&filter_bar);

    // 创建日志显示区域
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

    // 创建状态栏
    let status_bar = Statusbar::new();
    let context_id = status_bar.context_id("main");
    status_bar.push(context_id, "Ready - No log source connected");
    vbox.append(&status_bar);

    // 设置窗口内容
    window.set_child(Some(&vbox));

    // 状态变量
    let _log_entries: Rc<RefCell<Vec<LogEntry>>> = Rc::new(RefCell::new(Vec::new()));
    let _is_paused = Arc::new(AtomicBool::new(false));
    let _filter = Rc::new(RefCell::new(Filter::new()));
    let _current_source: Rc<RefCell<Option<Box<dyn LogSource>>>> = Rc::new(RefCell::new(None));

    // 存储状态引用以便在回调中使用
    // 使用 Rc<RefCell<>> 在闭包中共享状态
    // 注意：GTK4 推荐使用自定义 Widget 来管理状态，这里简化处理

    // 显示窗口
    window.present();
}

fn create_toolbar(window: &ApplicationWindow) -> Box {
    let toolbar = Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(6)
        .build();

    // 源选择器标签
    let source_label = Label::new(Some("Source:"));
    toolbar.append(&source_label);

    // 源选择下拉框
    let source_combo = DropDown::from_strings(&[
        "Local: dmesg",
        "Local: journalctl",
        "File...",
        "SSH...",
    ]);
    toolbar.append(&source_combo);

    // 分隔符
    let sep = Separator::new(gtk4::Orientation::Vertical);
    toolbar.append(&sep);

    // 级别选择标签
    let level_label = Label::new(Some("Level:"));
    toolbar.append(&level_label);

    // 级别选择下拉框
    let level_combo = DropDown::from_strings(&[
        "Verbose",
        "Debug",
        "Info",
        "Warn",
        "Error",
        "Fatal",
    ]);
    level_combo.set_selected(2); // 默认 Info
    toolbar.append(&level_combo);

    // 清除按钮
    let clear_btn = Button::builder()
        .label("Clear")
        .build();
    toolbar.append(&clear_btn);

    // 暂停按钮
    let pause_btn = Button::builder()
        .label("Pause")
        .build();
    toolbar.append(&pause_btn);

    // 设置按钮
    let settings_btn = Button::builder()
        .icon_name("preferences-system-symbolic")
        .build();
    toolbar.append(&settings_btn);

    toolbar
}

fn create_filter_bar() -> Box {
    let filter_bar = Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(6)
        .build();

    // 搜索框
    let search_entry = SearchEntry::builder()
        .placeholder_text("Filter logs...")
        .hexpand(true)
        .build();
    filter_bar.append(&search_entry);

    // 添加过滤按钮
    let add_filter_btn = Button::builder()
        .label("+ Filter")
        .build();
    filter_bar.append(&add_filter_btn);

    // 正则开关
    let regex_toggle = ToggleButton::builder()
        .label("Regex")
        .build();
    filter_bar.append(&regex_toggle);

    // 大小写敏感开关
    let case_toggle = ToggleButton::builder()
        .label("Aa")
        .build();
    filter_bar.append(&case_toggle);

    filter_bar
}

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
