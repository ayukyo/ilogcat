use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, Entry, Label, ListView, ScrolledWindow, Orientation, SignalListItemFactory, SingleSelection, Popover, GestureClick, FileChooserDialog, FileChooserAction, ResponseType, DropTarget};
use gtk4::gio;
use gtk4::glib;
use gtk4::gdk::Rectangle;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::mpsc::channel;
use std::boxed::Box as StdBox;
use std::collections::HashSet;

use crate::ssh::{SftpManager, SftpEntry};
use crate::ssh::config::SshConfig;

/// 检查是否为压缩包格式
fn is_compressed_archive(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.ends_with(".zip")
        || lower.ends_with(".tar")
        || lower.ends_with(".gz")
        || lower.ends_with(".tgz")
        || lower.ends_with(".tar.gz")
        || lower.ends_with(".tar.bz2")
        || lower.ends_with(".tar.xz")
        || lower.ends_with(".7z")
        || lower.ends_with(".rar")
        || lower.ends_with(".bz2")
        || lower.ends_with(".xz")
        || lower.ends_with(".lz")
        || lower.ends_with(".lzma")
        || lower.ends_with(".cab")
        || lower.ends_with(".iso")
        || lower.ends_with(".zst")
        || lower.ends_with(".tzst")
}

/// 文件面板组件
pub struct FilePanel {
    container: GtkBox,
    sftp: Rc<RefCell<Option<SftpManager>>>,
    ssh_config: Rc<RefCell<Option<SshConfig>>>,
    list_view: ListView,
    path_entry: Entry,
    model: gio::ListStore,
    entries: Rc<RefCell<Vec<SftpEntry>>>,
    is_operating: Arc<AtomicBool>,
    progress_label: Label,
    transferring_files: Rc<RefCell<HashSet<String>>>,  // 正在传输的文件路径
}

impl FilePanel {
    pub fn new() -> Self {
        let container = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(4)
            .margin_start(4)
            .margin_end(4)
            .margin_top(4)
            .margin_bottom(4)
            .width_request(250)
            .build();

        // 工具栏
        let toolbar = GtkBox::builder()
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

        let sftp: Rc<RefCell<Option<SftpManager>>> = Rc::new(RefCell::new(None));
        let ssh_config: Rc<RefCell<Option<SshConfig>>> = Rc::new(RefCell::new(None));
        let entries = Rc::new(RefCell::new(Vec::new()));
        let is_operating = Arc::new(AtomicBool::new(false));
        let transferring_files: Rc<RefCell<HashSet<String>>> = Rc::new(RefCell::new(HashSet::new()));

        // 右键菜单 - 先创建
        let popover = Popover::builder()
            .has_arrow(false)
            .position(gtk4::PositionType::Right)
            .build();

        // 当前选中的条目
        let selected_entry: Rc<RefCell<Option<SftpEntry>>> = Rc::new(RefCell::new(None));

        // 创建菜单按钮
        let menu_box = GtkBox::builder()
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

        // 创建 factory - 在 popover 和 selected_entry 之后
        let factory = SignalListItemFactory::new();

        let popover_for_factory = popover.clone();
        let selected_for_factory = selected_entry.clone();
        let selection_for_factory = selection.clone();
        let transferring_for_factory = transferring_files.clone();

        factory.connect_setup(move |_, list_item| {
            let label = Label::builder()
                .halign(gtk4::Align::Start)
                .hexpand(true)
                .margin_start(4)
                .margin_end(4)
                .build();
            list_item.set_child(Some(&label));
        });

        factory.connect_bind(move |factory, list_item| {
            if let Some(obj) = list_item.item().and_downcast::<glib::BoxedAnyObject>() {
                if let Some(label) = list_item.child().and_downcast::<Label>() {
                    let entry = obj.borrow::<SftpEntry>();
                    let size_str = Self::format_size(entry.size);
                    let is_transferring = transferring_for_factory.borrow().contains(&entry.path.to_string_lossy().to_string());

                    let text = if is_transferring {
                        // 正在传输中
                        format!("⏳ {} ({})", entry.name, size_str)
                    } else if entry.is_dir {
                        format!("📁 {}", entry.name)
                    } else if is_compressed_archive(&entry.name) {
                        format!("🗜️ {} ({})", entry.name, size_str)
                    } else {
                        format!("📄 {} ({})", entry.name, size_str)
                    };
                    label.set_text(&text);

                    // 添加右键手势
                    let gesture = GestureClick::new();
                    gesture.set_button(3);  // 右键

                    let selected_clone = selected_for_factory.clone();
                    let popover_clone = popover_for_factory.clone();
                    let sel_clone = selection_for_factory.clone();
                    let entry_clone = entry.clone();
                    let pos = list_item.position();

                    gesture.connect_pressed(move |_gesture, n_press, x, y| {
                        if n_press == 1 {
                            // 更新选中 - 设置点击的位置
                            *selected_clone.borrow_mut() = Some(entry_clone.clone());
                            sel_clone.set_selected(pos);

                            // 显示菜单
                            let rect = Rectangle::new(x as i32 + 5, y as i32 + 5, 1, 1);
                            popover_clone.set_pointing_to(Some(&rect));
                            popover_clone.show();
                        }
                    });

                    label.add_controller(gesture);
                }
            }
        });

        let list_view = ListView::builder()
            .model(&selection)
            .factory(&factory)
            .hexpand(true)
            .vexpand(true)
            .build();

        popover.set_parent(&list_view);

        let scrolled = ScrolledWindow::builder()
            .hexpand(true)
            .vexpand(true)
            .build();
        scrolled.set_child(Some(&list_view));

        container.append(&toolbar);
        container.append(&path_entry);
        container.append(&scrolled);

        // 刷新按钮
        let model_clone = model.clone();
        let sftp_clone = sftp.clone();
        let entries_clone = entries.clone();
        let progress_clone = progress_label.clone();
        refresh_btn.connect_clicked(move |_| {
            // 清除错误状态
            progress_clone.set_text("");
            progress_clone.remove_css_class("error");

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
                        progress_clone.set_text(&format!("刷新失败: {}", e));
                        progress_clone.add_css_class("error");
                    }
                }
            }
        });

        // 上级目录按钮
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

        // 主目录按钮
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

        // 路径输入框激活
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

        // 下载按钮
        let ssh_config_clone = ssh_config.clone();
        let selected_clone = selected_entry.clone();
        let popover_clone = popover.clone();
        let progress_clone = progress_label.clone();
        let transferring_clone = transferring_files.clone();
        let model_clone = model.clone();
        download_btn.connect_clicked(move |_| {
            popover_clone.hide();
            if let Some(entry) = selected_clone.borrow().clone() {
                let is_dir = entry.is_dir;
                let entry_name = entry.name.clone();
                let remote_path = entry.path.clone();
                let remote_path_for_transfer = remote_path.to_string_lossy().to_string();

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
                    let transferring = transferring_clone.clone();
                    let model = model_clone.clone();
                    dialog.connect_response(move |dialog, response| {
                        if response == ResponseType::Accept {
                            if let Some(file) = dialog.file() {
                                if let Some(parent_path) = file.path() {
                                    let local_path = parent_path.join(&entry_name);
                                    let remote = remote_path.clone();
                                    let cfg = config.clone();
                                    let remote_str = remote_path_for_transfer.clone();

                                    // 添加到传输列表并刷新显示
                                    transferring.borrow_mut().insert(remote_str.clone());
                                    model.items_changed(0, 0, 0);  // 触发刷新

                                    // 显示下载中
                                    progress.set_text("正在下载文件夹 0%");
                                    progress.remove_css_class("error");

                                    // 创建通道 - 用于进度更新
                                    let (tx, rx) = channel::<(String, bool)>();
                                    let progress = progress.clone();
                                    let transferring = transferring.clone();
                                    let model = model.clone();

                                    // 主线程监听结果
                                    glib::idle_add_local(move || {
                                        match rx.try_recv() {
                                            Ok((msg, is_error)) => {
                                                progress.set_text(&msg);
                                                if is_error {
                                                    progress.add_css_class("error");
                                                } else {
                                                    progress.remove_css_class("error");
                                                    if msg.contains("完成") {
                                                        // 从传输列表移除
                                                        transferring.borrow_mut().remove(&remote_str);
                                                        model.items_changed(0, 0, 0);  // 刷新显示
                                                        let p = progress.clone();
                                                        glib::timeout_add_local_once(std::time::Duration::from_secs(3), move || {
                                                            p.set_text("");
                                                            p.remove_css_class("error");
                                                        });
                                                    }
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
                                                    let tx_progress = tx.clone();
                                                    let progress_cb = |transferred: u64, total: u64| {
                                                        let percent = if total > 0 {
                                                            (transferred as f64 / total as f64 * 100.0) as u32
                                                        } else {
                                                            0
                                                        };
                                                        let _ = tx_progress.send((format!("正在下载文件夹 {}%", percent), false));
                                                    };

                                                    let result = sftp.download_dir_with_progress(
                                                        &remote,
                                                        &local_path,
                                                        Some(&progress_cb)
                                                    );

                                                    match result {
                                                        Ok(_) => {
                                                            let _ = tx.send(("下载完成".to_string(), false));
                                                        }
                                                        Err(e) => {
                                                            let _ = tx.send((format!("下载失败: {}", e), true));
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    let _ = tx.send((format!("连接失败: {}", e), true));
                                                }
                                            }
                                        } else {
                                            let _ = tx.send(("未连接 SSH".to_string(), true));
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
                    let transferring = transferring_clone.clone();
                    let model = model_clone.clone();
                    let remote_str = remote_path_for_transfer.clone();
                    dialog.connect_response(move |dialog, response| {
                        if response == ResponseType::Accept {
                            if let Some(file) = dialog.file() {
                                if let Some(local_path) = file.path() {
                                    let remote = remote_path.clone();
                                    let cfg = config.clone();

                                    // 添加到传输列表并刷新显示
                                    transferring.borrow_mut().insert(remote_str.clone());
                                    model.items_changed(0, 0, 0);

                                    // 显示下载中
                                    progress.set_text("正在下载 0%");
                                    progress.remove_css_class("error");

                                    // 创建通道
                                    let (tx, rx) = channel::<(String, bool)>();
                                    let progress = progress.clone();
                                    let transferring = transferring.clone();
                                    let model = model.clone();

                                    // 主线程监听结果
                                    let remote_str_clone = remote_str.clone();
                                    glib::idle_add_local(move || {
                                        match rx.try_recv() {
                                            Ok((msg, is_error)) => {
                                                progress.set_text(&msg);
                                                if is_error {
                                                    progress.add_css_class("error");
                                                } else {
                                                    progress.remove_css_class("error");
                                                    if msg.contains("完成") {
                                                        // 从传输列表移除
                                                        transferring.borrow_mut().remove(&remote_str_clone);
                                                        model.items_changed(0, 0, 0);
                                                        let p = progress.clone();
                                                        glib::timeout_add_local_once(std::time::Duration::from_secs(3), move || {
                                                            p.set_text("");
                                                            p.remove_css_class("error");
                                                        });
                                                    }
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
                                                    let tx_progress = tx.clone();
                                                    let progress_cb = move |transferred: u64, total: u64| {
                                                        let percent = if total > 0 {
                                                            (transferred as f64 / total as f64 * 100.0) as u32
                                                        } else {
                                                            0
                                                        };
                                                        let _ = tx_progress.send((format!("正在下载 {}%", percent), false));
                                                    };
                                                    match sftp.download_with_progress(&remote, &local_path, Some(StdBox::new(progress_cb))) {
                                                        Ok(()) => {
                                                            let _ = tx.send(("下载完成".to_string(), false));
                                                        }
                                                        Err(e) => {
                                                            let _ = tx.send((format!("下载失败: {}", e), true));
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    let _ = tx.send((format!("连接失败: {}", e), true));
                                                }
                                            }
                                        } else {
                                            let _ = tx.send(("未连接 SSH".to_string(), true));
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
        let ssh_config_clone = ssh_config.clone();
        let model_clone = model.clone();
        let selected_clone = selected_entry.clone();
        let popover_clone = popover.clone();
        let entries_clone = entries.clone();
        let progress_clone = progress_label.clone();
        let sftp_current = sftp.clone();
        delete_btn.connect_clicked(move |_| {
            popover_clone.hide();
            if let Some(entry) = selected_clone.borrow().clone() {
                let is_dir = entry.is_dir;
                let entry_path = entry.path.clone();
                let config = ssh_config_clone.borrow().clone();

                // 显示删除中
                progress_clone.set_text("正在删除 0%");
                progress_clone.remove_css_class("error");

                // 创建通道
                let (tx, rx) = channel::<(String, bool)>();
                let progress = progress_clone.clone();
                let model = model_clone.clone();
                let entries = entries_clone.clone();
                let sftp_update = sftp_current.clone();

                // 主线程监听结果
                glib::idle_add_local(move || {
                    match rx.try_recv() {
                        Ok((msg, is_error)) => {
                            progress.set_text(&msg);
                            if is_error {
                                progress.add_css_class("error");
                            } else {
                                progress.remove_css_class("error");
                                if msg.contains("完成") {
                                    // 刷新文件列表
                                    if let Some(ref sftp) = *sftp_update.borrow() {
                                        if let Ok(new_entries) = sftp.list_dir() {
                                            *entries.borrow_mut() = new_entries.clone();
                                            model.remove_all();
                                            for e in new_entries {
                                                model.append(&glib::BoxedAnyObject::new(e));
                                            }
                                        }
                                    }
                                    let p = progress.clone();
                                    glib::timeout_add_local_once(std::time::Duration::from_secs(2), move || {
                                        p.set_text("");
                                        p.remove_css_class("error");
                                    });
                                }
                            }
                        }
                        Err(std::sync::mpsc::TryRecvError::Empty) => {}
                        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                            return glib::ControlFlow::Break;
                        }
                    }
                    glib::ControlFlow::Continue
                });

                // 在后台线程执行删除
                std::thread::spawn(move || {
                    if let Some(ref cfg) = config {
                        match SftpManager::connect(cfg) {
                            Ok(sftp) => {
                                let tx_progress = tx.clone();

                                let result = if is_dir {
                                    let tx_progress = tx.clone();
                                    sftp.delete_dir_with_progress(&entry_path, Some(&|deleted, total| {
                                        let percent = if total > 0 {
                                            (deleted as f64 / total as f64 * 100.0) as u32
                                        } else {
                                            0
                                        };
                                        let _ = tx_progress.send((format!("正在删除 {}%", percent), false));
                                    }))
                                } else {
                                    sftp.delete_file(&entry_path).map(|_| 0u64)
                                };

                                match result {
                                    Ok(_) => {
                                        let _ = tx.send(("删除完成".to_string(), false));
                                    }
                                    Err(e) => {
                                        let _ = tx.send((format!("删除失败: {}", e), true));
                                    }
                                }
                            }
                            Err(e) => {
                                let _ = tx.send((format!("连接失败: {}", e), true));
                            }
                        }
                    } else {
                        let _ = tx.send(("未连接 SSH".to_string(), true));
                    }
                });
            }
        });

        // 拖拽上传支持
        let drop_target = DropTarget::new(gio::File::static_type(), gtk4::gdk::DragAction::COPY);

        let ssh_config_for_drop = ssh_config.clone();
        let progress_for_drop = progress_label.clone();
        let model_for_drop = model.clone();
        let entries_for_drop = entries.clone();
        let sftp_current = sftp.clone();
        let path_entry_for_drop = path_entry.clone();
        let transferring_for_drop = transferring_files.clone();

        drop_target.connect_drop(move |_target, value, _x, _y| {
            // 获取拖拽的文件
            let files: Vec<gio::File> = if let Ok(file) = value.get::<gio::File>() {
                vec![file]
            } else if let Ok(files) = value.get::<gio::ListStore>() {
                let mut result = Vec::new();
                for i in 0..files.n_items() {
                    if let Some(file) = files.item(i).and_downcast::<gio::File>() {
                        result.push(file);
                    }
                }
                result
            } else {
                return false;
            };

            if files.is_empty() {
                return false;
            }

            let config = ssh_config_for_drop.borrow().clone();

            // 获取当前目录路径（从路径输入框）
            let remote_dir = std::path::PathBuf::from(path_entry_for_drop.text().as_str());

            // 计算所有要上传的文件远程路径并添加到传输列表
            let remote_paths: Vec<String> = files.iter().filter_map(|f| {
                f.path().map(|p| {
                    let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                    remote_dir.join(&name).to_string_lossy().to_string()
                })
            }).collect();

            // 添加所有文件到传输列表并刷新显示
            let transferring = transferring_for_drop.clone();
            for path in &remote_paths {
                transferring.borrow_mut().insert(path.clone());
            }
            model_for_drop.items_changed(0, 0, 0);

            // 显示上传中
            progress_for_drop.set_text("正在上传 0%");
            progress_for_drop.remove_css_class("error");

            // 创建通道
            let (tx, rx) = channel::<(String, bool)>();
            let progress = progress_for_drop.clone();
            let model = model_for_drop.clone();
            let entries = entries_for_drop.clone();
            let sftp_update = sftp_current.clone();
            let remote_paths_clone = remote_paths.clone();
            let transferring = transferring.clone();

            // 主线程监听结果
            glib::idle_add_local(move || {
                match rx.try_recv() {
                    Ok((msg, is_error)) => {
                        progress.set_text(&msg);
                        if is_error {
                            progress.add_css_class("error");
                        } else {
                            progress.remove_css_class("error");
                            if msg.contains("完成") {
                                // 从传输列表移除所有文件
                                for path in &remote_paths_clone {
                                    transferring.borrow_mut().remove(path);
                                }
                                // 刷新文件列表
                                if let Some(ref sftp) = *sftp_update.borrow() {
                                    if let Ok(new_entries) = sftp.list_dir() {
                                        *entries.borrow_mut() = new_entries.clone();
                                        model.remove_all();
                                        for e in new_entries {
                                            model.append(&glib::BoxedAnyObject::new(e));
                                        }
                                    }
                                }
                                model.items_changed(0, 0, 0);
                                let p = progress.clone();
                                glib::timeout_add_local_once(std::time::Duration::from_secs(3), move || {
                                    p.set_text("");
                                    p.remove_css_class("error");
                                });
                            }
                        }
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => {}
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        return glib::ControlFlow::Break;
                    }
                }
                glib::ControlFlow::Continue
            });

            // 在后台线程执行上传
            std::thread::spawn(move || {
                if let Some(ref cfg) = config {
                    match SftpManager::connect(cfg) {
                        Ok(sftp) => {
                            let mut success_count = 0;
                            let mut fail_count = 0;
                            let mut total_files = files.len();
                            let mut processed_files = 0;

                            for file in files {
                                if let Some(local_path) = file.path() {
                                    let file_name = local_path.file_name()
                                        .map(|n| n.to_string_lossy().to_string())
                                        .unwrap_or_else(|| "unknown".to_string());

                                    let remote_path = remote_dir.join(&file_name);

                                    // 检查是文件还是目录
                                    if local_path.is_dir() {
                                        let tx_progress = tx.clone();
                                        let file_name_clone = file_name.clone();
                                        let progress_cb = move |transferred: u64, total: u64| {
                                            let percent = if total > 0 {
                                                (transferred as f64 / total as f64 * 100.0) as u32
                                            } else {
                                                0
                                            };
                                            let _ = tx_progress.send((format!("正在上传 {} {}%", file_name_clone, percent), false));
                                        };

                                        match sftp.upload_dir_with_progress(&local_path, &remote_path, Some(&progress_cb)) {
                                            Ok(_) => success_count += 1,
                                            Err(e) => {
                                                eprintln!("上传文件夹失败: {}", e);
                                                fail_count += 1;
                                            }
                                        }
                                    } else {
                                        let tx_progress = tx.clone();
                                        let file_name_clone = file_name.clone();
                                        let progress_cb = move |transferred: u64, total: u64| {
                                            let percent = if total > 0 {
                                                (transferred as f64 / total as f64 * 100.0) as u32
                                            } else {
                                                0
                                            };
                                            let _ = tx_progress.send((format!("正在上传 {} {}%", file_name_clone, percent), false));
                                        };

                                        match sftp.upload_with_progress(&local_path, &remote_path, Some(StdBox::new(progress_cb))) {
                                            Ok(()) => success_count += 1,
                                            Err(e) => {
                                                eprintln!("上传文件失败: {}", e);
                                                fail_count += 1;
                                            }
                                        }
                                    }
                                    processed_files += 1;
                                }
                            }

                            if fail_count == 0 {
                                let _ = tx.send((format!("上传完成 ({} 个文件/文件夹)", success_count), false));
                            } else {
                                let _ = tx.send((format!("上传失败 (成功: {}, 失败: {})", success_count, fail_count), true));
                            }
                        }
                        Err(e) => {
                            let _ = tx.send((format!("连接失败: {}", e), true));
                        }
                    }
                } else {
                    let _ = tx.send(("未连接 SSH".to_string(), true));
                }
            });

            true
        });

        scrolled.add_controller(drop_target);

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
            transferring_files,
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
    pub fn widget(&self) -> &GtkBox {
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
            transferring_files: self.transferring_files.clone(),
        }
    }
}

impl Default for FilePanel {
    fn default() -> Self {
        Self::new()
    }
}