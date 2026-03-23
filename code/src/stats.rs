use gtk4::prelude::*;
use gtk4::{Box, Label, Orientation, ProgressBar, Grid, ScrolledWindow, Frame};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::log::LogLevel;

/// 日志统计信息
#[derive(Debug)]
pub struct LogStatistics {
    /// 各级别日志数量
    pub level_counts: HashMap<LogLevel, usize>,
    /// 总日志数
    pub total_count: usize,
    /// 过滤后的日志数
    pub filtered_count: usize,
    /// 每秒日志速率
    pub logs_per_second: f64,
    /// 启动时间
    pub start_time: std::time::Instant,
    /// 最后更新时间
    pub last_update: std::time::Instant,
}

impl LogStatistics {
    pub fn new() -> Self {
        let now = std::time::Instant::now();
        Self {
            level_counts: HashMap::new(),
            total_count: 0,
            filtered_count: 0,
            logs_per_second: 0.0,
            start_time: now,
            last_update: now,
        }
    }

    /// 添加日志条目
    pub fn add_log(&mut self, level: LogLevel, is_filtered: bool) {
        self.total_count += 1;
        if !is_filtered {
            self.filtered_count += 1;
        }
        *self.level_counts.entry(level).or_insert(0) += 1;
        self.update_rate();
    }

    /// 更新日志速率
    fn update_rate(&mut self) {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            self.logs_per_second = self.total_count as f64 / elapsed;
        }
        self.last_update = std::time::Instant::now();
    }

    /// 获取指定级别的数量
    pub fn get_level_count(&self, level: LogLevel) -> usize {
        self.level_counts.get(&level).copied().unwrap_or(0)
    }

    /// 获取指定级别的百分比
    pub fn get_level_percentage(&self, level: LogLevel) -> f64 {
        if self.total_count == 0 {
            return 0.0;
        }
        let count = self.get_level_count(level);
        (count as f64 / self.total_count as f64) * 100.0
    }

    /// 获取运行时间（秒）
    pub fn get_uptime_seconds(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    /// 格式化运行时间
    pub fn format_uptime(&self) -> String {
        let seconds = self.get_uptime_seconds();
        let hours = seconds / 3600;
        let minutes = (seconds % 3600) / 60;
        let secs = seconds % 60;
        format!("{:02}:{:02}:{:02}", hours, minutes, secs)
    }

    /// 重置统计
    pub fn reset(&mut self) {
        self.level_counts.clear();
        self.total_count = 0;
        self.filtered_count = 0;
        self.logs_per_second = 0.0;
        self.start_time = std::time::Instant::now();
        self.last_update = std::time::Instant::now();
    }
}

/// 统计面板组件
pub struct StatsPanel {
    pub widget: Box,
    stats: RefCell<LogStatistics>,
    // 显示组件
    total_label: Label,
    filtered_label: Label,
    rate_label: Label,
    uptime_label: Label,
    level_labels: HashMap<LogLevel, Label>,
    level_bars: HashMap<LogLevel, ProgressBar>,
}

impl StatsPanel {
    pub fn new() -> Rc<RefCell<Self>> {
        let stats = LogStatistics::new();

        // 创建主容器
        let widget = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(12)
            .margin_top(12)
            .margin_bottom(12)
            .margin_start(12)
            .margin_end(12)
            .build();

        // 标题
        let title = Label::builder()
            .label("Log Statistics")
            .css_classes(vec!["title-2".to_string()])
            .halign(gtk4::Align::Start)
            .build();
        widget.append(&title);

        // 概览区域
        let overview_frame = Frame::builder()
            .label("Overview")
            .margin_top(6)
            .build();
        let overview_grid = Grid::builder()
            .row_spacing(6)
            .column_spacing(12)
            .margin_top(6)
            .margin_bottom(6)
            .margin_start(6)
            .margin_end(6)
            .build();

        // 总日志数
        let total_label = Label::new(Some("0"));
        total_label.set_css_classes(&["numeric"]);
        overview_grid.attach(&Label::new(Some("Total Logs:")), 0, 0, 1, 1);
        overview_grid.attach(&total_label, 1, 0, 1, 1);

        // 过滤后日志数
        let filtered_label = Label::new(Some("0"));
        filtered_label.set_css_classes(&["numeric"]);
        overview_grid.attach(&Label::new(Some("Filtered:")), 0, 1, 1, 1);
        overview_grid.attach(&filtered_label, 1, 1, 1, 1);

        // 日志速率
        let rate_label = Label::new(Some("0.0 /s"));
        rate_label.set_css_classes(&["numeric"]);
        overview_grid.attach(&Label::new(Some("Rate:")), 0, 2, 1, 1);
        overview_grid.attach(&rate_label, 1, 2, 1, 1);

        // 运行时间
        let uptime_label = Label::new(Some("00:00:00"));
        uptime_label.set_css_classes(&["numeric"]);
        overview_grid.attach(&Label::new(Some("Uptime:")), 0, 3, 1, 1);
        overview_grid.attach(&uptime_label, 1, 3, 1, 1);

        overview_frame.set_child(Some(&overview_grid));
        widget.append(&overview_frame);

        // 日志级别分布
        let levels_frame = Frame::builder()
            .label("Level Distribution")
            .margin_top(6)
            .build();
        let levels_box = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(6)
            .margin_top(6)
            .margin_bottom(6)
            .margin_start(6)
            .margin_end(6)
            .build();

        let mut level_labels = HashMap::new();
        let mut level_bars = HashMap::new();

        let levels = [
            (LogLevel::Fatal, "#CC0000", "Fatal"),
            (LogLevel::Error, "#CC0000", "Error"),
            (LogLevel::Warn, "#FF8800", "Warn"),
            (LogLevel::Info, "#008800", "Info"),
            (LogLevel::Debug, "#0066CC", "Debug"),
            (LogLevel::Verbose, "#808080", "Verbose"),
        ];

        for (level, color, name) in levels.iter() {
            let row = Box::builder()
                .orientation(Orientation::Horizontal)
                .spacing(6)
                .build();

            // 级别名称
            let name_label = Label::new(Some(*name));
            name_label.set_width_chars(10);
            name_label.set_xalign(0.0);
            row.append(&name_label);

            // 进度条
            let bar = ProgressBar::builder()
                .hexpand(true)
                .show_text(false)
                .build();
            bar.add_css_class("level-bar");
            row.append(&bar);

            // 数量标签
            let count_label = Label::new(Some("0"));
            count_label.set_width_chars(8);
            count_label.set_xalign(1.0);
            row.append(&count_label);

            levels_box.append(&row);
            level_labels.insert(*level, count_label);
            level_bars.insert(*level, bar);
        }

        levels_frame.set_child(Some(&levels_box));
        widget.append(&levels_frame);

        let panel = Rc::new(RefCell::new(Self {
            widget,
            stats: RefCell::new(stats),
            total_label,
            filtered_label,
            rate_label,
            uptime_label,
            level_labels,
            level_bars,
        }));

        // 启动定时更新
        Self::start_update_timer(panel.clone());

        panel
    }

    /// 添加日志到统计
    pub fn add_log(&self, level: LogLevel, is_filtered: bool) {
        self.stats.borrow_mut().add_log(level, is_filtered);
        self.update_display();
    }

    /// 更新显示
    fn update_display(&self) {
        let stats = self.stats.borrow();

        // 更新概览
        self.total_label.set_text(&stats.total_count.to_string());
        self.filtered_label.set_text(&stats.filtered_count.to_string());
        self.rate_label.set_text(&format!("{:.1} /s", stats.logs_per_second));
        self.uptime_label.set_text(&stats.format_uptime());

        // 更新级别分布
        for (level, label) in &self.level_labels {
            let count = stats.get_level_count(*level);
            let percentage = stats.get_level_percentage(*level);
            label.set_text(&format!("{} ({:.1}%)", count, percentage));

            if let Some(bar) = self.level_bars.get(level) {
                bar.set_fraction(percentage / 100.0);
            }
        }
    }

    /// 启动定时更新器
    fn start_update_timer(panel: Rc<RefCell<Self>>) {
        glib::timeout_add_seconds_local(1, move || {
            panel.borrow().update_display();
            glib::ControlFlow::Continue
        });
    }

    /// 获取统计信息副本
    pub fn get_stats(&self) -> LogStatistics {
        self.stats.borrow().clone()
    }

    /// 重置统计
    pub fn reset(&self) {
        self.stats.borrow_mut().reset();
        self.update_display();
    }

    /// 获取 widget 引用
    pub fn widget(&self) -> &Box {
        &self.widget
    }
}

impl Clone for LogStatistics {
    fn clone(&self) -> Self {
        Self {
            level_counts: self.level_counts.clone(),
            total_count: self.total_count,
            filtered_count: self.filtered_count,
            logs_per_second: self.logs_per_second,
            start_time: self.start_time,
            last_update: self.last_update,
        }
    }
}

impl Default for LogStatistics {
    fn default() -> Self {
        Self::new()
    }
}

/// 统计对话框
pub struct StatsDialog;

impl StatsDialog {
    /// 显示统计对话框
    pub fn show(parent: &gtk4::Window, stats: &LogStatistics) {
        let dialog = gtk4::Dialog::new();
        dialog.set_title(Some("Log Statistics"));
        dialog.set_transient_for(Some(parent));
        dialog.set_modal(true);
        dialog.set_default_size(400, 500);

        let content = dialog.content_area();
        content.set_spacing(12);
        content.set_margin_top(12);
        content.set_margin_bottom(12);
        content.set_margin_start(12);
        content.set_margin_end(12);

        // 创建统计面板
        let panel = StatsPanel::new();
        // 复制统计数据
        *panel.borrow().stats.borrow_mut() = stats.clone();
        panel.borrow().update_display();

        content.append(panel.borrow().widget());

        dialog.add_button("Close", gtk4::ResponseType::Close);
        dialog.add_button("Reset", gtk4::ResponseType::Other(1));

        let panel_clone = panel.clone();
        dialog.connect_response(move |dlg, response| {
            if response == gtk4::ResponseType::Other(1) {
                panel_clone.borrow().reset();
            } else {
                dlg.close();
            }
        });

        dialog.show();
    }
}