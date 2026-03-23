use gtk4::prelude::*;
use gtk4::{ApplicationWindow, Dialog, Entry, PasswordEntry, Label, Box, Button, Orientation, FileChooserDialog, FileChooserAction, ResponseType, ScrolledWindow, ListBox, ListBoxRow};
use std::path::PathBuf;
use std::collections::HashMap;

use crate::ssh::config::{SshConfig, AuthMethod};
use crate::config::{CustomLevelKeywords, SshServerConfig, Config};
use std::path::Path;

/// 显示 SSH 连接对话框
pub fn show_ssh_dialog<F>(parent: &ApplicationWindow, on_connect: F)
where
    F: Fn(SshConfig) + 'static,
{
    let dialog = Dialog::builder()
        .title("SSH Connection")
        .transient_for(parent)
        .modal(true)
        .default_width(400)
        .build();

    let content = dialog.content_area();
    content.set_spacing(12);
    content.set_margin_top(12);
    content.set_margin_bottom(12);
    content.set_margin_start(12);
    content.set_margin_end(12);

    // 连接名称
    let name_box = Box::new(Orientation::Horizontal, 6);
    let name_label = Label::builder()
        .label("Name:")
        .width_chars(12)
        .xalign(1.0)
        .build();
    let name_entry = Entry::builder()
        .placeholder_text("My Server")
        .hexpand(true)
        .build();
    name_box.append(&name_label);
    name_box.append(&name_entry);
    content.append(&name_box);

    // 主机地址
    let host_box = Box::new(Orientation::Horizontal, 6);
    let host_label = Label::builder()
        .label("Host:")
        .width_chars(12)
        .xalign(1.0)
        .build();
    let host_entry = Entry::builder()
        .placeholder_text("192.168.1.100 or example.com")
        .hexpand(true)
        .build();
    host_box.append(&host_label);
    host_box.append(&host_entry);
    content.append(&host_box);

    // 端口
    let port_box = Box::new(Orientation::Horizontal, 6);
    let port_label = Label::builder()
        .label("Port:")
        .width_chars(12)
        .xalign(1.0)
        .build();
    let port_entry = Entry::builder()
        .placeholder_text("22")
        .text("22")
        .hexpand(true)
        .build();
    port_box.append(&port_label);
    port_box.append(&port_entry);
    content.append(&port_box);

    // 用户名
    let user_box = Box::new(Orientation::Horizontal, 6);
    let user_label = Label::builder()
        .label("Username:")
        .width_chars(12)
        .xalign(1.0)
        .build();
    let user_entry = Entry::builder()
        .placeholder_text("root")
        .hexpand(true)
        .build();
    user_box.append(&user_label);
    user_box.append(&user_entry);
    content.append(&user_box);

    // 密码
    let pass_box = Box::new(Orientation::Horizontal, 6);
    let pass_label = Label::builder()
        .label("Password:")
        .width_chars(12)
        .xalign(1.0)
        .build();
    let pass_entry = PasswordEntry::builder()
        .hexpand(true)
        .show_peek_icon(true)
        .build();
    pass_box.append(&pass_label);
    pass_box.append(&pass_entry);
    content.append(&pass_box);

    // 按钮
    dialog.add_button("Cancel", ResponseType::Cancel);
    dialog.add_button("Connect", ResponseType::Accept);

    let dialog_clone = dialog.clone();
    let name_entry_clone = name_entry.clone();
    let host_entry_clone = host_entry.clone();
    let port_entry_clone = port_entry.clone();
    let user_entry_clone = user_entry.clone();
    let pass_entry_clone = pass_entry.clone();

    dialog.connect_response(move |dialog, response| {
        if response == ResponseType::Accept {
            let name = name_entry_clone.text().to_string();
            let host = host_entry_clone.text().to_string();
            let port = port_entry_clone.text().parse::<u16>().unwrap_or(22);
            let username = user_entry_clone.text().to_string();
            let password = pass_entry_clone.text().to_string();

            if !name.is_empty() && !host.is_empty() && !username.is_empty() {
                let config = SshConfig::new(name, host, username)
                    .with_port(port)
                    .with_password(password);
                on_connect(config);
            }
        }
        dialog.close();
    });

    dialog.present();
}

/// 显示文件选择对话框
pub fn show_file_dialog<F>(parent: &ApplicationWindow, on_select: F)
where
    F: Fn(PathBuf) + 'static,
{
    let dialog = FileChooserDialog::new(
        Some("Select Log File"),
        Some(parent),
        FileChooserAction::Open,
        &[
            ("Cancel", ResponseType::Cancel),
            ("Open", ResponseType::Accept),
        ],
    );

    dialog.connect_response(move |dialog, response| {
        if response == ResponseType::Accept {
            if let Some(file) = dialog.file() {
                if let Some(path) = file.path() {
                    on_select(path);
                }
            }
        }
        dialog.close();
    });

    dialog.present();
}

/// 显示错误对话框
pub fn show_error_dialog(parent: &ApplicationWindow, title: &str, message: &str) {
    let dialog = gtk4::MessageDialog::builder()
        .transient_for(parent)
        .modal(true)
        .message_type(gtk4::MessageType::Error)
        .buttons(gtk4::ButtonsType::Ok)
        .title(title)
        .text(message)
        .build();

    dialog.connect_response(|dialog, _| {
        dialog.close();
    });

    dialog.present();
}

/// 显示信息对话框
pub fn show_info_dialog(parent: &ApplicationWindow, title: &str, message: &str) {
    let dialog = gtk4::MessageDialog::builder()
        .transient_for(parent)
        .modal(true)
        .message_type(gtk4::MessageType::Info)
        .buttons(gtk4::ButtonsType::Ok)
        .title(title)
        .text(message)
        .build();

    dialog.connect_response(|dialog, _| {
        dialog.close();
    });

    dialog.present();
}

/// 显示确认对话框
pub fn show_confirm_dialog<F>(parent: &ApplicationWindow, title: &str, message: &str, on_confirm: F)
where
    F: Fn() + 'static,
{
    let dialog = gtk4::MessageDialog::builder()
        .transient_for(parent)
        .modal(true)
        .message_type(gtk4::MessageType::Question)
        .buttons(gtk4::ButtonsType::YesNo)
        .title(title)
        .text(message)
        .build();

    dialog.connect_response(move |dialog, response| {
        if response == ResponseType::Yes {
            on_confirm();
        }
        dialog.close();
    });

    dialog.present();
}

/// 显示自定义级别关键字设置对话框
pub fn show_custom_keywords_dialog<F>(
    parent: &ApplicationWindow,
    current_keywords: CustomLevelKeywords,
    on_save: F,
) where
    F: Fn(CustomLevelKeywords) + 'static,
{
    let dialog = Dialog::builder()
        .title("Custom Level Keywords")
        .transient_for(parent)
        .modal(true)
        .default_width(500)
        .default_height(400)
        .build();

    let content = dialog.content_area();
    content.set_spacing(12);
    content.set_margin_top(12);
    content.set_margin_bottom(12);
    content.set_margin_start(12);
    content.set_margin_end(12);

    // 说明标签
    let info_label = Label::builder()
        .label("Define custom keywords to detect log levels.\nKeywords are case-insensitive.")
        .wrap(true)
        .xalign(0.0)
        .build();
    content.append(&info_label);

    // 创建滚动区域
    let scrolled = ScrolledWindow::builder()
        .vexpand(true)
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .build();
    content.append(&scrolled);

    // 主容器
    let main_box = Box::new(Orientation::Vertical, 12);
    scrolled.set_child(Some(&main_box));

    // 级别配置
    let levels = vec![
        ("Verbose", current_keywords.verbose.clone()),
        ("Debug", current_keywords.debug.clone()),
        ("Info", current_keywords.info.clone()),
        ("Warn", current_keywords.warn.clone()),
        ("Error", current_keywords.error.clone()),
        ("Fatal", current_keywords.fatal.clone()),
    ];

    let mut level_entries: Vec<(String, Entry)> = Vec::new();

    for (level_name, keywords) in levels {
        let level_box = Box::new(Orientation::Horizontal, 6);
        
        let label = Label::builder()
            .label(&format!("{}:", level_name))
            .width_chars(10)
            .xalign(0.0)
            .build();
        level_box.append(&label);

        let entry = Entry::builder()
            .text(&keywords.join(", "))
            .placeholder_text("e.g., [v], [verbose]")
            .hexpand(true)
            .build();
        level_box.append(&entry);

        level_entries.push((level_name.to_lowercase(), entry));
        main_box.append(&level_box);
    }

    // 按钮
    dialog.add_button("Cancel", ResponseType::Cancel);
    dialog.add_button("Save", ResponseType::Accept);

    let dialog_clone = dialog.clone();

    dialog.connect_response(move |dialog, response| {
        if response == ResponseType::Accept {
            let mut new_keywords = CustomLevelKeywords::default();
            
            for (level, entry) in &level_entries {
                let text = entry.text().to_string();
                let keywords: Vec<String> = text
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                
                match level.as_str() {
                    "verbose" => new_keywords.verbose = keywords,
                    "debug" => new_keywords.debug = keywords,
                    "info" => new_keywords.info = keywords,
                    "warn" => new_keywords.warn = keywords,
                    "error" => new_keywords.error = keywords,
                    "fatal" => new_keywords.fatal = keywords,
                    _ => {}
                }
            }
            
            on_save(new_keywords);
        }
        dialog.close();
    });

    dialog.present();
}

/// 显示设置导出对话框
pub fn show_export_settings_dialog<F>(parent: &ApplicationWindow, on_export: F)
where
    F: Fn(PathBuf) + 'static,
{
    let dialog = FileChooserDialog::new(
        Some("Export Settings"),
        Some(parent),
        FileChooserAction::Save,
        &[
            ("Cancel", ResponseType::Cancel),
            ("Export", ResponseType::Accept),
        ],
    );
    
    // 设置默认文件名
    dialog.set_current_name("ilogcat-settings.toml");
    
    // 添加文件过滤器
    let filter = gtk4::FileFilter::new();
    filter.set_name(Some("TOML files"));
    filter.add_pattern("*.toml");
    dialog.add_filter(&filter);
    
    let filter_all = gtk4::FileFilter::new();
    filter_all.set_name(Some("All files"));
    filter_all.add_pattern("*");
    dialog.add_filter(&filter_all);

    dialog.connect_response(move |dialog, response| {
        if response == ResponseType::Accept {
            if let Some(file) = dialog.file() {
                if let Some(path) = file.path() {
                    // 确保文件扩展名是 .toml
                    let path = if path.extension().is_none() {
                        path.with_extension("toml")
                    } else {
                        path
                    };
                    on_export(path);
                }
            }
        }
        dialog.close();
    });

    dialog.present();
}

/// 显示设置导入对话框
pub fn show_import_settings_dialog<F>(parent: &ApplicationWindow, on_import: F)
where
    F: Fn(PathBuf) + 'static,
{
    let dialog = FileChooserDialog::new(
        Some("Import Settings"),
        Some(parent),
        FileChooserAction::Open,
        &[
            ("Cancel", ResponseType::Cancel),
            ("Import", ResponseType::Accept),
        ],
    );
    
    // 添加文件过滤器
    let filter = gtk4::FileFilter::new();
    filter.set_name(Some("TOML files"));
    filter.add_pattern("*.toml");
    dialog.add_filter(&filter);
    
    let filter_all = gtk4::FileFilter::new();
    filter_all.set_name(Some("All files"));
    filter_all.add_pattern("*");
    dialog.add_filter(&filter_all);

    dialog.connect_response(move |dialog, response| {
        if response == ResponseType::Accept {
            if let Some(file) = dialog.file() {
                if let Some(path) = file.path() {
                    on_import(path);
                }
            }
        }
        dialog.close();
    });

    dialog.present();
}

/// 显示 SSH 命令执行对话框
/// 允许用户在已连接的 SSH 服务器上执行自定义命令
pub fn show_ssh_command_dialog<F>(
    parent: &ApplicationWindow,
    saved_servers: Vec<SshServerConfig>,
    on_execute: F,
) where
    F: Fn(SshServerConfig, String) + 'static,
{
    let dialog = Dialog::builder()
        .title("Execute SSH Command")
        .transient_for(parent)
        .modal(true)
        .default_width(500)
        .default_height(300)
        .build();

    let content = dialog.content_area();
    content.set_spacing(12);
    content.set_margin_top(12);
    content.set_margin_bottom(12);
    content.set_margin_start(12);
    content.set_margin_end(12);

    // 服务器选择
    let server_box = Box::new(Orientation::Horizontal, 6);
    let server_label = Label::builder()
        .label("Server:")
        .width_chars(12)
        .xalign(1.0)
        .build();
    
    let server_names: Vec<String> = saved_servers.iter()
        .map(|s| format!("{} ({}@{})", s.name, s.username, s.host))
        .collect();
    
    let server_combo = if !server_names.is_empty() {
        gtk4::DropDown::from_strings(&server_names.iter().map(|s| s.as_str()).collect::<Vec<_>>())
    } else {
        gtk4::DropDown::from_strings(&["No saved servers"])
    };
    server_combo.set_hexpand(true);
    
    server_box.append(&server_label);
    server_box.append(&server_combo);
    content.append(&server_box);

    // 命令输入
    let command_box = Box::new(Orientation::Vertical, 6);
    let command_label = Label::builder()
        .label("Command:")
        .xalign(0.0)
        .build();
    
    let command_entry = Entry::builder()
        .placeholder_text("e.g., tail -f /var/log/app.log")
        .hexpand(true)
        .build();
    
    // 常用命令建议
    let suggestions_label = Label::builder()
        .label("Suggestions: dmesg -w, journalctl -f, tail -f /var/log/syslog")
        .wrap(true)
        .xalign(0.0)
        .css_classes(vec!["dim-label".to_string()])
        .build();
    
    command_box.append(&command_label);
    command_box.append(&command_entry);
    command_box.append(&suggestions_label);
    content.append(&command_box);

    // 按钮
    dialog.add_button("Cancel", ResponseType::Cancel);
    if !saved_servers.is_empty() {
        dialog.add_button("Execute", ResponseType::Accept);
    }

    let dialog_clone = dialog.clone();
    let server_combo_clone = server_combo.clone();
    let command_entry_clone = command_entry.clone();
    let saved_servers_clone = saved_servers.clone();

    dialog.connect_response(move |dialog, response| {
        if response == ResponseType::Accept && !saved_servers_clone.is_empty() {
            let idx = server_combo_clone.selected() as usize;
            if idx < saved_servers_clone.len() {
                let server = saved_servers_clone[idx].clone();
                let command = command_entry_clone.text().to_string();
                if !command.is_empty() {
                    on_execute(server, command);
                }
            }
        }
        dialog.close();
    });

    dialog.present();
}

/// 显示主题设置对话框
pub fn show_theme_dialog<F>(
    parent: &ApplicationWindow,
    current_theme: &str,
    on_save: F,
) where
    F: Fn(String) + 'static,
{
    let dialog = Dialog::builder()
        .title("Theme Settings")
        .transient_for(parent)
        .modal(true)
        .default_width(300)
        .build();

    let content = dialog.content_area();
    content.set_spacing(12);
    content.set_margin_top(12);
    content.set_margin_bottom(12);
    content.set_margin_start(12);
    content.set_margin_end(12);

    // 说明标签
    let info_label = Label::builder()
        .label("Select application theme:")
        .wrap(true)
        .xalign(0.0)
        .build();
    content.append(&info_label);

    // 主题选择
    let theme_box = Box::new(Orientation::Vertical, 12);
    
    // Light theme option
    let light_radio = gtk4::CheckButton::builder()
        .label("Light Theme")
        .build();
    
    // Dark theme option
    let dark_radio = gtk4::CheckButton::builder()
        .label("Dark Theme")
        .group(&light_radio)
        .build();
    
    // 设置当前选中状态
    if current_theme == "dark" {
        dark_radio.set_active(true);
    } else {
        light_radio.set_active(true);
    }
    
    theme_box.append(&light_radio);
    theme_box.append(&dark_radio);
    content.append(&theme_box);

    // 按钮
    dialog.add_button("Cancel", ResponseType::Cancel);
    dialog.add_button("Save", ResponseType::Accept);

    let light_radio_clone = light_radio.clone();
    let dark_radio_clone = dark_radio.clone();

    dialog.connect_response(move |dialog, response| {
        if response == ResponseType::Accept {
            let theme = if dark_radio_clone.is_active() {
                "dark".to_string()
            } else {
                "light".to_string()
            };
            on_save(theme);
        }
        dialog.close();
    });

    dialog.present();
}

/// 显示语言设置对话框
pub fn show_language_dialog<F>(
    parent: &ApplicationWindow,
    current_language: &str,
    on_save: F,
) where
    F: Fn(String) + 'static,
{
    let dialog = Dialog::builder()
        .title("Language Settings")
        .transient_for(parent)
        .modal(true)
        .default_width(300)
        .build();

    let content = dialog.content_area();
    content.set_spacing(12);
    content.set_margin_top(12);
    content.set_margin_bottom(12);
    content.set_margin_start(12);
    content.set_margin_end(12);

    // 说明标签
    let info_label = Label::builder()
        .label("Select application language:")
        .wrap(true)
        .xalign(0.0)
        .build();
    content.append(&info_label);

    // 语言选择
    let lang_box = Box::new(Orientation::Vertical, 12);
    
    // English option
    let en_radio = gtk4::CheckButton::builder()
        .label("English")
        .build();
    
    // Chinese option
    let zh_radio = gtk4::CheckButton::builder()
        .label("中文 (Chinese)")
        .group(&en_radio)
        .build();
    
    // 设置当前选中状态
    if current_language == "zh" {
        zh_radio.set_active(true);
    } else {
        en_radio.set_active(true);
    }
    
    lang_box.append(&en_radio);
    lang_box.append(&zh_radio);
    content.append(&lang_box);

    // 按钮
    dialog.add_button("Cancel", ResponseType::Cancel);
    dialog.add_button("Save", ResponseType::Accept);

    let en_radio_clone = en_radio.clone();
    let zh_radio_clone = zh_radio.clone();

    dialog.connect_response(move |dialog, response| {
        if response == ResponseType::Accept {
            let lang = if zh_radio_clone.is_active() {
                "zh".to_string()
            } else {
                "en".to_string()
            };
            on_save(lang);
        }
        dialog.close();
    });

    dialog.present();
}
