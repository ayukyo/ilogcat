use regex::Regex;
use std::cell::RefCell;
use std::rc::Rc;
use gtk4::prelude::*;
use glib::clone;
use crate::log::LogEntry;
use crate::i18n::{t, I18nKey};

/// 过滤模式类型
#[derive(Debug, Clone, PartialEq)]
pub enum FilterPattern {
    /// 包含模式（匹配则显示）
    Include(String),
    /// 排除模式（匹配则不显示）
    Exclude(String),
}

/// 增强型正则过滤器
pub struct EnhancedRegexFilter {
    patterns: Vec<FilterPattern>,
    compiled: Vec<(FilterPattern, Option<Regex>)>,
    case_sensitive: bool,
    use_regex: bool,
}

impl EnhancedRegexFilter {
    pub fn new() -> Self {
        Self {
            patterns: Vec::new(),
            compiled: Vec::new(),
            case_sensitive: false,
            use_regex: true,
        }
    }

    /// 设置是否区分大小写
    pub fn set_case_sensitive(&mut self, sensitive: bool) {
        self.case_sensitive = sensitive;
        self.recompile_all();
    }

    /// 设置是否使用正则表达式
    pub fn set_use_regex(&mut self, use_regex: bool) {
        self.use_regex = use_regex;
        self.recompile_all();
    }

    /// 添加包含模式
    pub fn add_include(&mut self, pattern: &str) -> Result<(), String> {
        let fp = FilterPattern::Include(pattern.to_string());
        let regex = self.compile_pattern(pattern)?;
        self.patterns.push(fp.clone());
        self.compiled.push((fp, regex));
        Ok(())
    }

    /// 添加排除模式
    pub fn add_exclude(&mut self, pattern: &str) -> Result<(), String> {
        let fp = FilterPattern::Exclude(pattern.to_string());
        let regex = self.compile_pattern(pattern)?;
        self.patterns.push(fp.clone());
        self.compiled.push((fp, regex));
        Ok(())
    }

    /// 移除指定索引的模式
    pub fn remove_pattern(&mut self, index: usize) {
        if index < self.patterns.len() {
            self.patterns.remove(index);
            self.compiled.remove(index);
        }
    }

    /// 清除所有模式
    pub fn clear(&mut self) {
        self.patterns.clear();
        self.compiled.clear();
    }

    /// 获取所有模式
    pub fn get_patterns(&self) -> &Vec<FilterPattern> {
        &self.patterns
    }

    /// 检查是否有过滤规则
    pub fn is_empty(&self) -> bool {
        self.patterns.is_empty()
    }

    /// 编译单个模式
    fn compile_pattern(&self, pattern: &str) -> Result<Option<Regex>, String> {
        if !self.use_regex {
            // 不使用正则，返回None
            return Ok(None);
        }

        let regex_pattern = if self.case_sensitive {
            pattern.to_string()
        } else {
            // 不区分大小写，添加(?i)标志
            format!("(?i){}", pattern)
        };

        match Regex::new(&regex_pattern) {
            Ok(re) => Ok(Some(re)),
            Err(e) => Err(format!("Invalid regex pattern: {}", e)),
        }
    }

    /// 重新编译所有模式
    fn recompile_all(&mut self) {
        self.compiled.clear();
        for pattern in &self.patterns {
            let regex = match pattern {
                FilterPattern::Include(p) | FilterPattern::Exclude(p) => {
                    self.compile_pattern(p).unwrap_or(None)
                }
            };
            self.compiled.push((pattern.clone(), regex));
        }
    }

    /// 检查文本是否匹配模式
    fn matches_pattern(&self, text: &str, pattern: &str, regex: &Option<Regex>) -> bool {
        if let Some(ref re) = regex {
            re.is_match(text)
        } else {
            // 简单字符串匹配
            if self.case_sensitive {
                text.contains(pattern)
            } else {
                text.to_lowercase().contains(&pattern.to_lowercase())
            }
        }
    }

    /// 检查日志条目是否通过过滤
    pub fn matches(&self, entry: &LogEntry) -> bool {
        if self.patterns.is_empty() {
            return true;
        }

        let full_text = format!(
            "{} {} {}: {}",
            entry.timestamp.format("%H:%M:%S.%3f"),
            entry.level,
            entry.tag,
            entry.message
        );

        let mut has_include = false;
        let mut include_matched = false;

        for (pattern, regex) in &self.compiled {
            match pattern {
                FilterPattern::Include(p) => {
                    has_include = true;
                    if self.matches_pattern(&full_text, p, regex) {
                        include_matched = true;
                    }
                }
                FilterPattern::Exclude(p) => {
                    if self.matches_pattern(&full_text, p, regex) {
                        return false; // 排除模式匹配，直接返回false
                    }
                }
            }
        }

        // 如果有包含模式，必须至少匹配一个
        if has_include {
            include_matched
        } else {
            true // 只有排除模式，且没有匹配到
        }
    }

    /// 序列化为字符串（用于保存配置）
    pub fn serialize(&self) -> String {
        let parts: Vec<String> = self.patterns.iter().map(|p| {
            match p {
                FilterPattern::Include(s) => format!("+{}", s),
                FilterPattern::Exclude(s) => format!("-{}", s),
            }
        }).collect();
        parts.join("\n")
    }

    /// 从字符串反序列化
    pub fn deserialize(&mut self, data: &str) {
        self.clear();
        for line in data.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some(rest) = line.strip_prefix('+') {
                let _ = self.add_include(rest);
            } else if let Some(rest) = line.strip_prefix('-') {
                let _ = self.add_exclude(rest);
            } else {
                // 默认作为包含模式
                let _ = self.add_include(line);
            }
        }
    }
}

/// 过滤对话框
pub struct FilterDialog;

impl FilterDialog {
    /// 显示增强过滤对话框
    pub fn show<F: Fn(Vec<FilterPattern>, bool, bool) + 'static>(
        parent: &gtk4::Window,
        current_patterns: Vec<FilterPattern>,
        case_sensitive: bool,
        use_regex: bool,
        callback: F,
    ) {
        let dialog = gtk4::Dialog::new();
        dialog.set_title(Some(&t(I18nKey::DialogAdvancedFilter)));
        dialog.set_transient_for(Some(parent));
        dialog.set_modal(true);
        dialog.set_default_size(500, 400);

        let content = dialog.content_area();
        content.set_spacing(12);
        content.set_margin_top(12);
        content.set_margin_bottom(12);
        content.set_margin_start(12);
        content.set_margin_end(12);

        // 选项区域
        let options_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);

        let case_sensitive_text = t(I18nKey::LabelCaseSensitive);
        let case_check = gtk4::CheckButton::with_label(&case_sensitive_text);
        case_check.set_active(case_sensitive);
        options_box.append(&case_check);

        let use_regex_text = t(I18nKey::LabelUseRegex);
        let regex_check = gtk4::CheckButton::with_label(&use_regex_text);
        regex_check.set_active(use_regex);
        options_box.append(&regex_check);

        content.append(&options_box);

        // 模式列表
        let scrolled = gtk4::ScrolledWindow::new();
        scrolled.set_vexpand(true);

        let list_box = gtk4::ListBox::new();
        list_box.set_selection_mode(gtk4::SelectionMode::None);

        // 存储模式行的引用
        let patterns: Rc<RefCell<Vec<(FilterPattern, gtk4::Box)>>> = Rc::new(RefCell::new(Vec::new()));

        // 添加现有模式
        for pattern in current_patterns {
            Self::add_pattern_row(&list_box, &patterns, pattern);
        }

        scrolled.set_child(Some(&list_box));
        content.append(&scrolled);

        // 添加新模式的输入区域
        let input_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);

        let type_combo = gtk4::ComboBoxText::new();
        let include_text = t(I18nKey::LabelInclude);
        let exclude_text = t(I18nKey::LabelExclude);
        type_combo.append(Some("include"), &include_text);
        type_combo.append(Some("exclude"), &exclude_text);
        type_combo.set_active_id(Some("include"));
        input_box.append(&type_combo);

        let pattern_entry = gtk4::Entry::new();
        pattern_entry.set_hexpand(true);
        let placeholder = t(I18nKey::PlaceholderPattern);
        pattern_entry.set_placeholder_text(Some(&placeholder));
        input_box.append(&pattern_entry);

        let add_text = t(I18nKey::ButtonAdd);
        let add_btn = gtk4::Button::with_label(&add_text);
        input_box.append(&add_btn);

        content.append(&input_box);

        // 添加按钮回调
        let list_box_clone = list_box.clone();
        let patterns_clone = patterns.clone();
        let type_combo_clone = type_combo.clone();
        let pattern_entry_clone = pattern_entry.clone();
        add_btn.connect_clicked(move |_| {
            let pattern_text = pattern_entry_clone.text().to_string();
            if !pattern_text.is_empty() {
                let pattern = if type_combo_clone.active_id().as_deref() == Some("exclude") {
                    FilterPattern::Exclude(pattern_text)
                } else {
                    FilterPattern::Include(pattern_text)
                };
                FilterDialog::add_pattern_row(&list_box_clone, &patterns_clone, pattern);
                pattern_entry_clone.set_text("");
            }
        });

        let cancel_text = t(I18nKey::ButtonCancel);
        let apply_text = t(I18nKey::ButtonApply);
        dialog.add_button(&cancel_text, gtk4::ResponseType::Cancel);
        dialog.add_button(&apply_text, gtk4::ResponseType::Accept);

        dialog.connect_response(move |dialog, response| {
            if response == gtk4::ResponseType::Accept {
                let final_patterns: Vec<FilterPattern> = patterns.borrow()
                    .iter()
                    .map(|(p, _)| p.clone())
                    .collect();
                callback(final_patterns, case_check.is_active(), regex_check.is_active());
            }
            dialog.close();
        });

        dialog.show();
    }

    fn add_pattern_row(
        list_box: &gtk4::ListBox,
        patterns: &Rc<RefCell<Vec<(FilterPattern, gtk4::Box)>>>,
        pattern: FilterPattern,
    ) {
        let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        row.set_margin_top(6);
        row.set_margin_bottom(6);
        row.set_margin_start(6);
        row.set_margin_end(6);

        let include_label = format!("{}:", t(I18nKey::LabelInclude));
        let exclude_label = format!("{}:", t(I18nKey::LabelExclude));
        // 类型标签
        let type_label = gtk4::Label::new(Some(match &pattern {
            FilterPattern::Include(_) => &include_label,
            FilterPattern::Exclude(_) => &exclude_label,
        }));
        type_label.set_width_chars(10);
        row.append(&type_label);

        // 模式文本
        let text = match &pattern {
            FilterPattern::Include(s) | FilterPattern::Exclude(s) => s.clone(),
        };
        let text_label = gtk4::Label::new(Some(&text));
        text_label.set_hexpand(true);
        text_label.set_xalign(0.0);
        row.append(&text_label);

        // 删除按钮
        let del_btn = gtk4::Button::from_icon_name("user-trash-symbolic");
        del_btn.add_css_class("flat");
        row.append(&del_btn);

        list_box.append(&row);
        patterns.borrow_mut().push((pattern, row.clone()));

        // 删除回调
        let patterns_clone = patterns.clone();
        let list_clone = list_box.clone();
        let row_clone = row.clone();
        del_btn.connect_clicked(move |_| {
            list_clone.remove(&row_clone);
            // 从patterns中移除对应的项
            patterns_clone.borrow_mut().retain(|(_, r)| r != &row_clone);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Local;

    fn create_test_entry(level: &str, tag: &str, message: &str) -> LogEntry {
        LogEntry {
            timestamp: Local::now(),
            level: level.parse().unwrap_or(crate::log::LogLevel::Info),
            tag: tag.to_string(),
            pid: None,
            message: message.to_string(),
            source: crate::log::LogSourceInfo::Local("test".to_string()),
            raw_line: format!("{} {}: {}", level, tag, message),
        }
    }

    #[test]
    fn test_include_only() {
        let mut filter = EnhancedRegexFilter::new();
        filter.add_include("error").unwrap();

        let entry1 = create_test_entry("ERROR", "Test", "An error occurred");
        let entry2 = create_test_entry("INFO", "Test", "All good");

        assert!(filter.matches(&entry1));
        assert!(!filter.matches(&entry2));
    }

    #[test]
    fn test_exclude_only() {
        let mut filter = EnhancedRegexFilter::new();
        filter.add_exclude("debug").unwrap();

        let entry1 = create_test_entry("DEBUG", "Test", "Debug message");
        let entry2 = create_test_entry("INFO", "Test", "Info message");

        assert!(!filter.matches(&entry1));
        assert!(filter.matches(&entry2));
    }

    #[test]
    fn test_include_and_exclude() {
        let mut filter = EnhancedRegexFilter::new();
        filter.add_include("Test").unwrap();
        filter.add_exclude("debug").unwrap();

        let entry1 = create_test_entry("DEBUG", "Test", "Test debug");
        let entry2 = create_test_entry("INFO", "Test", "Test info");
        let entry3 = create_test_entry("INFO", "Other", "Other info");

        assert!(!filter.matches(&entry1)); // 匹配include但匹配exclude
        assert!(filter.matches(&entry2));  // 匹配include且不匹配exclude
        assert!(!filter.matches(&entry3)); // 不匹配include
    }
}