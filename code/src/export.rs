use std::fs::File;
use std::io::Write;
use std::path::Path;
use gtk4::prelude::*;
use gtk4::{TextBuffer, FileChooserDialog, FileChooserAction, ResponseType};
use crate::log::LogEntry;
use crate::log::LogLevel;

/// 导出格式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExportFormat {
    /// 纯文本格式
    Text,
    /// CSV格式
    Csv,
}

impl ExportFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            ExportFormat::Text => "txt",
            ExportFormat::Csv => "csv",
        }
    }

    pub fn mime_type(&self) -> &'static str {
        match self {
            ExportFormat::Text => "text/plain",
            ExportFormat::Csv => "text/csv",
        }
    }
}

/// 导出管理器
pub struct ExportManager;

impl ExportManager {
    pub fn new() -> Self {
        Self
    }

    /// 导出文本缓冲区内容到文件
    pub fn export_text_buffer<P: AsRef<Path>>(
        &self,
        buffer: &TextBuffer,
        path: P,
        format: ExportFormat,
    ) -> Result<(), String> {
        let text = buffer
            .text(&buffer.start_iter(), &buffer.end_iter(), false)
            .to_string();

        match format {
            ExportFormat::Text => self.export_as_text(&text, path),
            ExportFormat::Csv => self.export_as_csv(&text, path),
        }
    }

    /// 导出为纯文本
    fn export_as_text<P: AsRef<Path>>(&self, text: &str, path: P) -> Result<(), String> {
        let mut file = File::create(path).map_err(|e| format!("Failed to create file: {}", e))?;
        file.write_all(text.as_bytes())
            .map_err(|e| format!("Failed to write file: {}", e))?;
        Ok(())
    }

    /// 导出为CSV格式
    fn export_as_csv<P: AsRef<Path>>(&self, text: &str, path: P) -> Result<(), String> {
        let mut file = File::create(path).map_err(|e| format!("Failed to create file: {}", e))?;

        // CSV 表头
        writeln!(file, "Timestamp,Level,Tag,Message")
            .map_err(|e| format!("Failed to write header: {}", e))?;

        // 解析每一行并转换为CSV
        for line in text.lines() {
            if line.trim().is_empty() {
                continue;
            }

            // 尝试解析日志格式
            // 格式示例: [2024-03-20 10:30:45] [INFO] tag: message
            let parsed = self.parse_log_line(line);
            match parsed {
                Some((timestamp, level, tag, message)) => {
                    writeln!(file, "{},{},{},\"{}\"", timestamp, level, tag, message)
                        .map_err(|e| format!("Failed to write line: {}", e))?;
                }
                None => {
                    // 无法解析的行，作为纯消息导出
                    writeln!(file, ",,,\"{}\"", line.replace("\"", "\"\""))
                        .map_err(|e| format!("Failed to write line: {}", e))?;
                }
            }
        }

        Ok(())
    }

    /// 解析日志行
    /// 支持格式: [timestamp] [LEVEL] tag: message
    fn parse_log_line(&self, line: &str) -> Option<(String, String, String, String)> {
        // 尝试匹配 [timestamp] [LEVEL] tag: message 格式
        if let Some(start_idx) = line.find('[') {
            if let Some(end_idx) = line[start_idx + 1..].find(']') {
                let timestamp = line[start_idx + 1..start_idx + 1 + end_idx].to_string();
                let rest = &line[start_idx + 1 + end_idx + 1..];

                // 查找级别
                if let Some(level_start) = rest.find('[') {
                    if let Some(level_end) = rest[level_start + 1..].find(']') {
                        let level = rest[level_start + 1..level_start + 1 + level_end].to_string();
                        let after_level = &rest[level_start + 1 + level_end + 1..];

                        // 查找 tag 和 message（用冒号分隔）
                        if let Some(colon_idx) = after_level.find(':') {
                            let tag = after_level[..colon_idx].trim().to_string();
                            let message = after_level[colon_idx + 1..].trim().to_string();
                            return Some((timestamp, level, tag, message));
                        } else {
                            // 没有tag，整个作为message
                            return Some((timestamp, level, "".to_string(), after_level.trim().to_string()));
                        }
                    }
                }
            }
        }

        // 尝试匹配简单的级别前缀格式
        let levels = ["VERBOSE", "DEBUG", "INFO", "WARN", "ERROR", "FATAL"];
        for level in &levels {
            if line.contains(&format!("[{}]", level)) {
                // 提取时间戳（假设在行首）
                let timestamp = if let Some(space_idx) = line.find(' ') {
                    line[..space_idx].to_string()
                } else {
                    "".to_string()
                };

                // 提取消息（级别后的内容）
                let message = if let Some(idx) = line.find(&format!("[{}]", level)) {
                    line[idx + level.len() + 2..].trim().to_string()
                } else {
                    line.to_string()
                };

                return Some((timestamp, level.to_string(), "".to_string(), message));
            }
        }

        None
    }

    /// 显示导出对话框
    pub fn show_export_dialog<F: Fn(ExportFormat, String) + 'static>(
        parent: &gtk4::Window,
        callback: F,
    ) {
        let dialog = FileChooserDialog::new(
            Some("Export Logs"),
            Some(parent),
            FileChooserAction::Save,
            &[("Cancel", ResponseType::Cancel), ("Export", ResponseType::Accept)],
        );

        // 添加格式选择
        let format_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        let label = gtk4::Label::new(Some("Format:"));
        format_box.append(&label);

        let format_combo = gtk4::ComboBoxText::new();
        format_combo.append(Some("text"), "Text (.txt)");
        format_combo.append(Some("csv"), "CSV (.csv)");
        format_combo.set_active_id(Some("text"));
        format_box.append(&format_combo);

        dialog.set_extra_widget(Some(&format_box));

        // 设置默认文件名
        dialog.set_current_name("ilogcat_export.txt");

        // 监听格式变化
        format_combo.connect_changed(clone!(dialog => move |combo| {
            let name = if combo.active_id().as_deref() == Some("csv") {
                "ilogcat_export.csv"
            } else {
                "ilogcat_export.txt"
            };
            dialog.set_current_name(name);
        }));

        dialog.connect_response(move |dialog, response| {
            if response == ResponseType::Accept {
                if let Some(path) = dialog.file().and_then(|f| f.path()) {
                    let format = if format_combo.active_id().as_deref() == Some("csv") {
                        ExportFormat::Csv
                    } else {
                        ExportFormat::Text
                    };
                    callback(format, path.to_string_lossy().to_string());
                }
            }
            dialog.close();
        });

        dialog.show();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_log_line() {
        let manager = ExportManager::new();

        // 测试标准格式
        let line = "[2024-03-20 10:30:45] [INFO] MyApp: Application started";
        let result = manager.parse_log_line(line);
        assert!(result.is_some());
        let (ts, level, tag, msg) = result.unwrap();
        assert_eq!(ts, "2024-03-20 10:30:45");
        assert_eq!(level, "INFO");
        assert_eq!(tag, "MyApp");
        assert_eq!(msg, "Application started");

        // 测试无tag格式
        let line2 = "[2024-03-20 10:30:45] [ERROR] Something went wrong";
        let result2 = manager.parse_log_line(line2);
        assert!(result2.is_some());
    }
}
