use gtk4::prelude::*;
use gtk4::{ApplicationWindow, Dialog, Entry, PasswordEntry, Label, Box, Button, Orientation, FileChooserDialog, FileChooserAction, ResponseType, ScrolledWindow, ListBox, ListBoxRow};
use std::path::PathBuf;
use std::collections::HashMap;

use crate::ssh::config::{SshConfig, AuthMethod};
use crate::config::CustomLevelKeywords;

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
