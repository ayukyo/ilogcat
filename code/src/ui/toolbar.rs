use gtk4::prelude::*;
use gtk4::{Box, Button, DropDown, Label, Separator, Orientation};
use std::cell::RefCell;
use std::rc::Rc;

use crate::config::Config;

/// 工具栏回调 trait
pub trait ToolbarCallbacks: Clone {
    fn on_source_changed(&self, source_type: SourceType);
    fn on_level_changed(&self, level: LogLevelFilter);
    fn on_clear_clicked(&self);
    fn on_pause_clicked(&self);
    fn on_settings_clicked(&self);
}

/// 日志源类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SourceType {
    Dmesg,
    Journalctl,
    File,
    Ssh,
}

impl SourceType {
    pub fn from_index(index: u32) -> Option<Self> {
        match index {
            0 => Some(SourceType::Dmesg),
            1 => Some(SourceType::Journalctl),
            2 => Some(SourceType::File),
            3 => Some(SourceType::Ssh),
            _ => None,
        }
    }

    pub fn to_index(&self) -> u32 {
        match self {
            SourceType::Dmesg => 0,
            SourceType::Journalctl => 1,
            SourceType::File => 2,
            SourceType::Ssh => 3,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            SourceType::Dmesg => "Local: dmesg",
            SourceType::Journalctl => "Local: journalctl",
            SourceType::File => "File...",
            SourceType::Ssh => "SSH...",
        }
    }
}

/// 日志级别过滤
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LogLevelFilter {
    Verbose,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

impl LogLevelFilter {
    pub fn from_index(index: u32) -> Option<Self> {
        match index {
            0 => Some(LogLevelFilter::Verbose),
            1 => Some(LogLevelFilter::Debug),
            2 => Some(LogLevelFilter::Info),
            3 => Some(LogLevelFilter::Warn),
            4 => Some(LogLevelFilter::Error),
            5 => Some(LogLevelFilter::Fatal),
            _ => None,
        }
    }

    pub fn to_index(&self) -> u32 {
        match self {
            LogLevelFilter::Verbose => 0,
            LogLevelFilter::Debug => 1,
            LogLevelFilter::Info => 2,
            LogLevelFilter::Warn => 3,
            LogLevelFilter::Error => 4,
            LogLevelFilter::Fatal => 5,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            LogLevelFilter::Verbose => "Verbose",
            LogLevelFilter::Debug => "Debug",
            LogLevelFilter::Info => "Info",
            LogLevelFilter::Warn => "Warn",
            LogLevelFilter::Error => "Error",
            LogLevelFilter::Fatal => "Fatal",
        }
    }
}

/// 工具栏组件
pub struct Toolbar {
    container: Box,
    source_combo: DropDown,
    level_combo: DropDown,
    clear_btn: Button,
    pause_btn: Button,
    settings_btn: Button,
    is_paused: Rc<RefCell<bool>>,
}

impl Toolbar {
    pub fn new<C: ToolbarCallbacks + 'static>(callbacks: C) -> Self {
        let container = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(6)
            .build();

        // 源选择器
        let source_label = Label::new(Some("Source:"));
        container.append(&source_label);

        let source_strings: Vec<&str> = vec![
            SourceType::Dmesg.display_name(),
            SourceType::Journalctl.display_name(),
            SourceType::File.display_name(),
            SourceType::Ssh.display_name(),
        ];
        let source_combo = DropDown::from_strings(&source_strings);
        source_combo.set_selected(0);
        container.append(&source_combo);

        // 分隔符
        let sep = Separator::new(Orientation::Vertical);
        container.append(&sep);

        // 级别选择器
        let level_label = Label::new(Some("Min Level:"));
        container.append(&level_label);

        let level_strings: Vec<&str> = vec![
            LogLevelFilter::Verbose.display_name(),
            LogLevelFilter::Debug.display_name(),
            LogLevelFilter::Info.display_name(),
            LogLevelFilter::Warn.display_name(),
            LogLevelFilter::Error.display_name(),
            LogLevelFilter::Fatal.display_name(),
        ];
        let level_combo = DropDown::from_strings(&level_strings);
        level_combo.set_selected(2); // 默认 Info
        container.append(&level_combo);

        // 分隔符
        let sep2 = Separator::new(Orientation::Vertical);
        container.append(&sep2);

        // 清除按钮
        let clear_btn = Button::builder()
            .label("Clear")
            .tooltip_text("Clear all logs (Ctrl+L)")
            .build();
        container.append(&clear_btn);

        // 暂停按钮
        let pause_btn = Button::builder()
            .label("Pause")
            .tooltip_text("Pause/Resume log stream (Ctrl+S)")
            .build();
        container.append(&pause_btn);

        // 设置按钮
        let settings_btn = Button::builder()
            .icon_name("preferences-system-symbolic")
            .tooltip_text("Settings")
            .build();
        container.append(&settings_btn);

        let is_paused = Rc::new(RefCell::new(false));

        // 连接信号
        let callbacks_clone = callbacks.clone();
        source_combo.connect_selected_notify(move |combo| {
            if let Some(source_type) = SourceType::from_index(combo.selected()) {
                callbacks_clone.on_source_changed(source_type);
            }
        });

        let callbacks_clone = callbacks.clone();
        level_combo.connect_selected_notify(move |combo| {
            if let Some(level) = LogLevelFilter::from_index(combo.selected()) {
                callbacks_clone.on_level_changed(level);
            }
        });

        let callbacks_clone = callbacks.clone();
        clear_btn.connect_clicked(move |_| {
            callbacks_clone.on_clear_clicked();
        });

        let callbacks_clone = callbacks.clone();
        let pause_btn_clone = pause_btn.clone();
        let is_paused_clone = is_paused.clone();
        pause_btn.connect_clicked(move |_| {
            let mut paused = is_paused_clone.borrow_mut();
            *paused = !*paused;
            pause_btn_clone.set_label(if *paused { "Resume" } else { "Pause" });
            callbacks_clone.on_pause_clicked();
        });

        let callbacks_clone = callbacks.clone();
        settings_btn.connect_clicked(move |_| {
            callbacks_clone.on_settings_clicked();
        });

        Self {
            container,
            source_combo,
            level_combo,
            clear_btn,
            pause_btn,
            settings_btn,
            is_paused,
        }
    }

    pub fn widget(&self) -> &Box {
        &self.container
    }

    pub fn set_paused(&self, paused: bool) {
        *self.is_paused.borrow_mut() = paused;
        self.pause_btn.set_label(if paused { "Resume" } else { "Pause" });
    }

    pub fn selected_source(&self) -> Option<SourceType> {
        SourceType::from_index(self.source_combo.selected())
    }

    pub fn selected_level(&self) -> Option<LogLevelFilter> {
        LogLevelFilter::from_index(self.level_combo.selected())
    }

    pub fn set_source(&self, source: SourceType) {
        self.source_combo.set_selected(source.to_index());
    }

    pub fn set_level(&self, level: LogLevelFilter) {
        self.level_combo.set_selected(level.to_index());
    }
}
