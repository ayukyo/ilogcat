use gtk4::prelude::*;
use gtk4::{Box, Button, Entry, Label, ListView, ScrolledWindow, Orientation, SignalListItemFactory, SingleSelection, Popover, GestureClick, FileChooserDialog, FileChooserAction, ResponseType};
use gtk4::gio;
use gtk4::glib;
use gtk4::gdk::Rectangle;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::mpsc::{channel, Sender};

use crate::ssh::{SftpManager, SftpEntry};
use crate::ssh::config::SshConfig;

/// 文件面板组件
pub struct FilePanel {
    container: Box,
    sftp: Rc<RefCell<Option<SftpManager>>>,
    ssh_config: Rc<RefCell<Option<SshConfig>>>,
    list_view: ListView,
    path_entry: Entry,
    model: gio::ListStore,
    entries: Rc<RefCell<Vec<SftpEntry>>>,
    is_operating: Arc<AtomicBool>,
    progress_label: Label,
}

impl FilePanel {
    pub fn new() -> Self {
        let container = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(4)
            .margin_start(4)
            .margin_end(4)
            .margin_top(4)
            .margin_bottom(4)
            .width_request(250)
            .build();

        // 工具栏
        let toolbar = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(4)
            .build();

        let home_btn = Button::builder()
            .icon_name("user-home-symbolic")
            .tooltip_text("主目录")
            .build();

        let refresh_btn = Button::builder()
            .icon_name("view-refresh-symbolic")
            .tooltip_text("刷新")
            .build();

        let up_btn = Button::builder()
            .icon_name("go-up-symbolic")
            .tooltip_text("上级目录")
            .build();

        // 进度提示标签
        let progress_label = Label::builder()
            .halign(gtk4::Align::End)
            .hexpand(true)
            .margin_end(4)
            .build();

        toolbar.append(&home_btn);
        toolbar.append(&refresh_btn);
        toolbar.append(&up_btn);
        toolbar.append(&progress_label);

        // 路径输入框
        let path_entry = Entry::builder()
            .hexpand(true)
            .placeholder_text("路径")
            .build();

        // 文件列表 - 使用 StringList
        let model = gio::ListStore::new::<glib::BoxedAnyObject>();
        let selection = SingleSelection::builder()
            .model(&model)
            .build();

        let factory = SignalListItemFactory::new();

        factory.connect_setup(|_, list_item| {
            let label = Label::builder()
                .halign(gtk4::Align::Start)
                .hexpand(true)
                .margin_start(4)
                .margin_end(4)
                .build();
            list_item.set_child(Some(&label));
        });

        factory.connect_bind(|_, list_item| {
            if let Some(obj) = list_item.item().and_downcast::<glib::BoxedAnyObject>() {
                if let Some(label) = list_item.child().and_downcast::<Label>() {
                    let entry = obj.borrow::<SftpEntry>();
                    let text = if entry.is_dir {
                        format!("📁 {}", entry.name)
                    } else {
                        let size_str = Self::format_size(entry.size);
                        format!("📄 {} ({})", entry.name, size_str)
                    };
                    label.set_text(&text);
                }
            }
        });

        let list_view = ListView::builder()
            .model(&selection)
            .factory(&factory)
            .hexpand(true)
            .vexpand(true)
            .build();

        let scrolled = ScrolledWindow::builder()
            .hexpand(true)
            .vexpand(true)
            .build();
        scrolled.set_child(Some(&list_view));

        container.append(&toolbar);
        container.append(&path_entry);
        container.append(&scrolled);

        let sftp: Rc<RefCell<Option<SftpManager>>> = Rc::new(RefCell::new(None));
        let ssh_config: Rc<RefCell<Option<SshConfig>>> = Rc::new(RefCell::new(None));
        let entries = Rc::new(RefCell::new(Vec::new()));
        let is_operating = Arc::new(AtomicBool::new(false));

        let model_clone = model.clone();
        let sftp_clone = sftp.clone();
        let entries_clone = entries.clone();
        refresh_btn.connect_clicked(move |_| {
            if let Some(ref sftp) = *sftp_clone.borrow() {
                match sftp.list_dir() {
                    Ok(new_entries) => {
                        *entries_clone.borrow_mut() = new_entries.clone();
                        model_clone.remove_all();
                        for entry in new_entries {
                            model_clone.append(&glib::BoxedAnyObject::new(entry));
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to list directory: {}", e);
                    }
                }
            }
        });

        let sftp_clone = sftp.clone();
        let model_clone = model.clone();
        let path_entry_clone = path_entry.clone();
        let entries_clone = entries.clone();
        up_btn.connect_clicked(move |_| {
            if let Some(ref mut sftp) = *sftp_clone.borrow_mut() {
                let _ = sftp.cd_up();
                if let Ok(new_entries) = sftp.list_dir() {
                    *entries_clone.borrow_mut() = new_entries.clone();
                    model_clone.remove_all();
                    for entry in new_entries {
                        model_clone.append(&glib::BoxedAnyObject::new(entry));
                    }
                    path_entry_clone.set_text(&sftp.current_path().to_string_lossy());
                }
            }
        });

        let sftp_clone = sftp.clone();
        let model_clone = model.clone();
        let path_entry_clone = path_entry.clone();
        let entries_clone = entries.clone();
        home_btn.connect_clicked(move |_| {
            if let Some(ref mut sftp) = *sftp_clone.borrow_mut() {
                let _ = sftp.cd_home();
                if let Ok(new_entries) = sftp.list_dir() {
                    *entries_clone.borrow_mut() = new_entries.clone();
                    model_clone.remove_all();
                    for entry in new_entries {
                        model_clone.append(&glib::BoxedAnyObject::new(entry));
                    }
                    path_entry_clone.set_text(&sftp.current_path().to_string_lossy());
                }
            }
        });

        let sftp_clone = sftp.clone();
        let model_clone = model.clone();
        let path_entry_clone = path_entry.clone();
        let entries_clone = entries.clone();
        path_entry.connect_activate(move |entry| {
            let path = entry.text();
            if let Some(ref mut sftp) = *sftp_clone.borrow_mut() {
                if sftp.cd_path(std::path::Path::new(&path)).is_ok() {
                    if let Ok(new_entries) = sftp.list_dir() {
                        *entries_clone.borrow_mut() = new_entries.clone();
                        model_clone.remove_all();
                        for e in new_entries {
                            model_clone.append(&glib::BoxedAnyObject::new(e));
                        }
                        path_entry_clone.set_text(&sftp.current_path().to_string_lossy());
                    }
                }
            }
        });

        // 双击进入目录或下载文件
        let sftp_clone = sftp.clone();
        let model_clone = model.clone();
        let path_entry_clone = path_entry.clone();
        let entries_clone = entries.clone();
        list_view.connect_activate(move |_, position| {
            if let Some(obj) = model_clone.item(position).and_downcast::<glib::BoxedAnyObject>() {
                let entry = obj.borrow::<SftpEntry>();
                let is_dir = entry.is_dir;
                let path = entry.path.clone();
                let name = entry.name.clone();

                if is_dir {
                    // 进入目录
                    if let Some(ref mut sftp) = *sftp_clone.borrow_mut() {
                        if sftp.cd_path(&path).is_ok() {
                            if let Ok(new_entries) = sftp.list_dir() {
                                *entries_clone.borrow_mut() = new_entries.clone();
                                model_clone.remove_all();
                                for e in new_entries {
                                    model_clone.append(&glib::BoxedAnyObject::new(e));
                                }
                                path_entry_clone.set_text(&sftp.current_path().to_string_lossy());
                            }
                        }
                    }
                } else {
                    // 下载文件
                    if let Some(ref sftp) = *sftp_clone.borrow() {
                        let local_dir = SftpManager::default_download_dir();
                        let local_path = local_dir.join(&name);
                        match sftp.download(&path, &local_path) {
                            Ok(()) => {
                                eprintln!("Downloaded to: {:?}", local_path);
                            }
                            Err(e) => {
                                eprintln!("Download failed: {}", e);
                            }
                        }
                    }
                }
            }
        });

        // 右键菜单
        let popover = Popover::builder()
            .has_arrow(false)
            .position(gtk4::PositionType::Right)
            .build();
        popover.set_parent(&list_view);

        // 创建菜单按钮
        let menu_box = Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(2)
            .build();

        let download_btn = Button::builder()
            .label("下载")
            .hexpand(true)
            .build();
        menu_box.append(&download_btn);

        let copy_path_btn = Button::builder()
            .label("复制路径")
            .hexpand(true)
            .build();
        menu_box.append(&copy_path_btn);

        let delete_btn = Button::builder()
            .label("删除")
            .hexpand(true)
            .build();
        menu_box.append(&delete_btn);

        popover.set_child(Some(&menu_box));

        // 当前选中的条目
        let selected_entry: Rc<RefCell<Option<SftpEntry>>> = Rc::new(RefCell::new(None));

        // 下载按钮
        let ssh_config_clone = ssh_config.clone();
        let selected_clone = selected_entry.clone();
        let popover_clone = popover.clone();
        let progress_clone = progress_label.clone();
        download_btn.connect_clicked(move |_| {
            popover_clone.hide();
            if let Some(entry) = selected_clone.borrow().clone() {
                let is_dir = entry.is_dir;
                let entry_name = entry.name.clone();
                let remote_path = entry.path.clone();

                // 获取 SSH 配置用于后台下载
                let config = ssh_config_clone.borrow().clone();

                if is_dir {
                    // 文件夹：选择保存位置
                    let dialog = FileChooserDialog::new(
                        Some("选择保存位置"),
                        None::<&gtk4::Window>,
                        FileChooserAction::SelectFolder,
                        &[
                            ("取消", ResponseType::Cancel),
                            ("选择", ResponseType::Accept),
                        ],
                    );
                    dialog.set_modal(true);

                    let progress = progress_clone.clone();
                    dialog.connect_response(move |dialog, response| {
                        if response == ResponseType::Accept {
                            if let Some(file) = dialog.file() {
                                if let Some(parent_path) = file.path() {
                                    let local_path = parent_path.join(&entry_name);
                                    let remote = remote_path.clone();
                                    let cfg = config.clone();

                                    // 显示下载中
                                    progress.set_text("正在下载文件夹...");

                                    // 创建通道
                                    let (tx, rx) = channel::<String>();
                                    let progress = progress.clone();

                                    // 主线程监听结果
                                    glib::idle_add_local(move || {
                                        match rx.try_recv() {
                                            Ok(msg) => {
                                                progress.set_text(&msg);
                                                if msg.starts_with("下载完成") {
                                                    let p = progress.clone();
                                                    glib::timeout_add_local_once(std::time::Duration::from_secs(3), move || {
                                                        p.set_text("");
                                                    });
                                                }
                                            }
                                            Err(std::sync::mpsc::TryRecvError::Empty) => {}
                                            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                                                return glib::ControlFlow::Break;
                                            }
                                        }
                                        glib::ControlFlow::Continue
                                    });

                                    // 在后台线程创建新连接并下载
                                    std::thread::spawn(move || {
                                        if let Some(ref ssh_cfg) = cfg {
                                            match SftpManager::connect(ssh_cfg) {
                                                Ok(sftp) => {
                                                    let result = sftp.download_dir(&remote, &local_path);
                                                    match result {
                                                        Ok(()) => {
                                                            let _ = tx.send("下载完成".to_string());
                                                        }
                                                        Err(e) => {
                                                            let _ = tx.send(format!("下载失败: {}", e));
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    let _ = tx.send(format!("连接失败: {}", e));
                                                }
                                            }
                                        }
                                    });
                                }
                            }
                        }
                        dialog.close();
                    });
                    dialog.show();
                } else {
                    // 文件：选择保存文件
                    let dialog = FileChooserDialog::new(
                        Some("保存文件"),
                        None::<&gtk4::Window>,
                        FileChooserAction::Save,
                        &[
                            ("取消", ResponseType::Cancel),
                            ("保存", ResponseType::Accept),
                        ],
                    );
                    dialog.set_modal(true);
                    dialog.set_current_name(&entry_name);

                    let progress = progress_clone.clone();
                    dialog.connect_response(move |dialog, response| {
                        if response == ResponseType::Accept {
                            if let Some(file) = dialog.file() {
                                if let Some(local_path) = file.path() {
                                    let remote = remote_path.clone();
                                    let cfg = config.clone();

                                    // 显示下载中
                                    progress.set_text("正在下载...");

                                    // 创建通道
                                    let (tx, rx) = channel::<String>();
                                    let progress = progress.clone();

                                    // 主线程监听结果
                                    glib::idle_add_local(move || {
                                        match rx.try_recv() {
                                            Ok(msg) => {
                                                progress.set_text(&msg);
                                                if msg.starts_with("下载完成") {
                                                    let p = progress.clone();
                                                    glib::timeout_add_local_once(std::time::Duration::from_secs(3), move || {
                                                        p.set_text("");
                                                    });
                                                }
                                            }
                                            Err(std::sync::mpsc::TryRecvError::Empty) => {}
                                            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                                                return glib::ControlFlow::Break;
                                            }
                                        }
                                        glib::ControlFlow::Continue
                                    });

                                    // 在后台线程创建新连接并下载
                                    std::thread::spawn(move || {
                                        if let Some(ref ssh_cfg) = cfg {
                                            match SftpManager::connect(ssh_cfg) {
                                                Ok(sftp) => {
                                                    let result = sftp.download(&remote, &local_path);
                                                    match result {
                                                        Ok(()) => {
                                                            let _ = tx.send("下载完成".to_string());
                                                        }
                                                        Err(e) => {
                                                            let _ = tx.send(format!("下载失败: {}", e));
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    let _ = tx.send(format!("连接失败: {}", e));
                                                }
                                            }
                                        }
                                    });
                                }
                            }
                        }
                        dialog.close();
                    });
                    dialog.show();
                }
            }
        });

        // 复制路径按钮
        let selected_clone = selected_entry.clone();
        let popover_clone = popover.clone();
        copy_path_btn.connect_clicked(move |_| {
            if let Some(entry) = selected_clone.borrow().clone() {
                let path = entry.path.to_string_lossy().to_string();
                if let Some(display) = gtk4::gdk::Display::default() {
                    let clipboard = display.clipboard();
                    clipboard.set_text(&path);
                    eprintln!("已复制路径: {}", path);
                }
            }
            popover_clone.hide();
        });

        // 删除按钮
        let sftp_clone = sftp.clone();
        let model_clone = model.clone();
        let selected_clone = selected_entry.clone();
        let popover_clone = popover.clone();
        let entries_clone = entries.clone();
        delete_btn.connect_clicked(move |_| {
            if let Some(ref sftp) = *sftp_clone.borrow() {
                if let Some(entry) = selected_clone.borrow().clone() {
                    let result = if entry.is_dir {
                        sftp.delete_dir(&entry.path)
                    } else {
                        sftp.delete_file(&entry.path)
                    };
                    match result {
                        Ok(()) => {
                            eprintln!("已删除: {:?}", entry.path);
                            // 刷新列表
                            if let Ok(new_entries) = sftp.list_dir() {
                                *entries_clone.borrow_mut() = new_entries.clone();
                                model_clone.remove_all();
                                for e in new_entries {
                                    model_clone.append(&glib::BoxedAnyObject::new(e));
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("删除失败: {}", e);
                        }
                    }
                }
            }
            popover_clone.hide();
        });

        // 添加右键点击事件
        let popover_clone = popover.clone();
        let selected_clone = selected_entry.clone();
        let list_view_clone = list_view.clone();
        let gesture = GestureClick::new();
        gesture.set_button(3);  // 右键
        gesture.connect_pressed(move |_gesture, n_press, x, y| {
            if n_press == 1 {
                // 使用 pick 获取点击位置的 widget
                let picked = list_view_clone.pick(x, y, gtk4::PickFlags::DEFAULT);

                let mut found_entry: Option<(SftpEntry, u32)> = None;

                // 遍历 widget 层级找到对应的条目
                let mut current = picked.clone();
                while let Some(ref widget) = current {
                    // 检查是否是 Label（文件名显示的 widget）
                    if let Some(label) = widget.clone().downcast::<Label>().ok() {
                        // 从 Label 的文本匹配条目
                        let text = label.text().to_string();
                        // 查找匹配的条目
                        if let Some(sel_model) = list_view_clone.model().and_downcast::<SingleSelection>() {
                            if let Some(model) = sel_model.model() {
                                for i in 0..model.n_items() {
                                    if let Some(obj) = model.item(i).and_downcast::<glib::BoxedAnyObject>() {
                                        let entry = obj.borrow::<SftpEntry>();
                                        if text.contains(&entry.name) {
                                            found_entry = Some((entry.clone(), i));
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        break;
                    }
                    current = widget.parent();
                }

                // 如果通过 widget 没找到，回退到索引计算
                if found_entry.is_none() {
                    if let Some(sel_model) = list_view_clone.model().and_downcast::<SingleSelection>() {
                        if let Some(m) = sel_model.model() {
                            let row_height = 26f64;
                            let scroll_y = list_view_clone.vadjustment().map(|a| a.value()).unwrap_or(0.0);
                            let clicked_idx = ((y + scroll_y) / row_height).floor() as u32;
                            if clicked_idx < m.n_items() {
                                if let Some(obj) = m.item(clicked_idx).and_downcast::<glib::BoxedAnyObject>() {
                                    let entry = obj.borrow::<SftpEntry>().clone();
                                    found_entry = Some((entry, clicked_idx));
                                }
                            }
                        }
                    }
                }

                if let Some((entry, idx)) = found_entry {
                    *selected_clone.borrow_mut() = Some(entry);
                    // 更新选中项，让用户看到右键点击的是哪个文件
                    if let Some(sel_model) = list_view_clone.model().and_downcast::<SingleSelection>() {
                        sel_model.set_selected(idx);
                    }
                }

                // 设置 popover 位置并显示
                let rect = Rectangle::new(x as i32 + 5, y as i32 + 5, 1, 1);
                popover_clone.set_pointing_to(Some(&rect));
                popover_clone.show();
            }
        });
        list_view.add_controller(gesture);

        let panel = Self {
            container,
            sftp,
            ssh_config,
            list_view,
            path_entry,
            model,
            entries,
            is_operating,
            progress_label,
        };

        panel
    }

    /// 格式化文件大小
    fn format_size(size: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if size >= GB {
            format!("{:.1}G", size as f64 / GB as f64)
        } else if size >= MB {
            format!("{:.1}M", size as f64 / MB as f64)
        } else if size >= KB {
            format!("{:.1}K", size as f64 / KB as f64)
        } else {
            format!("{}B", size)
        }
    }

    /// 设置 SFTP 管理器
    pub fn set_sftp(&self, sftp: Option<SftpManager>, config: Option<SshConfig>) {
        *self.sftp.borrow_mut() = sftp;
        *self.ssh_config.borrow_mut() = config;
        self.refresh();
    }

    /// 获取容器 widget
    pub fn widget(&self) -> &Box {
        &self.container
    }

    /// 刷新当前目录
    pub fn refresh(&self) {
        let sftp = self.sftp.borrow();
        if let Some(ref sftp) = *sftp {
            match sftp.list_dir() {
                Ok(entries) => {
                    *self.entries.borrow_mut() = entries.clone();
                    self.model.remove_all();
                    for entry in entries {
                        self.model.append(&glib::BoxedAnyObject::new(entry));
                    }
                    self.path_entry.set_text(&sftp.current_path().to_string_lossy());
                }
                Err(e) => {
                    eprintln!("Failed to list directory: {}", e);
                }
            }
        }
    }

    /// 复制选中路径到剪贴板
    pub fn copy_selected_path(&self) {
        let selection = self.list_view.model()
            .and_then(|m| m.downcast::<SingleSelection>().ok());
        if let Some(selection) = selection {
            let pos = selection.selected();
            if let Some(obj) = selection.model().and_then(|m| m.item(pos)).and_downcast::<glib::BoxedAnyObject>() {
                let entry = obj.borrow::<SftpEntry>();
                let path = entry.path.to_string_lossy().to_string();
                if let Some(display) = gtk4::gdk::Display::default() {
                    let clipboard = display.clipboard();
                    clipboard.set_text(&path);
                }
            }
        }
    }

    /// 是否有 SFTP 连接
    pub fn is_connected(&self) -> bool {
        self.sftp.borrow().is_some()
    }
}

impl Clone for FilePanel {
    fn clone(&self) -> Self {
        Self {
            container: self.container.clone(),
            sftp: self.sftp.clone(),
            ssh_config: self.ssh_config.clone(),
            list_view: self.list_view.clone(),
            path_entry: self.path_entry.clone(),
            model: self.model.clone(),
            entries: self.entries.clone(),
            is_operating: self.is_operating.clone(),
            progress_label: self.progress_label.clone(),
        }
    }
}

impl Default for FilePanel {
    fn default() -> Self {
        Self::new()
    }
}