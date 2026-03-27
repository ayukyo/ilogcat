use gtk4::prelude::*;
use gtk4::{ApplicationWindow, Dialog, Entry, PasswordEntry, Label, Box, Button, Orientation, FileChooserDialog, FileChooserAction, ResponseType, ScrolledWindow, ListBox, ListBoxRow};
use std::path::PathBuf;
use std::collections::HashMap;
use std::rc::Rc;

use crate::ssh::config::{SshConfig, AuthMethod};
use crate::config::{CustomLevelKeywords, Config, LastSshInput};
use std::path::Path;
use crate::i18n::{t, I18nKey};

/// 显示 SSH 连接对话框
pub fn show_ssh_dialog<F>(parent: &ApplicationWindow, last_input: Option<&LastSshInput>, on_connect: F)
where
    F: Fn(SshConfig) + 'static,
{
    let dialog = Dialog::builder()
        .title(&t(I18nKey::DialogSshConnection))
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
        .label(&t(I18nKey::LabelName))
        .width_chars(12)
        .xalign(1.0)
        .build();
    let name_text = last_input.map(|i| i.name.clone()).unwrap_or_default();
    let name_entry = Entry::builder()
        .placeholder_text("My Server")
        .hexpand(true)
        .text(&name_text)
        .build();
    name_box.append(&name_label);
    name_box.append(&name_entry);
    content.append(&name_box);

    // 主机地址
    let host_box = Box::new(Orientation::Horizontal, 6);
    let host_label = Label::builder()
        .label(&t(I18nKey::LabelHost))
        .width_chars(12)
        .xalign(1.0)
        .build();
    let host_text = last_input.map(|i| i.host.clone()).unwrap_or_default();
    let host_entry = Entry::builder()
        .placeholder_text("192.168.1.100 or example.com")
        .hexpand(true)
        .text(&host_text)
        .build();
    host_box.append(&host_label);
    host_box.append(&host_entry);
    content.append(&host_box);

    // 端口
    let port_box = Box::new(Orientation::Horizontal, 6);
    let port_label = Label::builder()
        .label(&t(I18nKey::LabelPort))
        .width_chars(12)
        .xalign(1.0)
        .build();
    let port_text = last_input.map(|i| i.port.to_string()).unwrap_or_else(|| "22".to_string());
    let port_entry = Entry::builder()
        .placeholder_text("22")
        .text(&port_text)
        .hexpand(true)
        .build();
    port_box.append(&port_label);
    port_box.append(&port_entry);
    content.append(&port_box);

    // 用户名
    let user_box = Box::new(Orientation::Horizontal, 6);
    let user_label = Label::builder()
        .label(&t(I18nKey::LabelUsername))
        .width_chars(12)
        .xalign(1.0)
        .build();
    let user_text = last_input.map(|i| i.username.clone()).unwrap_or_default();
    let user_entry = Entry::builder()
        .placeholder_text("root")
        .hexpand(true)
        .text(&user_text)
        .build();
    user_box.append(&user_label);
    user_box.append(&user_entry);
    content.append(&user_box);

    // 密码
    let pass_box = Box::new(Orientation::Horizontal, 6);
    let pass_label = Label::builder()
        .label(&t(I18nKey::LabelPassword))
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
    dialog.add_button(&t(I18nKey::ButtonCancel), ResponseType::Cancel);
    dialog.add_button(&t(I18nKey::ButtonConnect), ResponseType::Accept);

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
        .title(&t(I18nKey::DialogError))
        .text(title)
        .secondary_text(message)
        .build();

    dialog.connect_response(|dialog, _| {
        dialog.close();
    });

    dialog.present();
}

/// 日志源类型
#[derive(Clone, Debug)]
pub enum LocalSourceType {
    Dmesg,
    Journalctl,
    File(PathBuf),
}

/// 显示选择本地日志源对话框
pub fn show_select_source_dialog<F>(parent: &ApplicationWindow, on_select: F)
where
    F: Fn(LocalSourceType) + 'static + Clone,
{
    let dialog = Dialog::builder()
        .title(&t(I18nKey::DialogSelectSource))
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

    // dmesg 选项
    let dmesg_btn = gtk4::Button::builder()
        .label(&format!("🖥️ {}", &t(I18nKey::SourceDmesg)))
        .tooltip_text(&t(I18nKey::SourceDmesgDesc))
        .hexpand(true)
        .height_request(50)
        .build();
    content.append(&dmesg_btn);

    // journalctl 选项
    let journalctl_btn = gtk4::Button::builder()
        .label(&format!("📋 {}", &t(I18nKey::SourceJournalctl)))
        .tooltip_text(&t(I18nKey::SourceJournalctlDesc))
        .hexpand(true)
        .height_request(50)
        .build();
    content.append(&journalctl_btn);

    // 文件选项
    let file_btn = gtk4::Button::builder()
        .label(&format!("📁 {}", &t(I18nKey::SourceFile)))
        .tooltip_text(&t(I18nKey::SourceFileDesc))
        .hexpand(true)
        .height_request(50)
        .build();
    content.append(&file_btn);

    // 取消按钮
    dialog.add_button(&t(I18nKey::ButtonCancel), ResponseType::Cancel);

    // 使用 Rc 来共享回调
    let on_select_rc = Rc::new(on_select.clone());

    // dmesg 点击事件
    let dialog_clone = dialog.clone();
    let on_select_clone = on_select_rc.clone();
    dmesg_btn.connect_clicked(move |_| {
        on_select_clone(LocalSourceType::Dmesg);
        dialog_clone.close();
    });

    // journalctl 点击事件
    let dialog_clone = dialog.clone();
    let on_select_clone = on_select_rc.clone();
    journalctl_btn.connect_clicked(move |_| {
        on_select_clone(LocalSourceType::Journalctl);
        dialog_clone.close();
    });

    // 文件点击事件 - 需要打开文件选择对话框
    let parent_clone = parent.clone();
    let dialog_clone = dialog.clone();
    let on_select_clone = on_select_rc;
    file_btn.connect_clicked(move |_| {
        let parent_for_file = parent_clone.clone();
        let on_select_for_file = (*on_select_clone).clone();
        show_file_dialog(&parent_for_file, move |path| {
            on_select_for_file(LocalSourceType::File(path));
        });
        dialog_clone.close();
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
        .title(&t(I18nKey::DialogInfo))
        .text(title)
        .secondary_text(message)
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
        .title(&t(I18nKey::DialogConfirm))
        .text(title)
        .secondary_text(message)
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
        .title(&t(I18nKey::DialogCustomKeywords))
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
        .label(&t(I18nKey::CustomKeywordsInfo))
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
    let verbose_str = t(I18nKey::LevelVerbose);
    let debug_str = t(I18nKey::LevelDebug);
    let info_str = t(I18nKey::LevelInfo);
    let warn_str = t(I18nKey::LevelWarn);
    let error_str = t(I18nKey::LevelError);
    let fatal_str = t(I18nKey::LevelFatal);

    let levels: Vec<(String, Vec<String>)> = vec![
        (verbose_str, current_keywords.verbose.clone()),
        (debug_str, current_keywords.debug.clone()),
        (info_str, current_keywords.info.clone()),
        (warn_str, current_keywords.warn.clone()),
        (error_str, current_keywords.error.clone()),
        (fatal_str, current_keywords.fatal.clone()),
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
            .placeholder_text(&t(I18nKey::CustomKeywordsPlaceholder))
            .hexpand(true)
            .build();
        level_box.append(&entry);

        level_entries.push((level_name.to_lowercase(), entry));
        main_box.append(&level_box);
    }

    // 按钮
    dialog.add_button(&t(I18nKey::ButtonCancel), ResponseType::Cancel);
    dialog.add_button(&t(I18nKey::ButtonSave), ResponseType::Accept);

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
            (&t(I18nKey::ButtonCancel), ResponseType::Cancel),
            ("Export", ResponseType::Accept),
        ],
    );

    dialog.set_modal(true);
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
        if response == ResponseType::Accept || response == ResponseType::Ok {
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
        dialog.destroy();
    });

    dialog.present();
}

/// 显示设置导入对话框
pub fn show_import_settings_dialog<F>(parent: &ApplicationWindow, on_import: F)
where
    F: Fn(PathBuf) + 'static,
{
    let dialog = FileChooserDialog::new(
        Some(&t(I18nKey::DialogImportSettings)),
        Some(parent),
        FileChooserAction::Open,
        &[
            (&t(I18nKey::ButtonCancel), ResponseType::Cancel),
            ("Import", ResponseType::Accept),
        ],
    );

    dialog.set_modal(true);

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
        if response == ResponseType::Accept || response == ResponseType::Ok {
            if let Some(file) = dialog.file() {
                if let Some(path) = file.path() {
                    on_import(path);
                }
            }
        }
        dialog.destroy();
    });

    dialog.present();
}

/// SSH 连接状态
#[derive(Debug, Clone)]
pub enum SshConnectionState {
    ResolvingHost,
    Connecting,
    Authenticating,
    Connected,
    Failed(String),
}

/// 显示 SSH 连接进度对话框
pub fn show_ssh_connection_progress_dialog<F>(
    parent: &ApplicationWindow,
    host: &str,
    on_state_change: F,
) -> gtk4::MessageDialog
where
    F: Fn(SshConnectionState) + 'static,
{
    let dialog = gtk4::MessageDialog::builder()
        .transient_for(parent)
        .modal(true)
        .message_type(gtk4::MessageType::Info)
        .buttons(gtk4::ButtonsType::Cancel)
        .title(&t(I18nKey::DialogSshConnection))
        .text(&format!("{}: {}", &t(I18nKey::LabelHost), host))
        .secondary_text(&t(I18nKey::StatusConnecting))
        .build();

    let on_state_change = std::rc::Rc::new(std::cell::RefCell::new(on_state_change));
    dialog.connect_response(move |dialog, response| {
        if response == ResponseType::Cancel {
            (*on_state_change.borrow())(SshConnectionState::Failed("Cancelled by user".to_string()));
        }
        dialog.close();
    });

    dialog.present();
    dialog
}

/// 更新 SSH 连接进度对话框状态
pub fn update_ssh_progress_dialog(
    dialog: &gtk4::MessageDialog,
    state: &SshConnectionState,
) {
    let secondary_text = match state {
        SshConnectionState::ResolvingHost => "Resolving host address...",
        SshConnectionState::Connecting => "Connecting to server...",
        SshConnectionState::Authenticating => "Authenticating...",
        SshConnectionState::Connected => "Connected successfully!",
        SshConnectionState::Failed(msg) => &format!("Connection failed: {}", msg),
    };
    dialog.set_secondary_text(Some(secondary_text));
}

/// 显示 SSH 命令执行对话框
/// 允许用户在已连接的 SSH 服务器上执行自定义命令
pub fn show_ssh_command_dialog<F>(
    parent: &ApplicationWindow,
    saved_servers: Vec<SshConfig>,
    on_execute: F,
) where
    F: Fn(SshConfig, String) + 'static,
{
    let dialog = Dialog::builder()
        .title(&t(I18nKey::DialogSshCommand))
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
        .label(&t(I18nKey::LabelServer))
        .width_chars(12)
        .xalign(1.0)
        .build();

    let server_names: Vec<String> = saved_servers.iter()
        .map(|s| format!("{} ({}@{})", s.name, s.username, s.host))
        .collect();

    let server_combo = if !server_names.is_empty() {
        gtk4::DropDown::from_strings(&server_names.iter().map(|s| s.as_str()).collect::<Vec<_>>())
    } else {
        gtk4::DropDown::from_strings(&[&t(I18nKey::MsgNoSavedServers)])
    };
    server_combo.set_hexpand(true);

    server_box.append(&server_label);
    server_box.append(&server_combo);
    content.append(&server_box);

    // 命令输入
    let command_box = Box::new(Orientation::Vertical, 6);
    let command_label = Label::builder()
        .label(&t(I18nKey::LabelCommand))
        .xalign(0.0)
        .build();

    let command_entry = Entry::builder()
        .placeholder_text("e.g., tail -f /var/log/app.log")
        .hexpand(true)
        .build();

    // 常用命令建议
    let suggestions_label = Label::builder()
        .label(&t(I18nKey::MsgSuggestions))
        .wrap(true)
        .xalign(0.0)
        .css_classes(vec!["dim-label".to_string()])
        .build();

    command_box.append(&command_label);
    command_box.append(&command_entry);
    command_box.append(&suggestions_label);
    content.append(&command_box);

    // 按钮
    dialog.add_button(&t(I18nKey::ButtonCancel), ResponseType::Cancel);
    if !saved_servers.is_empty() {
        dialog.add_button(&t(I18nKey::ButtonExecute), ResponseType::Accept);
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
    _current_theme: &str,
    on_save: F,
) where
    F: Fn(String) + 'static,
{
    let dialog = Dialog::builder()
        .title(&t(I18nKey::DialogThemeSettings))
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
        .label(&format!("{}: {}", &t(I18nKey::LabelTheme), &t(I18nKey::ThemeLight)))
        .wrap(true)
        .xalign(0.0)
        .build();
    content.append(&info_label);

    // 按钮
    dialog.add_button(&t(I18nKey::ButtonOk), ResponseType::Ok);

    dialog.connect_response(move |dialog, response| {
        if response == ResponseType::Ok {
            on_save("light".to_string());
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
    F: Fn(String, bool) + 'static,  // 第二个参数表示是否需要重启
{
    let dialog = Dialog::builder()
        .title(&t(I18nKey::DialogLanguageSettings))
        .transient_for(parent)
        .modal(true)
        .default_width(350)
        .build();

    let content = dialog.content_area();
    content.set_spacing(12);
    content.set_margin_top(12);
    content.set_margin_bottom(12);
    content.set_margin_start(12);
    content.set_margin_end(12);

    // 说明标签
    let info_label = Label::builder()
        .label(&format!("{}:", &t(I18nKey::LabelLanguage)))
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

    // 重启提示标签
    let restart_label = Label::builder()
        .label(&format!("⚠️ {}", &t(I18nKey::MsgRestartRequired)))
        .wrap(true)
        .xalign(0.0)
        .css_classes(vec!["warning".to_string()])
        .build();
    content.append(&restart_label);

    // 按钮
    dialog.add_button(&t(I18nKey::ButtonCancel), ResponseType::Cancel);
    dialog.add_button(&t(I18nKey::ButtonResetDefault), ResponseType::Other(1)); // 恢复默认
    dialog.add_button(&t(I18nKey::ButtonSave), ResponseType::Accept);

    let en_radio_clone = en_radio.clone();
    let zh_radio_clone = zh_radio.clone();
    let current_lang_clone = current_language.to_string();

    dialog.connect_response(move |dialog, response| {
        match response {
            ResponseType::Accept => {
                let lang = if zh_radio_clone.is_active() {
                    "zh".to_string()
                } else {
                    "en".to_string()
                };
                // 检查语言是否真的改变了
                let lang_changed = lang != current_lang_clone;
                on_save(lang, lang_changed);
            }
            ResponseType::Other(1) => {
                // 恢复默认：en
                let lang_changed = "en" != current_lang_clone;
                on_save("en".to_string(), lang_changed);
            }
            _ => {}
        }
        dialog.close();
    });

    dialog.present();
}
