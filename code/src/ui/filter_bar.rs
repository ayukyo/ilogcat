use gtk4::prelude::*;
use gtk4::{Box, SearchEntry, Button, ToggleButton, ComboBoxText, Label};

/// 过滤工具栏
pub struct FilterBar {
    pub container: Box,
    pub search_entry: SearchEntry,
    pub level_combo: ComboBoxText,
    pub regex_toggle: ToggleButton,
    pub case_toggle: ToggleButton,
    pub add_filter_btn: Button,
    pub clear_btn: Button,
}

impl FilterBar {
    /// 创建新的过滤工具栏
    pub fn new() -> Self {
        let container = Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(6)
            .build();

        // 搜索框
        let search_entry = SearchEntry::builder()
            .placeholder_text("Filter logs...")
            .hexpand(true)
            .build();
        container.append(&search_entry);

        // 级别选择
        let level_label = Label::new(Some("Level:"));
        container.append(&level_label);

        let level_combo = ComboBoxText::builder()
            .build();
        level_combo.append_text("All");
        level_combo.append_text("Verbose");
        level_combo.append_text("Debug");
        level_combo.append_text("Info");
        level_combo.append_text("Warn");
        level_combo.append_text("Error");
        level_combo.append_text("Fatal");
        level_combo.set_active(Some(0));
        container.append(&level_combo);

        // 正则开关
        let regex_toggle = ToggleButton::builder()
            .label("Regex")
            .build();
        container.append(&regex_toggle);

        // 大小写敏感开关
        let case_toggle = ToggleButton::builder()
            .label("Aa")
            .build();
        container.append(&case_toggle);

        // 添加过滤按钮
        let add_filter_btn = Button::builder()
            .label("+ Filter")
            .build();
        container.append(&add_filter_btn);

        // 清除按钮
        let clear_btn = Button::builder()
            .label("Clear")
            .build();
        container.append(&clear_btn);

        Self {
            container,
            search_entry,
            level_combo,
            regex_toggle,
            case_toggle,
            add_filter_btn,
            clear_btn,
        }
    }

    /// 获取搜索文本
    pub fn search_text(&self) -> String {
        self.search_entry.text().to_string()
    }

    /// 设置搜索文本
    pub fn set_search_text(&self, text: &str) {
        self.search_entry.set_text(text);
    }

    /// 获取选中的级别
    pub fn selected_level(&self) -> Option<String> {
        self.level_combo.active_text().map(|t| t.to_string())
    }

    /// 是否启用正则
    pub fn is_regex_enabled(&self) -> bool {
        self.regex_toggle.is_active()
    }

    /// 是否大小写敏感
    pub fn is_case_sensitive(&self) -> bool {
        self.case_toggle.is_active()
    }

    /// 获取容器
    pub fn widget(&self) -> &Box {
        &self.container
    }

    /// 连接搜索变化回调
    pub fn connect_search_changed<F: Fn(&SearchEntry) + 'static>(&self, f: F) {
        self.search_entry.connect_search_changed(f);
    }

    /// 连接清除按钮点击
    pub fn connect_clear_clicked<F: Fn(&Button) + 'static>(&self, f: F) {
        self.clear_btn.connect_clicked(f);
    }

    /// 连接添加过滤按钮点击
    pub fn connect_add_filter_clicked<F: Fn(&Button) + 'static>(&self, f: F) {
        self.add_filter_btn.connect_clicked(f);
    }

    /// 连接级别变化
    pub fn connect_level_changed<F: Fn(&ComboBoxText) + 'static>(&self, f: F) {
        self.level_combo.connect_changed(f);
    }
}

impl Default for FilterBar {
    fn default() -> Self {
        Self::new()
    }
}
